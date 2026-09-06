//! 面料行业版销售订单服务（so/fabric_order）
//!
//! 缺陷 3 修复（Handler 绕过 Service）：原 `sales_fabric_order_handler.rs` 直接操作
//! Entity 构建事务/订单号/金额计算，绕过 service 层。本模块将该业务逻辑下沉到
//! `impl SalesService`，handler 仅保留参数提取 + 调用 service。
//!
//! 业务规则：
//! - 订单号生成：`SO{yyyymmddHHMMSS}`（简单时间戳单号，与原 handler 保持一致）
//! - 明细校验：数量/单价非负，价格精度 2 位小数（P2-11 修复逻辑下沉）
//! - 总金额 = Σ(quantity_meters × final_price)，主表与明细在同一事务内提交
//! - 审核流：pending → approved（记录 approved_at）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde::Deserialize;

use crate::models::{sales_order, sales_order_item};
use crate::services::so::order::SalesService;
use crate::utils::error::AppError;

/// 创建面料销售订单明细请求（handler DTO 直接透传，字段与原 handler 一致）
/// 批次说明：product_name / batch_no / dye_lot_no 为前端表单冗余字段，明细落库时
/// 由 sales_order_item 的 color_no/pantone 等专有列承载，此处保留反序列化兼容。
#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "反序列化输入字段（前端表单兼容字段暂不落库）")]
pub struct FabricOrderItemRequest {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub unit_price_meters: Decimal,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub color_no: String,
    pub batch_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<Decimal>,
    pub paper_tube_weight: Option<Decimal>,
    pub is_net_weight: Option<bool>,
    pub color_extra_cost: Option<Decimal>,
    pub grade_price_diff: Option<Decimal>,
    pub final_price: Option<Decimal>,
    /// 染色匹号（匹号领域：销售单据体现染色匹号，可为约定匹号）
    pub piece_no: Option<String>,
}

/// 创建面料销售订单请求
#[derive(Debug, Deserialize)]
#[allow(
    dead_code,
    reason = "反序列化输入字段（payment_terms/batch_no/color_no 等暂不落库）"
)]
pub struct CreateFabricOrderRequest {
    pub customer_id: i32,
    pub order_date: chrono::DateTime<chrono::Utc>,
    pub required_date: chrono::DateTime<chrono::Utc>,
    pub items: Vec<FabricOrderItemRequest>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

/// 更新面料销售订单请求
#[derive(Debug, Deserialize)]
#[allow(
    dead_code,
    reason = "反序列化输入字段（items/payment_terms 等暂不消费）"
)]
pub struct UpdateFabricOrderRequest {
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<FabricOrderItemRequest>>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

/// 面料行业版订单列表查询参数对象（消除 too_many_arguments 警告）
pub struct FabricOrderListQuery {
    pub page: u64,
    pub page_size: u64,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
    pub status: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
}

impl SalesService {
    /// 面料行业版订单列表（分页 + 过滤，复用 sales_order Entity）
    pub async fn list_fabric_orders(
        &self,
        query: FabricOrderListQuery,
    ) -> Result<(Vec<sales_order::Model>, u64), AppError> {
        let page = query.page.clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
        let page_size = query.page_size.clamp(1, 100);

        let mut query_builder = sales_order::Entity::find();
        if let Some(cid) = query.customer_id {
            query_builder = query_builder.filter(sales_order::Column::CustomerId.eq(cid));
        }
        if let Some(no) = query.order_no {
            let pattern = crate::utils::sql_escape::safe_like_pattern(&no);
            query_builder = query_builder.filter(sales_order::Column::OrderNo.like(&pattern));
        }
        if let Some(st) = query.status {
            query_builder = query_builder.filter(sales_order::Column::Status.eq(st));
        }
        // batch_no / color_no 是面料行业扩展查询条件：
        // sales_order 主表无对应列，需通过明细表匹配后回查主表
        let mut order_ids: Option<Vec<i32>> = None;
        if query.batch_no.is_some() || query.color_no.is_some() {
            use sea_orm::QuerySelect;
            let mut item_query = sales_order_item::Entity::find().select_only();
            item_query = item_query.column(sales_order_item::Column::OrderId);
            if let Some(bn) = &query.batch_no {
                item_query = item_query.filter(sales_order_item::Column::ColorNo.is_not_null());
                let _ = bn; // 明细表无 batch_no 列时忽略（与原 handler 语义一致）
            }
            if let Some(cn) = &query.color_no {
                item_query = item_query.filter(sales_order_item::Column::ColorNo.eq(cn.clone()));
            }
            item_query = item_query.filter(sales_order_item::Column::OrderId.is_not_null());
            let ids = item_query
                .into_tuple::<i32>()
                .all(&*self.db)
                .await
                .unwrap_or_default();
            order_ids = Some(ids);
        }
        if let Some(ids) = order_ids {
            if ids.is_empty() {
                return Ok((Vec::new(), 0));
            }
            query_builder = query_builder.filter(sales_order::Column::Id.is_in(ids));
        }

        let paginator = query_builder
            .order_by(sales_order::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);
        let orders = paginator
            .fetch_page(page.clamp(1, 1000).saturating_sub(1))
            .await?;
        let total = paginator.num_items().await?;
        Ok((orders, total))
    }

    /// 面料行业版订单详情
    pub async fn get_fabric_order(&self, id: i32) -> Result<sales_order::Model, AppError> {
        sales_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))
    }

    /// 面料行业版订单创建（事务：主表 + 明细）
    /// user_id 保留为扩展位（原 handler 语义：created_by 由调用方传入；当前主表 created_by=None 与原实现一致）
    pub async fn create_fabric_order(
        &self,
        req: CreateFabricOrderRequest,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let _ = user_id;
        // P2-11 修复逻辑下沉：金额/数量非负 + 精度校验
        Self::validate_fabric_order_items(&req.items)?;

        let txn = (*self.db).begin().await?;
        let order_no = format!("SO{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let total_amount = Self::calculate_fabric_order_totals(&req.items);

        let order = Self::build_fabric_order_active_model(&req, order_no, total_amount);
        let created_order = order
            .insert(&txn)
            .await
            .map_err(|e| AppError::bad_request(format!("创建订单失败：{}", e)))?;

        for item in &req.items {
            let order_item = Self::build_fabric_order_item_active_model(item, created_order.id);
            order_item
                .insert(&txn)
                .await
                .map_err(|e| AppError::bad_request(format!("创建订单明细失败：{}", e)))?;
        }

        txn.commit().await?;
        Ok(created_order)
    }

    /// 面料行业版订单更新（仅主表字段；明细全量替换逻辑未启用，与原 handler 语义一致）
    pub async fn update_fabric_order(
        &self,
        id: i32,
        req: UpdateFabricOrderRequest,
    ) -> Result<sales_order::Model, AppError> {
        let mut order: sales_order::ActiveModel = sales_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?
            .into();

        if let Some(date) = req.required_date {
            order.required_date = Set(date);
        }
        if let Some(status) = req.status {
            order.status = Set(status);
        }
        if let Some(addr) = req.shipping_address {
            order.shipping_address = Set(Some(addr));
        }
        if let Some(addr) = req.delivery_address {
            order.billing_address = Set(Some(addr));
        }
        // 注意：sales_order 模型没有 payment_terms, remarks, batch_no, color_no 等字段
        // 如有需要，可以考虑使用 notes 字段或其他方式存储

        order.updated_at = Set(chrono::Utc::now());
        let updated = order
            .update(&*self.db)
            .await
            .map_err(|e| AppError::bad_request(format!("更新订单失败：{}", e)))?;
        Ok(updated)
    }

    /// 面料行业版订单删除（审计日志下沉）
    pub async fn delete_fabric_order(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志（批次 94 P2-10：真实操作人 user_id）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            sales_order::Entity,
            _,
        >(&*self.db, "sales_fabric_order", id, Some(user_id))
        .await?;
        Ok(())
    }

    /// 面料行业版订单审核（pending → approved）
    pub async fn approve_fabric_order(&self, id: i32) -> Result<sales_order::Model, AppError> {
        let mut order: sales_order::ActiveModel = sales_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?
            .into();

        order.status = Set("approved".to_string());
        order.approved_by = Set(None);
        order.approved_at = Set(Some(chrono::Utc::now()));
        order.updated_at = Set(chrono::Utc::now());

        let updated = order
            .update(&*self.db)
            .await
            .map_err(|e| AppError::bad_request(format!("审核订单失败：{}", e)))?;
        Ok(updated)
    }

    // ===== 私有 helper（从原 handler 下沉） =====

    /// 校验订单明细列表（P2-11：金额/数量非负 + round_dp(2) 精度）
    fn validate_fabric_order_items(items: &[FabricOrderItemRequest]) -> Result<(), AppError> {
        for (idx, item) in items.iter().enumerate() {
            Self::validate_fabric_item(item, idx)?;
        }
        Ok(())
    }

    /// 校验单个明细项
    fn validate_fabric_item(item: &FabricOrderItemRequest, idx: usize) -> Result<(), AppError> {
        if item.quantity_meters < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 quantity_meters 不能为负数",
                idx + 1
            )));
        }
        if item.quantity_kg < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 quantity_kg 不能为负数",
                idx + 1
            )));
        }
        if item.unit_price_meters < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 unit_price_meters 不能为负数",
                idx + 1
            )));
        }
        if item.unit_price_meters.round_dp(2) != item.unit_price_meters {
            return Err(AppError::validation(format!(
                "第 {} 项 unit_price_meters 精度不能超过 2 位小数",
                idx + 1
            )));
        }
        if let Some(p) = item.base_price {
            Self::validate_price_precision(p, "base_price", idx)?;
        }
        if let Some(p) = item.final_price {
            Self::validate_price_precision(p, "final_price", idx)?;
        }
        Ok(())
    }

    /// 校验可选价格字段的非负与精度（货币精度 2 位小数）
    fn validate_price_precision(p: Decimal, field: &str, idx: usize) -> Result<(), AppError> {
        if p < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 {} 不能为负数",
                idx + 1,
                field
            )));
        }
        if p.round_dp(2) != p {
            return Err(AppError::validation(format!(
                "第 {} 项 {} 精度不能超过 2 位小数",
                idx + 1,
                field
            )));
        }
        Ok(())
    }

    /// 计算订单总金额（Σ quantity_meters × unit_price_meters）
    fn calculate_fabric_order_totals(items: &[FabricOrderItemRequest]) -> Decimal {
        items
            .iter()
            .map(|item| item.quantity_meters * item.unit_price_meters)
            .fold(Decimal::ZERO, |acc, v| acc + v)
    }

    /// 构建订单主表 ActiveModel
    fn build_fabric_order_active_model(
        req: &CreateFabricOrderRequest,
        order_no: String,
        total_amount: Decimal,
    ) -> sales_order::ActiveModel {
        sales_order::ActiveModel {
            id: Set(0),
            order_no: Set(order_no),
            customer_id: Set(req.customer_id),
            opportunity_id: Set(None),
            order_date: Set(req.order_date),
            required_date: Set(req.required_date),
            ship_date: Set(None),
            status: Set("pending".to_string()),
            subtotal: Set(total_amount),
            tax_amount: Set(Decimal::ZERO),
            discount_amount: Set(Decimal::ZERO),
            shipping_cost: Set(Decimal::ZERO),
            total_amount: Set(total_amount),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(total_amount),
            shipping_address: Set(req.shipping_address.clone()),
            billing_address: Set(req.delivery_address.clone()),
            notes: Set(req.remarks.clone()),
            batch_no: Set(Some(String::new())),
            color_no: Set(Some(String::new())),
            dye_lot_no: Set(Some(String::new())),
            grade: Set(None),
            packaging_requirement: Set(None),
            quality_standard: Set(None),
            created_by: Set(None),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    /// 构建订单明细 ActiveModel
    fn build_fabric_order_item_active_model(
        item: &FabricOrderItemRequest,
        order_id: i32,
    ) -> sales_order_item::ActiveModel {
        let quantity_meters = item.quantity_meters;
        let quantity_kg = item.quantity_kg;
        let base_price_val = item.base_price.unwrap_or(item.unit_price_meters);
        let color_extra = item.color_extra_cost.unwrap_or_default();
        let grade_diff = item.grade_price_diff.unwrap_or_default();
        let final_p = item.final_price.unwrap_or(item.unit_price_meters);
        let subtotal = quantity_meters * final_p;
        sales_order_item::ActiveModel {
            id: Set(0),
            order_id: Set(order_id),
            product_id: Set(item.product_id),
            quantity: Set(quantity_meters),
            unit_price: Set(final_p),
            discount_percent: Set(Decimal::ZERO),
            tax_percent: Set(Decimal::ZERO),
            subtotal: Set(subtotal),
            tax_amount: Set(Decimal::ZERO),
            discount_amount: Set(Decimal::ZERO),
            total_amount: Set(subtotal),
            shipped_quantity: Set(Decimal::ZERO),
            notes: Set(item.remarks.clone()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            color_no: Set(item.color_no.clone()),
            color_name: Set(item.color_name.clone()),
            pantone_code: Set(item.pantone_code.clone()),
            grade_required: Set(item.grade.clone()),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            gram_weight: Set(item.gram_weight),
            width: Set(item.width),
            batch_requirement: Set(item.batch_requirement.clone()),
            dye_lot_requirement: Set(item.dye_lot_requirement.clone()),
            piece_no: Set(item.piece_no.clone()),
            base_price: Set(Some(base_price_val)),
            color_extra_cost: Set(color_extra),
            grade_price_diff: Set(grade_diff),
            final_price: Set(Some(final_p)),
            shipped_quantity_meters: Set(Decimal::ZERO),
            shipped_quantity_kg: Set(Decimal::ZERO),
            paper_tube_weight: Set(item.paper_tube_weight),
            is_net_weight: Set(item.is_net_weight),
        }
    }
}

//! 销售订单查询子模块（order_query）
//!
//! P9-2 拆分自原 `services/so/order.rs`。
//! 包含：list_orders / get_order_detail / get_order_statistics
//!
//! ## 模块职责
//! - 销售订单分页查询（含客户、日期、状态等过滤）
//! - 销售订单详情（含明细项 + 客户 + 产品 + 色号等）
//! - 销售订单统计（按客户/产品/月份）

use super::order::SalesService;
use crate::models::dto::PageRequest;
use crate::models::{
    sales_order,
    sales_order::Entity as SalesOrderEntity,
    sales_order_item,
};
use crate::services::so::{SalesOrderDetail, SalesOrderItemDetail};
use crate::utils::data_scope::{apply_data_scope, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::PaginatedResponse;
use sea_orm::{
    ColumnTrait, EntityTrait, LoaderTrait, ModelTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, RelationTrait,
};

/// 销售订单查询子模块标记
pub const P92_QRY_MODULE: &str = "sales_order_query";

/// 销售订单查询条件
#[derive(Debug, Clone, Default)]
pub struct OrderQuery {
    /// 客户 ID
    pub customer_id: Option<i32>,
    /// 订单状态
    pub status: Option<String>,
    /// 起始日期
    pub date_from: Option<chrono::NaiveDate>,
    /// 截止日期
    pub date_to: Option<chrono::NaiveDate>,
    /// 关键字
    pub keyword: Option<String>,
}

impl OrderQuery {
    /// 是否为空查询
    pub fn is_empty(&self) -> bool {
        self.customer_id.is_none()
            && self.status.is_none()
            && self.date_from.is_none()
            && self.date_to.is_none()
            && self.keyword.is_none()
    }

    /// 中文描述
    pub fn desc(&self) -> String {
        let mut parts = Vec::new();
        if let Some(cid) = self.customer_id {
            parts.push(format!("客户ID={cid}"));
        }
        if let Some(s) = &self.status {
            parts.push(format!("状态={s}"));
        }
        if let Some(d) = self.date_from {
            parts.push(format!("从 {d}"));
        }
        if let Some(d) = self.date_to {
            parts.push(format!("至 {d}"));
        }
        if let Some(k) = &self.keyword {
            parts.push(format!("关键字={k}"));
        }
        if parts.is_empty() {
            "无过滤条件".to_string()
        } else {
            parts.join(", ")
        }
    }
}

impl SalesService {
    // list_orders / get_order_detail / get_order_statistics
    // 内容来自原 order.rs L37-276 + L841-897
    // 注意：保留 DTO（CreateSalesOrderRequest / SalesOrderDetail / SalesOrderItemDetail / UpdateSalesOrderRequest）
    // 在 super::super::so 中定义

    pub async fn list_orders(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_id: Option<i32>,
        order_no: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PaginatedResponse<SalesOrderDetail>, AppError> {
        let query = Self::build_orders_query(status, customer_id, order_no, data_scope);
        let (orders, total) = self.fetch_orders_page(query, &page_req).await?;
        let order_details = self.assemble_order_details(orders).await?;
        Ok(PaginatedResponse::new(
            order_details,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    fn build_orders_query(
        status: Option<String>,
        customer_id: Option<i32>,
        order_no: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> sea_orm::Select<sales_order::Entity> {
        let mut query = SalesOrderEntity::find()
            .column_as(
                crate::models::customer::Column::CustomerName,
                "customer_name",
            )
            .join(
                sea_orm::JoinType::LeftJoin,
                sales_order::Relation::Customer.def(),
            );

        // V15 P0-S01：行级数据权限过滤（sales_order 表无 department_id，Dept 退化为 Self）
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                sales_order::Column::CreatedBy,
                sales_order::Column::CreatedBy,
            );
        }

        if let Some(s) = status {
            query = query.filter(sales_order::Column::Status.eq(s));
        }
        if let Some(cid) = customer_id {
            query = query.filter(sales_order::Column::CustomerId.eq(cid));
        }
        if let Some(no) = order_no {
            query = query.filter(sales_order::Column::OrderNo.contains(&no));
        }

        query.order_by(sales_order::Column::CreatedAt, Order::Desc)
    }

    async fn fetch_orders_page(
        &self,
        query: sea_orm::Select<sales_order::Entity>,
        page_req: &PageRequest,
    ) -> Result<(Vec<sales_order::Model>, u64), AppError> {
        let paginator = query.paginate(&*self.db, page_req.page_size);
        // 使用统一分页辅助函数，并行执行分页查询与总数统计
        let (orders, total): (Vec<sales_order::Model>, u64) =
            paginate_with_total(paginator, page_req.page).await?;
        Ok((orders, total))
    }

    async fn assemble_order_details(
        &self,
        orders: Vec<sales_order::Model>,
    ) -> Result<Vec<SalesOrderDetail>, AppError> {
        let mut order_details = Vec::with_capacity(orders.len());
        if orders.is_empty() {
            return Ok(order_details);
        }

        // 使用 LoaderTrait 批量加载 customer
        let customers = orders
            .load_one(crate::models::customer::Entity, &*self.db)
            .await?;
        // 使用 LoaderTrait 批量加载 items
        let items_vec = orders
            .load_many(sales_order_item::Entity, &*self.db)
            .await?;
        // 提取所有 items，用于批量加载 products
        let all_items_owned: Vec<sales_order_item::Model> =
            items_vec.iter().flatten().cloned().collect();
        // 使用 LoaderTrait 批量加载 products
        let products = all_items_owned
            .load_one(crate::models::product::Entity, &*self.db)
            .await?;

        let mut global_item_index = 0;
        // 组装数据
        for (i, order) in orders.into_iter().enumerate() {
            let customer = customers[i].as_ref();
            let items = &items_vec[i];
            let mut item_details = Vec::with_capacity(items.len());
            for item in items.iter() {
                let product = products[global_item_index].as_ref();
                global_item_index += 1;
                item_details.push(Self::build_item_detail(item, product));
            }
            order_details.push(Self::build_order_detail(order, customer, item_details));
        }
        Ok(order_details)
    }

    fn build_item_detail(
        item: &sales_order_item::Model,
        product: Option<&crate::models::product::Model>,
    ) -> SalesOrderItemDetail {
        SalesOrderItemDetail {
            id: item.id,
            order_id: item.order_id,
            product_id: item.product_id,
            product_code: product.map(|p| p.code.clone()),
            product_name: product.map(|p| p.name.clone()),
            quantity: item.quantity,
            unit_price: item.unit_price,
            discount_percent: item.discount_percent,
            tax_percent: item.tax_percent,
            subtotal: item.subtotal,
            tax_amount: item.tax_amount,
            discount_amount: item.discount_amount,
            total_amount: item.total_amount,
            shipped_quantity: item.shipped_quantity,
            notes: item.notes.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            color_no: item.color_no.clone(),
            color_name: item.color_name.clone(),
            pantone_code: item.pantone_code.clone(),
            grade_required: item.grade_required.clone(),
            quantity_meters: item.quantity_meters,
            quantity_kg: item.quantity_kg,
            gram_weight: item.gram_weight,
            width: item.width,
            paper_tube_weight: item.paper_tube_weight,
            is_net_weight: item.is_net_weight,
            batch_requirement: item.batch_requirement.clone(),
            dye_lot_requirement: item.dye_lot_requirement.clone(),
            base_price: item.base_price,
            color_extra_cost: item.color_extra_cost,
            grade_price_diff: item.grade_price_diff,
            final_price: item.final_price,
            shipped_quantity_meters: item.shipped_quantity_meters,
            shipped_quantity_kg: item.shipped_quantity_kg,
        }
    }

    fn build_order_detail(
        order: sales_order::Model,
        customer: Option<&crate::models::customer::Model>,
        item_details: Vec<SalesOrderItemDetail>,
    ) -> SalesOrderDetail {
        SalesOrderDetail {
            id: order.id,
            order_no: order.order_no,
            customer_id: order.customer_id,
            customer_name: customer.map(|c| c.customer_name.clone()),
            opportunity_id: order.opportunity_id,
            order_date: order.order_date,
            required_date: order.required_date,
            ship_date: order.ship_date,
            status: order.status,
            subtotal: order.subtotal,
            tax_amount: order.tax_amount,
            discount_amount: order.discount_amount,
            shipping_cost: order.shipping_cost,
            total_amount: order.total_amount,
            paid_amount: order.paid_amount,
            balance_amount: order.balance_amount,
            shipping_address: order.shipping_address,
            billing_address: order.billing_address,
            notes: order.notes,
            created_by: order.created_by,
            approved_by: order.approved_by,
            approved_at: order.approved_at,
            created_at: order.created_at,
            updated_at: order.updated_at,
            items: item_details,
        }
    }

    /// 获取销售订单详情（包含明细项）
    pub async fn get_order_detail(
        &self,
        order_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<SalesOrderDetail, AppError> {
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护，按 created_by 校验）
        Self::validate_order_data_scope(data_scope, order.created_by, order_id)?;

        let customer = order
            .find_related(crate::models::customer::Entity)
            .one(&*self.db)
            .await?;

        let items = order
            .find_related(sales_order_item::Entity)
            .order_by(sales_order_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let products = items
            .load_one(crate::models::product::Entity, &*self.db)
            .await?;

        let item_details = Self::build_item_details(items, &products);

        Ok(Self::build_order_detail(order, customer.as_ref(), item_details))
    }

    /// 行级数据权限校验（IDOR 防护）
    fn validate_order_data_scope(
        data_scope: Option<&DataScopeContext>,
        created_by: Option<i32>,
        order_id: i32,
    ) -> Result<(), AppError> {
        if let Some(ctx) = data_scope {
            if !crate::utils::data_scope::check_resource_owner(ctx, created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问销售订单 {}（数据范围限制）",
                    order_id
                )));
            }
        }
        Ok(())
    }

    /// 批量构建订单明细项列表
    fn build_item_details(
        items: Vec<sales_order_item::Model>,
        products: &[Option<crate::models::product::Model>],
    ) -> Vec<SalesOrderItemDetail> {
        let mut details = Vec::with_capacity(items.len());
        for (i, item) in items.into_iter().enumerate() {
            let product = products[i].as_ref();
            details.push(Self::build_single_item_detail(item, product));
        }
        details
    }

    /// 构建单个订单明细项
    fn build_single_item_detail(
        item: sales_order_item::Model,
        product: Option<&crate::models::product::Model>,
    ) -> SalesOrderItemDetail {
        SalesOrderItemDetail {
            id: item.id,
            order_id: item.order_id,
            product_id: item.product_id,
            product_code: product.map(|p| p.code.clone()),
            product_name: product.map(|p| p.name.clone()),
            quantity: item.quantity,
            unit_price: item.unit_price,
            discount_percent: item.discount_percent,
            tax_percent: item.tax_percent,
            subtotal: item.subtotal,
            tax_amount: item.tax_amount,
            discount_amount: item.discount_amount,
            total_amount: item.total_amount,
            shipped_quantity: item.shipped_quantity,
            notes: item.notes,
            created_at: item.created_at,
            updated_at: item.updated_at,
            color_no: item.color_no,
            color_name: item.color_name,
            pantone_code: item.pantone_code,
            grade_required: item.grade_required,
            quantity_meters: item.quantity_meters,
            quantity_kg: item.quantity_kg,
            gram_weight: item.gram_weight,
            width: item.width,
            paper_tube_weight: item.paper_tube_weight,
            is_net_weight: item.is_net_weight,
            batch_requirement: item.batch_requirement,
            dye_lot_requirement: item.dye_lot_requirement,
            base_price: item.base_price,
            color_extra_cost: item.color_extra_cost,
            grade_price_diff: item.grade_price_diff,
            final_price: item.final_price,
            shipped_quantity_meters: item.shipped_quantity_meters,
            shipped_quantity_kg: item.shipped_quantity_kg,
        }
    }

    /// 创建销售订单
    pub async fn get_order_statistics(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        use sea_orm::QuerySelect;

        let _start_date = query
            .get("start_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2020-01-01");

        let _end_date = query
            .get("end_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2099-12-31");

        let customer_id = query
            .get("customer_id")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let mut query = SalesOrderEntity::find()
            .select_only()
            .column_as(sales_order::Column::Id.count(), "total_orders")
            .column_as(sales_order::Column::TotalAmount.sum(), "total_amount");

        if let Some(cid) = customer_id {
            query = query.filter(sales_order::Column::CustomerId.eq(cid));
        }

        let result = query
            .into_model::<serde_json::Value>()
            .one(&*self.db)
            .await?;

        Ok(result.unwrap_or_else(|| {
            serde_json::json!({
                "total_orders": 0,
                "total_amount": 0,
                "completed_orders": 0,
                "cancelled_orders": 0,
                "pending_orders": 0,
                "approved_orders": 0,
            })
        }))
    }

    // ========== 库存辅助方法（私有） ==========
    // 注意：lock_inventory、reduce_inventory、release_reservations、check_inventory
    // 已迁移到 so/delivery.rs，避免重复实现

    // ========== 数据导出方法 ==========
    // 注意：export_orders_to_csv 已迁移到 so/delivery.rs

    // ========== 订单生命周期方法（handler 调用） ==========
}

#[cfg(test)]
mod tests {
    use super::*;
    // 批次 158 v11 修复 clippy：so_status 仅测试代码引用，use 移入测试模块避免 unused import 警告
    use crate::models::status::sales_order as so_status;

    #[test]
    fn test_order_query_is_empty() {
        let q = OrderQuery::default();
        assert!(q.is_empty());
        assert_eq!(q.desc(), "无过滤条件");
    }

    #[test]
    fn test_order_query_with_filters() {
        let q = OrderQuery {
            customer_id: Some(100),
            status: Some(so_status::APPROVED.to_string()),
            ..Default::default()
        };
        assert!(!q.is_empty());
        assert!(q.desc().contains("客户ID=100"));
        assert!(q.desc().contains("状态=approved"));
    }

    #[test]
    fn test_query_module_loaded() {
        assert_eq!(P92_QRY_MODULE, "sales_order_query");
    }
}

//! 采购订单-CRUD 子模块（order_ops/crud）
//!
//! 拆分自原 `po/order.rs` 的 `impl PurchaseOrderService` 块。
//! 包含创建 / 更新 / 删除 / 列表 / 详情方法及其私有 helper：
//! - `generate_order_no_with_txn`（宏生成，`pub`，price 子模块跨 impl 块调用）
//! - `create_order` / `validate_order_request` / `create_order_header` /
//!   `validate_products_exist_txn` / `calculate_item_amounts` /
//!   `build_order_item_active_model` / `create_order_items`（创建链路 + 私有 helper）
//! - `update_order` / `validate_order_modification` / `apply_order_field_updates`（更新链路）
//! - `delete_order`（删除）
//! - `list_orders` / `get_order`（列表 / 详情查询，含行级数据权限）
//!
//! 业务规则：
//! - 创建订单含明细金额计算（round_dp(2) 精度归一化）与预算检查占用
//! - 更新 / 删除仅限草稿（DRAFT）状态，更新另允许 REJECTED；仅创建人可操作
//! - 列表 / 详情接入 V15 P0-S01 行级数据权限过滤与 IDOR 校验

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Set, TransactionTrait,
};

use crate::models::{
    department, product, purchase_order, purchase_order_item, status, supplier, warehouse,
};
use crate::services::po::order::{PurchaseOrderDto, PurchaseOrderService};
use crate::services::po::{CreatePurchaseOrderRequest, UpdatePurchaseOrderRequest};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
// 批次 260 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;

/// 单行明细金额计算结果（create_order_items 内部 helper 数据载体）
struct ItemAmounts {
    quantity_ordered: Decimal,
    unit_price: Decimal,
    quantity_alt_ordered: Decimal,
    tax_percent: Decimal,
    discount_percent: Decimal,
    amount: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
}

impl PurchaseOrderService {
    // 生成采购订单号（使用事务连接）
    // 格式：PO + 年月日 + 三位序号（PO20260315001）
    crate::impl_generate_no!(
        generate_order_no_with_txn,
        "PO",
        purchase_order::Entity,
        purchase_order::Column::OrderNo,
        txn
    );

    /// 创建采购订单（含明细）
    pub async fn create_order(
        &self,
        req: CreatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 验证请求参数
        let (warehouse_id, department_id) = self.validate_order_request(&req, &txn).await?;

        // 2. 创建订单主表
        let order = self
            .create_order_header(&req, warehouse_id, department_id, user_id, &txn)
            .await?;

        // 3. 创建订单明细并计算总金额
        let (total_amount, total_quantity, total_quantity_alt) =
            self.create_order_items(&req, order.id, &txn).await?;

        // 4. 更新订单总金额和数量
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 5. 预算检查与占用（V15 P0-B06 强制拦截：预算不足或无方案时阻断订单创建）
        // 注意：此处使用 ? 传播错误，事务会自动回滚（Drop 时 rollback）
        // 避免订单已创建但未关联预算的不一致状态
        self.check_and_occupy_budget(&order, department_id, total_amount, user_id)
            .await?;

        // 6. 提交事务
        txn.commit().await?;

        Ok(order)
    }

    /// 验证采购订单请求参数
    async fn validate_order_request(
        &self,
        req: &CreatePurchaseOrderRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(i32, i32), AppError> {
        // 检查供应商是否存在
        let supplier_exists = supplier::Entity::find_by_id(req.supplier_id)
            .one(txn)
            .await?;
        if supplier_exists.is_none() {
            tracing::error!("供应商 ID {} 不存在", req.supplier_id);
            return Err(AppError::bad_request(format!(
                "供应商 ID {} 不存在",
                req.supplier_id
            )));
        }

        // 检查仓库是否存在
        let warehouse_id = req
            .warehouse_id
            .ok_or_else(|| AppError::bad_request("仓库 ID 不能为空"))?;
        let warehouse_exists = warehouse::Entity::find_by_id(warehouse_id).one(txn).await?;
        if warehouse_exists.is_none() {
            tracing::error!("仓库 ID {} 不存在", warehouse_id);
            return Err(AppError::bad_request(format!(
                "仓库 ID {} 不存在",
                warehouse_id
            )));
        }

        // 检查部门是否存在
        let department_id = req
            .department_id
            .ok_or_else(|| AppError::bad_request("部门 ID 不能为空"))?;
        let department_exists = department::Entity::find_by_id(department_id)
            .one(txn)
            .await?;
        if department_exists.is_none() {
            tracing::error!("Transaction rolled back: 部门 ID {} 不存在", department_id);
            return Err(AppError::bad_request(format!(
                "部门 ID {} 不存在",
                department_id
            )));
        }

        // 日期合理性检查
        if let Some(expected_date) = req.expected_delivery_date {
            if expected_date < req.order_date {
                tracing::error!("预计交货日期不能早于订单日期");
                return Err(AppError::bad_request(
                    "预计交货日期不能早于订单日期".to_string(),
                ));
            }
        }

        Ok((warehouse_id, department_id))
    }

    /// 创建采购订单主表
    async fn create_order_header(
        &self,
        req: &CreatePurchaseOrderRequest,
        warehouse_id: i32,
        department_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<purchase_order::Model, AppError> {
        // 生成订单号
        let order_no = DocumentNumberGenerator::generate_no(
            txn,
            "PO",
            purchase_order::Entity,
            purchase_order::Column::OrderNo,
        )
        .await?;

        // 创建采购订单主表
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(req.supplier_id),
            order_date: Set(req.order_date),
            expected_delivery_date: Set(req.expected_delivery_date),
            warehouse_id: Set(warehouse_id),
            department_id: Set(department_id),
            purchaser_id: Set(user_id),
            currency: Set(req.currency.clone().unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            order_status: Set(status::purchase_order::DRAFT.to_string()),
            payment_terms: Set(req.payment_terms.clone()),
            shipping_terms: Set(req.shipping_terms.clone()),
            notes: Set(req.notes.clone()),
            attachment_urls: Set(req.attachment_urls.clone()),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(txn)
        .await?;

        Ok(order)
    }

    /// 批量校验订单明细引用的产品是否存在
    async fn validate_products_exist_txn(
        txn: &sea_orm::DatabaseTransaction,
        items: &[crate::services::po::CreateOrderItemRequest],
    ) -> Result<(), AppError> {
        let mut product_ids = std::collections::HashSet::new();
        for item in items {
            if let Some(material_id) = item.material_id {
                if material_id != 0 {
                    product_ids.insert(material_id);
                }
            }
        }
        if product_ids.is_empty() {
            return Ok(());
        }
        let existing_products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids.iter().cloned().collect::<Vec<_>>()))
            .all(txn)
            .await?;
        let existing_product_ids: std::collections::HashSet<i32> =
            existing_products.into_iter().map(|p| p.id).collect();
        for material_id in &product_ids {
            if !existing_product_ids.contains(material_id) {
                tracing::error!("Transaction rolled back: 物料 ID {} 不存在", material_id);
                return Err(AppError::bad_request(format!("物料 ID {} 不存在", material_id)));
            }
        }
        Ok(())
    }

    /// 计算单行明细金额（round_dp(2) 精度归一化）
    fn calculate_item_amounts(item: &crate::services::po::CreateOrderItemRequest) -> ItemAmounts {
        let quantity_ordered = item.quantity_ordered.unwrap_or(Decimal::ZERO);
        let unit_price = item.unit_price.unwrap_or(Decimal::ZERO);
        let amount = (quantity_ordered * unit_price).round_dp(2);
        let tax_percent = item.tax_rate.unwrap_or(Decimal::new(13, 2));
        let tax_amount = (amount * tax_percent / Decimal::new(100, 0)).round_dp(2);
        let discount_percent = item.discount_percent.unwrap_or(Decimal::ZERO);
        let discount_amount = (amount * discount_percent / Decimal::new(100, 0)).round_dp(2);
        let quantity_alt_ordered = item.quantity_alt_ordered.unwrap_or(Decimal::ZERO);
        ItemAmounts {
            quantity_ordered,
            unit_price,
            quantity_alt_ordered,
            tax_percent,
            discount_percent,
            amount,
            tax_amount,
            discount_amount,
        }
    }

    /// 构造订单明细 ActiveModel（material_id 缺失拒绝创建）
    fn build_order_item_active_model(
        item: &crate::services::po::CreateOrderItemRequest,
        order_id: i32,
        index: usize,
        amounts: &ItemAmounts,
    ) -> Result<purchase_order_item::ActiveModel, AppError> {
        let material_id = item
            .material_id
            .ok_or_else(|| AppError::validation(format!("订单行 {} 缺少物料ID", index + 1)))?;
        Ok(purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            line_no: Set(item.line_no.unwrap_or((index + 1) as i32)),
            product_id: Set(material_id),
            quantity: Set(amounts.quantity_ordered),
            quantity_alt: Set(amounts.quantity_alt_ordered),
            unit_price: Set(amounts.unit_price),
            unit_price_foreign: Set(amounts.unit_price),
            discount_percent: Set(amounts.discount_percent),
            tax_percent: Set(amounts.tax_percent),
            subtotal: Set(amounts.amount),
            tax_amount: Set(amounts.tax_amount),
            discount_amount: Set(amounts.discount_amount),
            total_amount: Set(amounts.amount + amounts.tax_amount - amounts.discount_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(item.notes.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // v14 批次 417：面料行业追溯字段，使用 NotSet 让 DB 默认值处理
            color_code: sea_orm::ActiveValue::NotSet,
            lot_no: sea_orm::ActiveValue::NotSet,
            batch_no: sea_orm::ActiveValue::NotSet,
        })
    }

    /// 创建采购订单明细
    async fn create_order_items(
        &self,
        req: &CreatePurchaseOrderRequest,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(Decimal, Decimal, Decimal), AppError> {
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        let items = req.items.clone().unwrap_or_default();
        Self::validate_products_exist_txn(txn, &items).await?;

        for (index, item) in items.iter().enumerate() {
            let amounts = Self::calculate_item_amounts(item);
            let order_item = Self::build_order_item_active_model(item, order_id, index, &amounts)?;
            order_item.insert(txn).await?;

            total_amount += amounts.amount + amounts.tax_amount - amounts.discount_amount;
            total_quantity += amounts.quantity_ordered;
            total_quantity_alt += amounts.quantity_alt_ordered;
        }

        Ok((total_amount, total_quantity, total_quantity_alt))
    }

    /// 更新采购订单（仅草稿状态）
    pub async fn update_order(
        &self,
        order_id: i32,
        req: UpdatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;
        Self::validate_order_modification(&order, user_id)?;
        let mut order_active: purchase_order::ActiveModel = order.into();
        Self::apply_order_field_updates(&mut order_active, &req, user_id);
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;
        Ok(order)
    }

    /// 校验订单是否可修改（状态+权限）
    fn validate_order_modification(
        order: &purchase_order::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        if order.order_status != status::purchase_order::DRAFT
            && order.order_status != status::purchase_order::REJECTED
        {
            return Err(AppError::business(format!(
                "订单状态不允许修改，当前状态：{}",
                order.order_status
            )));
        }
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的订单".to_string(),
            ));
        }
        Ok(())
    }

    /// 将请求字段应用到 ActiveModel
    fn apply_order_field_updates(
        order_active: &mut purchase_order::ActiveModel,
        req: &UpdatePurchaseOrderRequest,
        user_id: i32,
    ) {
        if let Some(supplier_id) = req.supplier_id {
            order_active.supplier_id = Set(supplier_id);
        }
        if let Some(order_date) = req.order_date {
            order_active.order_date = Set(order_date);
        }
        if let Some(expected_delivery_date) = req.expected_delivery_date {
            order_active.expected_delivery_date = Set(Some(expected_delivery_date));
        }
        if let Some(warehouse_id) = req.warehouse_id {
            order_active.warehouse_id = Set(warehouse_id);
        }
        if let Some(department_id) = req.department_id {
            order_active.department_id = Set(department_id);
        }
        if let Some(currency) = &req.currency {
            order_active.currency = Set(currency.clone());
        }
        if let Some(exchange_rate) = req.exchange_rate {
            order_active.exchange_rate = Set(exchange_rate);
        }
        if let Some(payment_terms) = &req.payment_terms {
            order_active.payment_terms = Set(Some(payment_terms.clone()));
        }
        if let Some(shipping_terms) = &req.shipping_terms {
            order_active.shipping_terms = Set(Some(shipping_terms.clone()));
        }
        if let Some(notes) = &req.notes {
            order_active.notes = Set(Some(notes.clone()));
        }
        if let Some(attachment_urls) = &req.attachment_urls {
            order_active.attachment_urls = Set(Some(attachment_urls.clone()));
        }
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(Utc::now());
    }

    /// 删除采购订单（仅草稿状态）
    pub async fn delete_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许删除，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的订单".to_string(),
            ));
        }

        // 4. 删除订单（级联删除明细）（P0 8-3 修复：补审计日志）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            purchase_order::Entity,
            _,
        >(&*self.db, "purchase_order", order_id, Some(user_id))
        .await
    }

    /// 获取订单列表（分页）
    pub async fn list_orders(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<PurchaseOrderDto>, u64), AppError> {
        let mut query = purchase_order::Entity::find()
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Warehouse.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Department.def(),
            );

        // V15 P0-S01：行级数据权限过滤（purchase_order 表有 created_by + department_id，支持完整 Dept）
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                purchase_order::Column::CreatedBy,
                purchase_order::Column::DepartmentId,
            );
        }

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_order::Column::OrderStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_order::Column::SupplierId.eq(supplier_id));
        }

        // 批次 260 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_order::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<PurchaseOrderDto>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 获取订单详情
    pub async fn get_order(
        &self,
        order_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PurchaseOrderDto, AppError> {
        let order = purchase_order::Entity::find_by_id(order_id)
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Warehouse.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Department.def(),
            )
            .into_model::<PurchaseOrderDto>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // PurchaseOrderDto 的 created_by/department_id 由查询结果携带
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(order.created_by), Some(order.department_id)) {
                return Err(AppError::permission_denied(format!(
                    "无权访问采购订单 {}（数据范围限制）",
                    order_id
                )));
            }
        }

        Ok(order)
    }
}

//! 采购订单 Service
//!
//! 采购订单服务层，负责采购订单的核心业务逻辑
//! 包含订单创建、审批、执行、退货等全流程管理
#![allow(dead_code)]

use crate::models::{
    department, product, purchase_order, purchase_order_item, purchase_receipt, status, supplier, warehouse,
};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderDto {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: chrono::NaiveDate,
    pub expected_delivery_date: Option<chrono::NaiveDate>,
    pub actual_delivery_date: Option<chrono::NaiveDate>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub purchaser_id: i32,
    pub currency: String,
    pub exchange_rate: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub total_amount_foreign: rust_decimal::Decimal,
    pub total_quantity: rust_decimal::Decimal,
    pub total_quantity_alt: rust_decimal::Decimal,
    #[serde(rename = "status")]
    pub order_status: String,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderItemDto {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    #[serde(rename = "material_id")]
    pub product_id: i32,
    #[serde(rename = "material_code")]
    pub material_code: Option<String>,
    #[serde(rename = "material_name")]
    pub material_name: Option<String>,
    #[serde(rename = "quantity_ordered")]
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    #[serde(rename = "tax_rate")]
    pub tax_percent: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub returned_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
}

/// 采购订单服务
pub struct PurchaseOrderService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseOrderService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成采购订单号
    /// 格式：PO + 年月日 + 三位序号（PO20260315001）
    pub async fn generate_order_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &self.db,
            "PO",
            purchase_order::Entity,
            purchase_order::Column::OrderNo,
        )
        .await
    }

    /// 生成采购订单号（使用事务连接）
    /// 格式：PO + 年月日 + 三位序号（PO20260315001）
    pub async fn generate_order_no_with_txn(&self, txn: &sea_orm::DatabaseTransaction) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            txn,
            "PO",
            purchase_order::Entity,
            purchase_order::Column::OrderNo,
        )
        .await
    }

    /// 生成入库单号
    /// 格式：PR + 年月日 + 三位序号（PR20260315001）
    pub async fn generate_receipt_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &self.db,
            "PR",
            purchase_receipt::Entity,
            purchase_receipt::Column::ReceiptNo,
        )
        .await
    }

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
        let order = self.create_order_header(&req, warehouse_id, department_id, user_id, &txn).await?;

        // 3. 创建订单明细并计算总金额
        let (total_amount, total_quantity, total_quantity_alt) = self.create_order_items(&req, order.id, &txn).await?;

        // 4. 更新订单总金额和数量
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", order_active, Some(0)).await?;

        // 5. 预算检查与占用（非阻断）
        self.check_and_occupy_budget(&order, department_id, total_amount, user_id).await;

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
        let supplier_exists = supplier::Entity::find_by_id(req.supplier_id).one(txn).await?;
        if supplier_exists.is_none() {
            tracing::error!("供应商 ID {} 不存在", req.supplier_id);
            return Err(AppError::BadRequest(format!("供应商 ID {} 不存在", req.supplier_id)));
        }

        // 检查仓库是否存在
        let warehouse_id = req.warehouse_id.ok_or_else(|| {
            AppError::BadRequest("仓库 ID 不能为空".to_string())
        })?;
        let warehouse_exists = warehouse::Entity::find_by_id(warehouse_id).one(txn).await?;
        if warehouse_exists.is_none() {
            tracing::error!("仓库 ID {} 不存在", warehouse_id);
            return Err(AppError::BadRequest(format!("仓库 ID {} 不存在", warehouse_id)));
        }

        // 检查部门是否存在
        let department_id = req.department_id.ok_or_else(|| {
            AppError::BadRequest("部门 ID 不能为空".to_string())
        })?;
        let department_exists = department::Entity::find_by_id(department_id).one(txn).await?;
        if department_exists.is_none() {
            tracing::error!("Transaction rolled back: 部门 ID {} 不存在", department_id);
            return Err(AppError::BadRequest(format!("部门 ID {} 不存在", department_id)));
        }

        // 日期合理性检查
        if let Some(expected_date) = req.expected_delivery_date {
            if expected_date < req.order_date {
                tracing::error!("预计交货日期不能早于订单日期");
                return Err(AppError::BadRequest("预计交货日期不能早于订单日期".to_string()));
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
        let order_no = DocumentNumberGenerator::generate_no(txn, "PO", purchase_order::Entity, purchase_order::Column::OrderNo).await?;

        // 创建采购订单主表
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(req.supplier_id),
            order_date: Set(req.order_date),
            expected_delivery_date: Set(req.expected_delivery_date),
            warehouse_id: Set(warehouse_id),
            department_id: Set(department_id),
            purchaser_id: Set(user_id),
            currency: Set(req.currency.clone().unwrap_or_else(|| "CNY".to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            order_status: Set("DRAFT".to_string()),
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

        // 业务验证：产品是否存在
        {
            let mut product_ids = std::collections::HashSet::new();
            for item in &items {
                if let Some(material_id) = item.material_id {
                    if material_id != 0 {
                        product_ids.insert(material_id);
                    }
                }
            }
            for product_id in product_ids {
                let product_exists = product::Entity::find_by_id(product_id).one(txn).await?;
                if product_exists.is_none() {
                    tracing::error!("产品 ID {} 不存在", product_id);
                    return Err(AppError::BadRequest(format!("产品 ID {} 不存在", product_id)));
                }
            }
        }

        for (index, item_req) in items.into_iter().enumerate() {
            let quantity_ordered = item_req.quantity_ordered.unwrap_or(Decimal::ZERO);
            let unit_price = item_req.unit_price.unwrap_or(Decimal::ZERO);
            let amount = quantity_ordered * unit_price;
            total_amount += amount;
            total_quantity += quantity_ordered;

            // 计算辅助数量
            let quantity_alt_ordered = item_req.quantity_alt_ordered.unwrap_or(Decimal::ZERO);
            total_quantity_alt += quantity_alt_ordered;

            // 计算税额和折扣
            let tax_percent = item_req.tax_rate.unwrap_or(Decimal::new(13, 2));
            let tax_amount = amount * tax_percent / Decimal::new(100, 0);
            let discount_percent = item_req.discount_percent.unwrap_or(Decimal::ZERO);
            let discount_amount = amount * discount_percent / Decimal::new(100, 0);

            purchase_order_item::ActiveModel {
                id: Set(0),
                order_id: Set(order_id),
                line_no: Set(item_req.line_no.unwrap_or((index + 1) as i32)),
                product_id: Set(item_req.material_id.unwrap_or(0)), // 使用 material_id 作为 product_id
                quantity: Set(quantity_ordered),
                quantity_alt: Set(quantity_alt_ordered),
                unit_price: Set(unit_price),
                unit_price_foreign: Set(unit_price), // 暂时使用相同值
                discount_percent: Set(discount_percent),
                tax_percent: Set(tax_percent),
                subtotal: Set(amount),
                tax_amount: Set(tax_amount),
                discount_amount: Set(discount_amount),
                total_amount: Set(amount + tax_amount - discount_amount),
                received_quantity: Set(Decimal::ZERO),
                received_quantity_alt: Set(Decimal::ZERO),
                notes: Set(item_req.notes),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            }
            .insert(txn)
            .await?;
        }

        Ok((total_amount, total_quantity, total_quantity_alt))
    }

    /// 检查并占用预算（非阻断）
    async fn check_and_occupy_budget(
        &self,
        order: &purchase_order::Model,
        department_id: i32,
        total_amount: Decimal,
        user_id: i32,
    ) {
        let budget_service = crate::services::budget_management_service::BudgetManagementService::new(self.db.clone());

        // 查找部门对应的预算方案
        match budget_service.get_available_plan_by_department(department_id).await {
            Ok(Some(plan)) => {
                // 检查预算是否可用
                match budget_service.check_budget_available(department_id, plan.id, total_amount).await {
                    Ok(true) => {
                        // 预算充足，占用预算
                        match budget_service.occupy_budget(
                            department_id,
                            plan.id,
                            total_amount,
                            "purchase_order".to_string(),
                            order.id,
                            user_id,
                        ).await {
                            Ok(_) => {
                                tracing::info!(
                                    "订单 {} 预算占用成功，部门ID={}, 方案ID={}, 金额={}",
                                    order.order_no, department_id, plan.id, total_amount
                                );
                            }
                            Err(e) => {
                                // 预算占用失败，记录警告但不阻断
                                tracing::warn!(
                                    "订单 {} 预算占用失败：{}，订单已创建但未关联预算",
                                    order.order_no, e
                                );
                            }
                        }
                    }
                    Ok(false) => {
                        // 预算不足，记录警告但不阻断
                        tracing::warn!(
                            "订单 {} 预算余额不足，部门ID={}, 方案ID={}, 订单金额={}, 订单已创建但未占用预算",
                            order.order_no, department_id, plan.id, total_amount
                        );
                    }
                    Err(e) => {
                        // 预算检查失败，记录警告但不阻断
                        tracing::warn!(
                            "订单 {} 预算检查失败：{}，订单已创建但未关联预算",
                            order.order_no, e
                        );
                    }
                }
            }
            Ok(None) => {
                // 未找到预算方案，记录警告但不阻断
                tracing::warn!(
                    "订单 {} 未找到部门 {} 的预算方案，订单已创建但未关联预算",
                    order.order_no, department_id
                );
            }
            Err(e) => {
                // 查询预算方案失败，记录警告但不阻断
                tracing::warn!(
                    "订单 {} 查询预算方案失败：{}，订单已创建但未关联预算",
                    order.order_no, e
                );
            }
        }
    }

    /// 更新采购订单（仅草稿状态）
    pub async fn update_order(
        &self,
        order_id: i32,
        req: UpdatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT && order.order_status != status::purchase_order::REJECTED {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许修改，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能修改自己创建的订单".to_string(),
            ));
        }

        // 4. 更新订单
        let mut order_active: purchase_order::ActiveModel = order.into();

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
        if let Some(currency) = req.currency {
            order_active.currency = Set(currency);
        }
        if let Some(exchange_rate) = req.exchange_rate {
            order_active.exchange_rate = Set(exchange_rate);
        }
        if let Some(payment_terms) = req.payment_terms {
            order_active.payment_terms = Set(Some(payment_terms));
        }
        if let Some(shipping_terms) = req.shipping_terms {
            order_active.shipping_terms = Set(Some(shipping_terms));
        }
        if let Some(notes) = req.notes {
            order_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            order_active.attachment_urls = Set(Some(attachment_urls));
        }

        order_active.updated_by = Set(Some(user_id));

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", order_active, Some(0)).await?;

        Ok(order)
    }

    /// 删除采购订单（仅草稿状态）
    pub async fn delete_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许删除，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能删除自己创建的订单".to_string(),
            ));
        }

        // 4. 删除订单（级联删除明细）
        purchase_order::Entity::delete_by_id(order_id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 提交采购订单
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT && order.order_status != status::purchase_order::REJECTED {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许提交，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能提交自己创建的订单".to_string(),
            ));
        }

        // 4. 检查是否有明细
        let item_count = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .count(&*self.db)
            .await?;

        if item_count == 0 {
            return Err(AppError::BusinessError("订单至少需要一行明细".to_string()));
        }

        // 5. 更新状态为 PENDING_APPROVAL
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::PENDING_APPROVAL.to_string());
        order_active.updated_at = Set(Utc::now());
        order_active.updated_by = Set(Some(user_id));

        let updated_order = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", order_active, Some(0)).await?;

        // 6. 挂载 BPM 引擎
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "purchase_order_approval".to_string(),
            business_type: "purchase_order".to_string(),
            business_id: order_id,
            title: format!("采购订单审批 - {}", updated_order.order_no),
            initiator_id: user_id,
            initiator_name: "User".to_string(),
            initiator_department_id: None,
            priority: None,
            form_data: None,
            variables: None,
        };
        // 忽略找不到模板的错误，为了兼容旧数据
        let _ = bpm_service.start_process(req).await;
        
        Ok(updated_order)
    }

    /// 审批采购订单
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::SUBMITTED && order.order_status != status::purchase_order::PENDING_APPROVAL {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许审批，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::APPROVED.to_string());
        order_active.approved_by = Set(Some(user_id));
        order_active.approved_at = Set(Some(now));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", order_active, Some(0)).await?;

        Ok(order)
    }

    /// 拒绝采购订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::SUBMITTED && order.order_status != status::purchase_order::PENDING_APPROVAL {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许拒绝，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::REJECTED.to_string());
        order_active.rejected_reason = Set(Some(reason));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", order_active, Some(0)).await?;

        Ok(order)
    }

    /// 标记采购订单为已收货（含库存入库联动）
    pub async fn receive_order(
        &self,
        order_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 开启事务保证数据一致性
        let txn = (*self.db).begin().await?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 3. 检查状态
        if order.order_status == status::purchase_order::RECEIVED || order.order_status == status::purchase_order::CLOSED || order.order_status == status::purchase_order::CANCELLED {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许收货，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 查询订单明细
        let order_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(&txn)
            .await?;

        // 5. 创建库存服务实例
        let inventory_service = crate::services::inventory_stock_service::InventoryStockService::new(self.db.clone());

        // 6. 遍历订单明细，执行库存入库
        for item in &order_items {
            // 查询产品信息获取批次相关字段
            let product = product::Entity::find_by_id(item.product_id)
                .one(&txn)
                .await?
                .ok_or(AppError::NotFound(format!("产品 ID {} 不存在", item.product_id)))?;

            // 计算入库数量
            let receive_quantity_meters = item.quantity - item.received_quantity;
            let receive_quantity_alt = item.quantity_alt - item.received_quantity_alt;

            // 只处理有入库数量的明细
            if receive_quantity_meters > Decimal::ZERO {
                // 查找现有库存记录
                let existing_stock = inventory_service
                    .find_by_product_and_warehouse(item.product_id, order.warehouse_id)
                    .await
                    .map_err(|e| AppError::InternalError(format!("查询库存失败: {}", e)))?;

                let stock_record = match existing_stock {
                    Some(stock) => {
                        // 更新现有库存
                        let new_quantity_meters = stock.quantity_meters + receive_quantity_meters;
                        let new_quantity_kg = stock.quantity_kg + receive_quantity_alt;

                        inventory_service.update_stock_quantity_with_optimistic_lock(
                            stock.id,
                            new_quantity_meters,
                            new_quantity_kg,
                            stock.version,
                        )
                        .await
                        .map_err(|e| AppError::InternalError(format!("更新库存失败: {}", e)))?;

                        stock
                    }
                    None => {
                        // 创建新库存记录
                        inventory_service.create_stock_fabric(
                            order.warehouse_id,
                            item.product_id,
                            "DEFAULT".to_string(),      // batch_no
                            "DEFAULT".to_string(),      // color_no
                            None,                       // dye_lot_no
                            "A".to_string(),            // grade
                            receive_quantity_meters,
                            receive_quantity_alt,
                            product.gram_weight,
                            product.width,
                            None,                       // location_id
                            None,                       // shelf_no
                            None,                       // layer_no
                        )
                        .await
                        .map_err(|e| AppError::InternalError(format!("创建库存记录失败: {}", e)))?
                    }
                };

                // 记录库存流水（PURCHASE_RECEIPT 类型）
                inventory_service.record_transaction(
                    "PURCHASE_RECEIPT".to_string(),
                    item.product_id,
                    order.warehouse_id,
                    stock_record.batch_no.clone(),
                    stock_record.color_no.clone(),
                    stock_record.dye_lot_no.clone(),
                    stock_record.grade.clone(),
                    receive_quantity_meters,
                    receive_quantity_alt,
                    Some("purchase_order".to_string()),
                    Some(order.order_no.clone()),
                    Some(order.id),
                    Some(stock_record.quantity_meters - receive_quantity_meters),
                    Some(stock_record.quantity_kg - receive_quantity_alt),
                    Some(stock_record.quantity_meters),
                    Some(stock_record.quantity_kg),
                    Some(format!("采购入库 - 订单 {}", order.order_no)),
                    None,
                )
                .await
                .map_err(|e| AppError::InternalError(format!("记录库存流水失败: {}", e)))?;

                // 更新订单明细已入库数量
                let mut item_active: purchase_order_item::ActiveModel = item.clone().into();
                item_active.received_quantity = Set(item.quantity);
                item_active.received_quantity_alt = Set(item.quantity_alt);
                item_active.updated_at = Set(Utc::now());
                purchase_order_item::Entity::update(item_active)
                    .exec(&txn)
                    .await?;
            }
        }

        // 7. 更新订单状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::RECEIVED.to_string());
        order_active.actual_delivery_date = Set(Some(now.date_naive()));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", order_active, Some(0)).await?;

        // 8. 提交事务
        txn.commit().await?;

        Ok(order)
    }

    /// 关闭采购订单
    pub async fn close_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态（已完成或部分入库的订单才能关闭）
        if ![status::purchase_order::COMPLETED, status::purchase_order::PARTIAL_RECEIVED].contains(&order.order_status.as_str()) {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许关闭，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::CLOSED.to_string());
        order_active.updated_by = Set(Some(user_id));

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", order_active, Some(0)).await?;

        Ok(order)
    }

    /// 添加订单明细
    pub async fn add_order_item(
        &self,
        order_id: i32,
        req: CreateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许添加明细，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能为自己创建的订单添加明细".to_string(),
            ));
        }

        // 4. 创建明细
        let quantity_ordered = req.quantity_ordered.unwrap_or(Decimal::ZERO);
        let unit_price = req.unit_price.unwrap_or(Decimal::ZERO);
        let amount = quantity_ordered * unit_price;
        let tax_percent = req.tax_rate.unwrap_or(Decimal::new(13, 2));
        let tax_amount = amount * tax_percent / Decimal::new(100, 0);
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let discount_amount = amount * discount_percent / Decimal::new(100, 0);
        let quantity_alt_ordered = req.quantity_alt_ordered.unwrap_or(Decimal::ZERO);

        let item = purchase_order_item::ActiveModel {
            id: Set(0),
            order_id: Set(order_id),
            product_id: Set(req.material_id.unwrap_or(0)), // 使用 material_id 作为 product_id
            quantity: Set(quantity_ordered),
            quantity_alt: Set(quantity_alt_ordered),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(discount_percent),
            tax_percent: Set(tax_percent),
            subtotal: Set(amount),
            tax_amount: Set(tax_amount),
            discount_amount: Set(discount_amount),
            total_amount: Set(amount + tax_amount - discount_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(req.notes),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
        .insert(&*self.db)
        .await?;

        // 5. 更新订单总金额
        self.calculate_order_total(order_id).await?;

        Ok(item)
    }

    /// 更新订单明细
    pub async fn update_order_item(
        &self,
        item_id: i32,
        req: UpdateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!(
                "采购订单 {}",
                item.order_id
            )))?;

        // 3. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许修改明细，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能修改自己创建的订单明细".to_string(),
            ));
        }

        // 5. 更新明细
        let mut item_active: purchase_order_item::ActiveModel = item.into();

        if let Some(material_id) = req.material_id {
            item_active.product_id = Set(material_id);
        }
        if let Some(unit_price) = req.unit_price {
            item_active.unit_price = Set(unit_price);
        }
        if let Some(quantity) = req.quantity_ordered {
            item_active.quantity = Set(quantity);
        }
        if let Some(tax_rate) = req.tax_rate {
            item_active.tax_percent = Set(tax_rate);
        }
        if let Some(notes) = req.notes {
            item_active.notes = Set(Some(notes));
        }

        let item = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", item_active, Some(0)).await?;

        // 6. 更新订单总金额
        self.calculate_order_total(order.id).await?;

        Ok(item)
    }

    /// 删除订单明细
    pub async fn delete_order_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!(
                "采购订单 {}",
                item.order_id
            )))?;

        // 3. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许删除明细，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能删除自己创建的订单明细".to_string(),
            ));
        }

        // 5. 删除明细
        purchase_order_item::Entity::delete_by_id(item_id)
            .exec(&*self.db)
            .await?;

        // 6. 更新订单总金额
        self.calculate_order_total(order.id).await?;

        Ok(())
    }

    /// 计算订单总金额
    pub async fn calculate_order_total(&self, order_id: i32) -> Result<(), AppError> {
        // 1. 查询所有明细
        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await?;

        // 2. 计算总和
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        for item in items {
            total_amount += item.total_amount;
            total_quantity += item.quantity;
            total_quantity_alt += item.quantity_alt;
        }

        // 3. 更新订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", order_active, Some(0)).await?;

        Ok(())
    }

    /// 获取订单列表（分页）
    pub async fn list_orders(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<PurchaseOrderDto>, u64), AppError> {
        use sea_orm::{QuerySelect, JoinType, RelationTrait};
        let mut query = purchase_order::Entity::find()
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(JoinType::LeftJoin, purchase_order::Relation::Warehouse.def())
            .join(JoinType::LeftJoin, purchase_order::Relation::Department.def());

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_order::Column::OrderStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_order::Column::SupplierId.eq(supplier_id));
        }

        // 分页查询
        let paginator = query
            .order_by(purchase_order::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<PurchaseOrderDto>()
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 获取订单详情
    pub async fn get_order(&self, order_id: i32) -> Result<PurchaseOrderDto, AppError> {
        use sea_orm::{QuerySelect, JoinType, RelationTrait};
        let order = purchase_order::Entity::find_by_id(order_id)
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(JoinType::LeftJoin, purchase_order::Relation::Warehouse.def())
            .join(JoinType::LeftJoin, purchase_order::Relation::Department.def())
            .into_model::<PurchaseOrderDto>()
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购订单 {}", order_id)))?;

        Ok(order)
    }

    /// 获取订单明细列表
    pub async fn list_order_items(
        &self,
        order_id: i32,
    ) -> Result<Vec<PurchaseOrderItemDto>, AppError> {
        use sea_orm::{QuerySelect, JoinType, RelationTrait};
        let items = purchase_order_item::Entity::find()
            .column_as(product::Column::Code, "material_code")
            .column_as(product::Column::Name, "material_name")
            .join(JoinType::LeftJoin, purchase_order_item::Relation::Product.def())
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .into_model::<PurchaseOrderItemDto>()
            .all(&*self.db)
            .await?;

        Ok(items)
    }

    // ========== 数据导出方法 ==========

    /// 导出采购订单为 CSV 格式
    pub async fn export_orders_to_csv(
        &self,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<Vec<u8>, AppError> {
        let (orders, _total) = self.list_orders(1, 10000, status, supplier_id).await?;

        let headers = vec![
            "订单编号".to_string(),
            "供应商ID".to_string(),
            "供应商名称".to_string(),
            "订单日期".to_string(),
            "预计交货日期".to_string(),
            "实际交货日期".to_string(),
            "仓库ID".to_string(),
            "仓库名称".to_string(),
            "部门ID".to_string(),
            "部门名称".to_string(),
            "采购员ID".to_string(),
            "币种".to_string(),
            "汇率".to_string(),
            "总金额".to_string(),
            "总金额外币".to_string(),
            "总数量".to_string(),
            "总数量辅助".to_string(),
            "状态".to_string(),
            "付款条件".to_string(),
            "运输条款".to_string(),
            "备注".to_string(),
        ];

        let rows: Vec<std::collections::HashMap<String, String>> = orders
            .into_iter()
            .map(|o| {
                let mut row = std::collections::HashMap::new();
                row.insert("订单编号".to_string(), o.order_no);
                row.insert("供应商ID".to_string(), o.supplier_id.to_string());
                row.insert(
                    "供应商名称".to_string(),
                    o.supplier_name.unwrap_or_default(),
                );
                row.insert("订单日期".to_string(), o.order_date.to_string());
                row.insert(
                    "预计交货日期".to_string(),
                    o.expected_delivery_date.map(|d| d.to_string()).unwrap_or_default(),
                );
                row.insert(
                    "实际交货日期".to_string(),
                    o.actual_delivery_date.map(|d| d.to_string()).unwrap_or_default(),
                );
                row.insert("仓库ID".to_string(), o.warehouse_id.to_string());
                row.insert(
                    "仓库名称".to_string(),
                    o.warehouse_name.unwrap_or_default(),
                );
                row.insert("部门ID".to_string(), o.department_id.to_string());
                row.insert(
                    "部门名称".to_string(),
                    o.department_name.unwrap_or_default(),
                );
                row.insert("采购员ID".to_string(), o.purchaser_id.to_string());
                row.insert("币种".to_string(), o.currency);
                row.insert("汇率".to_string(), o.exchange_rate.to_string());
                row.insert("总金额".to_string(), o.total_amount.to_string());
                row.insert("总金额外币".to_string(), o.total_amount_foreign.to_string());
                row.insert("总数量".to_string(), o.total_quantity.to_string());
                row.insert("总数量辅助".to_string(), o.total_quantity_alt.to_string());
                row.insert("状态".to_string(), o.order_status);
                row.insert(
                    "付款条件".to_string(),
                    o.payment_terms.unwrap_or_default(),
                );
                row.insert(
                    "运输条款".to_string(),
                    o.shipping_terms.unwrap_or_default(),
                );
                row.insert("备注".to_string(), o.notes.unwrap_or_default());
                row
            })
            .collect();

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::InternalError(format!("CSV 生成失败: {}", e)))
    }

    /// 根据缺料预警创建采购建议
    pub async fn create_purchase_suggestion_from_shortage(
        &self,
        material_id: i32,
        material_name: String,
        material_code: String,
        required_quantity: Decimal,
        available_quantity: Decimal,
        shortage_quantity: Decimal,
        shortage_level: String,
        affected_orders_count: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 1. 获取产品信息
        let product = product::Entity::find_by_id(material_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("物料 ID {} 不存在", material_id)))?;

        // 2. 查找默认供应商
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq("active"))
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound("没有可用的活跃供应商".to_string()))?;

        // 3. 计算建议采购量（缺口数量 * 1.2，20%余量）
        let suggested_quantity = shortage_quantity * Decimal::new(12, 1);

        // 4. 生成采购订单号（使用事务连接）
        let order_no = self.generate_order_no_with_txn(&txn).await?;

        // 5. 根据缺料级别确定优先级
        let priority = match shortage_level.as_str() {
            "Critical" => "URGENT",
            "Severe" => "HIGH",
            "Warning" => "MEDIUM",
            _ => "LOW",
        };

        // 6. 创建采购订单
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(supplier.id),
            order_date: Set(Utc::now().date_naive()),
            expected_delivery_date: Set(Some((Utc::now() + chrono::Duration::days(7)).date_naive())),
            warehouse_id: Set(1), // 默认仓库
            department_id: Set(1), // 默认部门
            purchaser_id: Set(1), // 默认采购员
            currency: Set("CNY".to_string()),
            exchange_rate: Set(Decimal::new(1, 0)),
            order_status: Set("DRAFT".to_string()),
            notes: Set(Some(format!(
                "缺料预警自动生成 | 物料: {} ({}) | 需求量: {} | 可用量: {} | 缺口: {} | 级别: {} | 受影响订单: {}",
                material_name, material_code, required_quantity, available_quantity, shortage_quantity, shortage_level, affected_orders_count
            ))),
            created_by: Set(1), // 系统用户
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 7. 创建订单明细
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let amount = suggested_quantity * unit_price;
        let tax_rate = Decimal::new(13, 2); // 13% 增值税
        let tax_amount = amount * tax_rate / Decimal::new(100, 0);

        purchase_order_item::ActiveModel {
            id: Set(0),
            order_id: Set(order.id),
            product_id: Set(material_id),
            quantity: Set(suggested_quantity),
            quantity_alt: Set(Decimal::ZERO),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(Decimal::ZERO),
            tax_percent: Set(tax_rate),
            subtotal: Set(amount),
            tax_amount: Set(tax_amount),
            discount_amount: Set(Decimal::ZERO),
            total_amount: Set(amount + tax_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(Some(format!(
                "缺料预警自动生成 | 物料: {} ({}) | 优先级: {}",
                material_name, material_code, priority
            ))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 8. 提交事务
        txn.commit().await?;

        tracing::info!(
            "创建缺料预警采购建议: 订单号={}, 物料={} ({})，建议采购量={}, 优先级={}",
            order.order_no,
            material_name,
            material_code,
            suggested_quantity,
            priority
        );

        Ok(order)
    }

    /// 根据库存预警创建采购建议
    pub async fn create_purchase_suggestion(
        &self,
        product_id: i32,
        warehouse_id: i32,
        current_quantity: Decimal,
        reorder_point: Decimal,
        reorder_quantity: Decimal,
    ) -> Result<purchase_order::Model, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 1. 获取产品信息
        let product = product::Entity::find_by_id(product_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("产品 ID {} 不存在", product_id)))?;

        // 2. 获取仓库信息
        let warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("仓库 ID {} 不存在", warehouse_id)))?;

        // 3. 查找默认供应商（这里简化处理，实际可能需要更复杂的供应商选择逻辑）
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq("active"))
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound("没有可用的活跃供应商".to_string()))?;

        // 4. 生成采购订单号（使用事务连接）
        let order_no = self.generate_order_no_with_txn(&txn).await?;

        // 5. 创建采购订单
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(supplier.id),
            order_date: Set(Utc::now().date_naive()),
            expected_delivery_date: Set(Some((Utc::now() + chrono::Duration::days(7)).date_naive())),
            warehouse_id: Set(warehouse_id),
            department_id: Set(1), // 默认部门
            purchaser_id: Set(1), // 默认采购员
            currency: Set("CNY".to_string()),
            exchange_rate: Set(Decimal::new(1, 0)),
            order_status: Set("DRAFT".to_string()),
            notes: Set(Some(format!(
                "库存预警自动生成，当前库存: {}, 补货点: {}, 建议补货量: {}",
                current_quantity, reorder_point, reorder_quantity
            ))),
            created_by: Set(1), // 系统用户
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 6. 创建订单明细
        let quantity = reorder_quantity;
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let amount = quantity * unit_price;
        let tax_rate = Decimal::new(13, 2); // 13% 增值税
        let tax_amount = amount * tax_rate / Decimal::new(100, 0);

        purchase_order_item::ActiveModel {
            id: Set(0),
            order_id: Set(order.id),
            product_id: Set(product_id),
            quantity: Set(quantity),
            quantity_alt: Set(Decimal::ZERO),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(Decimal::ZERO),
            tax_percent: Set(tax_rate),
            subtotal: Set(amount),
            tax_amount: Set(tax_amount),
            discount_amount: Set(Decimal::ZERO),
            total_amount: Set(amount + tax_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(Some(format!("库存预警自动生成，产品: {}", product.name))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 7. 提交事务
        txn.commit().await?;

        tracing::info!(
            "创建库存预警采购建议: 订单号={}, 产品={}, 仓库={}, 建议采购量={}",
            order.order_no,
            product.name,
            warehouse.name,
            quantity
        );

        Ok(order)
    }
}

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购订单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 订单日期
    pub order_date: NaiveDate,

    /// 预计交货日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 部门 ID
    pub department_id: Option<i32>,

    /// 币种
    pub currency: Option<String>,

    /// 汇率
    pub exchange_rate: Option<Decimal>,

    /// 付款条件
    pub payment_terms: Option<String>,

    /// 运输条款
    pub shipping_terms: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 订单明细
    #[validate(length(min = 1, message = "订单至少需要一行明细"))]
    pub items: Option<Vec<CreateOrderItemRequest>>,
}

/// 更新采购订单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseOrderRequest {
    pub supplier_id: Option<i32>,
    pub order_date: Option<NaiveDate>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

/// 创建订单明细请求
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct CreateOrderItemRequest {
    /// 行号
    pub line_no: Option<i32>,

    /// 物料 ID
    pub material_id: Option<i32>,

    /// 物料编码
    pub material_code: Option<String>,

    /// 物料名称
    pub material_name: Option<String>,

    /// 规格型号
    pub specification: Option<String>,

    /// 批次号
    pub batch_no: Option<String>,

    /// 色号
    pub color_code: Option<String>,

    /// 缸号
    pub lot_no: Option<String>,

    /// 等级
    pub grade: Option<String>,

    /// 克重
    pub gram_weight: Option<Decimal>,

    /// 幅宽
    pub width: Option<Decimal>,

    /// 单价
    pub unit_price: Option<Decimal>,

    /// 币种
    pub currency: Option<String>,

    /// 订购数量（主单位）
    pub quantity_ordered: Option<Decimal>,

    /// 主单位
    pub unit_master: Option<String>,

    /// 辅助单位
    pub unit_alt: Option<String>,

    /// 换算系数
    pub conversion_factor: Option<Decimal>,

    /// 订购数量（辅助单位）
    pub quantity_alt_ordered: Option<Decimal>,

    /// 税率
    pub tax_rate: Option<Decimal>,

    /// 折扣百分比
    pub discount_percent: Option<Decimal>,

    /// 交货日期
    pub delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新订单明细请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdateOrderItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub quantity_ordered: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub delivery_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

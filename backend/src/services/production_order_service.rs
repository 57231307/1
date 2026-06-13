//! 生产订单 Service
//!
//! 提供生产订单的CRUD操作和状态管理

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

use crate::models::production_order::{
    ActiveModel, Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
use crate::utils::error::AppError;

use crate::models::bom::{Column as BomColumn, Entity as BomEntity};
use crate::models::bom_item::{Column as BomItemColumn, Entity as BomItemEntity};
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::product::Entity as ProductEntity;
use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::warehouse::Entity as WarehouseEntity;
use crate::models::work_center::Entity as WorkCenterEntity;

/// 创建生产订单请求
#[derive(Debug, Clone)]
pub struct CreateProductionOrderRequest {
    pub order_no: Option<String>,
    pub sales_order_id: Option<i32>,
    pub product_id: i32,
    pub planned_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: i32,
}

/// 更新生产订单请求
#[derive(Debug, Clone)]
pub struct UpdateProductionOrderRequest {
    pub planned_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
}

/// 生产订单查询参数
#[derive(Debug, Clone)]
pub struct ProductionOrderQuery {
    pub status: Option<String>,
    pub product_id: Option<i32>,
    pub page: u64,
    pub page_size: u64,
}

/// 生产订单 Service
pub struct ProductionOrderService {
    db: Arc<DatabaseConnection>,
}

impl ProductionOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 验证产品是否存在
    async fn validate_product_exists(&self, product_id: i32) -> Result<(), AppError> {
        let product = ProductEntity::find_by_id(product_id).one(&*self.db).await?;

        if product.is_none() {
            return Err(AppError::validation(format!(
                "产品ID {} 不存在",
                product_id
            )));
        }
        Ok(())
    }

    /// 验证销售订单是否存在
    async fn validate_sales_order_exists(&self, sales_order_id: i32) -> Result<(), AppError> {
        let sales_order = SalesOrderEntity::find_by_id(sales_order_id)
            .one(&*self.db)
            .await?;

        if sales_order.is_none() {
            return Err(AppError::validation(format!(
                "销售订单ID {} 不存在",
                sales_order_id
            )));
        }
        Ok(())
    }

    /// 验证工作中心是否存在
    async fn validate_work_center_exists(&self, work_center_id: i32) -> Result<(), AppError> {
        let work_center = WorkCenterEntity::find_by_id(work_center_id)
            .one(&*self.db)
            .await?;

        if work_center.is_none() {
            return Err(AppError::validation(format!(
                "工作中心ID {} 不存在",
                work_center_id
            )));
        }
        Ok(())
    }

    /// 生成唯一订单号（带重试机制）
    async fn generate_unique_order_no(&self) -> Result<String, AppError> {
        let max_retries = 5;
        for _ in 0..max_retries {
            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let random = crate::utils::random::random_4_digit();
            let order_no = format!("PO-{}-{:04}", timestamp, random);

            // 检查订单号是否已存在
            let existing = ProductionOrderEntity::find()
                .filter(crate::models::production_order::Column::OrderNo.eq(&order_no))
                .one(&*self.db)
                .await?;

            if existing.is_none() {
                return Ok(order_no);
            }
        }
        Err(AppError::internal(
            "无法生成唯一订单号，请稍后重试".to_string(),
        ))
    }

    /// 验证状态转换是否合法
    fn validate_status_transition(current_status: &str, new_status: &str) -> Result<(), AppError> {
        let valid_transitions = std::collections::HashMap::from([
            ("DRAFT", vec!["SCHEDULED", "PENDING_APPROVAL", "CANCELLED"]),
            ("SCHEDULED", vec!["IN_PROGRESS", "CANCELLED"]),
            ("IN_PROGRESS", vec!["COMPLETED", "CANCELLED"]),
            ("COMPLETED", vec![]),
            ("CANCELLED", vec![]),
            ("PENDING_APPROVAL", vec!["APPROVED", "REJECTED"]),
            ("APPROVED", vec!["SCHEDULED"]),
            ("REJECTED", vec!["DRAFT"]),
        ]);

        if let Some(allowed) = valid_transitions.get(current_status) {
            if allowed.contains(&new_status) {
                Ok(())
            } else {
                Err(AppError::business(format!(
                    "不允许从 {} 状态转换到 {} 状态",
                    current_status, new_status
                )))
            }
        } else {
            Err(AppError::business(format!(
                "未知的状态: {}",
                current_status
            )))
        }
    }

    /// 创建生产订单
    pub async fn create(
        &self,
        req: CreateProductionOrderRequest,
    ) -> Result<ProductionOrderModel, AppError> {
        // 验证产品是否存在
        self.validate_product_exists(req.product_id).await?;

        // 验证BOM是否存在（生产订单需要BOM进行物料计算）
        let has_bom = BomEntity::find()
            .filter(BomColumn::ProductId.eq(req.product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .filter(BomColumn::Status.eq("ACTIVE"))
            .one(&*self.db)
            .await?
            .is_some();

        if !has_bom {
            tracing::warn!(
                "产品ID {} 没有默认BOM，生产完成时将无法自动扣减原材料",
                req.product_id
            );
        }

        // 验证销售订单是否存在（如果提供）
        if let Some(sales_order_id) = req.sales_order_id {
            self.validate_sales_order_exists(sales_order_id).await?;
        }

        // 验证工作中心是否存在（如果提供）
        if let Some(work_center_id) = req.work_center_id {
            self.validate_work_center_exists(work_center_id).await?;
        }

        // 生成或验证订单号
        let order_no = match req.order_no {
            Some(no) => {
                // 检查提供的订单号是否已存在
                let existing = ProductionOrderEntity::find()
                    .filter(crate::models::production_order::Column::OrderNo.eq(&no))
                    .one(&*self.db)
                    .await?;

                if existing.is_some() {
                    return Err(AppError::validation(format!("订单号 {} 已存在", no)));
                }
                no
            }
            None => self.generate_unique_order_no().await?,
        };

        let active_model = ActiveModel {
            order_no: Set(order_no),
            sales_order_id: Set(req.sales_order_id),
            product_id: Set(req.product_id),
            planned_quantity: Set(req.planned_quantity.unwrap_or_default()),
            planned_start_date: Set(req.planned_start_date),
            planned_end_date: Set(req.planned_end_date),
            status: Set("DRAFT".to_string()),
            priority: Set(req.priority.unwrap_or(0)),
            work_center_id: Set(req.work_center_id),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let model = active_model.insert(&*self.db).await.map_err(|e| {
            // 处理唯一约束冲突
            let err_str = e.to_string();
            if err_str.contains("unique constraint") || err_str.contains("duplicate") {
                AppError::validation("订单号已存在，请稍后重试")
            } else {
                AppError::database(e.to_string())
            }
        })?;

        Ok(model)
    }

    /// 根据ID获取生产订单
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ProductionOrderModel>, AppError> {
        let model = ProductionOrderEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 获取生产订单列表
    pub async fn list(
        &self,
        query: ProductionOrderQuery,
    ) -> Result<(Vec<ProductionOrderModel>, u64), AppError> {
        let mut select = ProductionOrderEntity::find();

        if let Some(status) = query.status {
            select = select.filter(crate::models::production_order::Column::Status.eq(status));
        }

        if let Some(product_id) = query.product_id {
            select =
                select.filter(crate::models::production_order::Column::ProductId.eq(product_id));
        }

        let total = select.clone().count(&*self.db).await?;

        let paginator = select
            .order_by_desc(crate::models::production_order::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator.fetch_page(query.page - 1).await?;

        Ok((models, total))
    }

    /// 更新生产订单
    pub async fn update(
        &self,
        id: i32,
        req: UpdateProductionOrderRequest,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 只允许编辑草稿和已排产状态的订单
        if !matches!(model.status.as_str(), "DRAFT" | "SCHEDULED") {
            return Err(AppError::business(format!(
                "不允许编辑 {} 状态的生产订单",
                model.status
            )));
        }

        let mut active_model: ActiveModel = model.into();

        if let Some(planned_quantity) = req.planned_quantity {
            active_model.planned_quantity = Set(planned_quantity);
        }
        if let Some(planned_start_date) = req.planned_start_date {
            active_model.planned_start_date = Set(Some(planned_start_date));
        }
        if let Some(planned_end_date) = req.planned_end_date {
            active_model.planned_end_date = Set(Some(planned_end_date));
        }
        if let Some(priority) = req.priority {
            active_model.priority = Set(priority);
        }
        if let Some(work_center_id) = req.work_center_id {
            active_model.work_center_id = Set(Some(work_center_id));
        }
        if let Some(remarks) = req.remarks {
            active_model.remarks = Set(Some(remarks));
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除生产订单（软删除 - 设为取消状态）
    #[allow(dead_code)]
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证是否可以取消
        Self::validate_status_transition(&model.status, "CANCELLED")?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("CANCELLED".to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 更新生产订单状态
    pub async fn update_status(
        &self,
        id: i32,
        status: String,
        actual_quantity: Option<Decimal>,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        Self::validate_status_transition(&model.status, &status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(status.clone());
        active_model.updated_at = Set(Utc::now());

        // 如果状态变为生产中，设置实际开始日期
        if status == "IN_PROGRESS" {
            active_model.actual_start_date = Set(Some(chrono::Utc::now().date_naive()));
        }

        // 如果状态变为已完成，设置实际完成日期和实际生产数量
        if status == "COMPLETED" {
            active_model.actual_end_date = Set(Some(chrono::Utc::now().date_naive()));
            if let Some(qty) = actual_quantity {
                active_model.actual_quantity = Set(Some(qty));
            }
        }

        let updated = active_model.update(&*self.db).await?;

        // 生产完成时执行库存联动：扣减原材料 + 入库成品
        if status == "COMPLETED" {
            self.handle_production_completion_inventory(&updated)
                .await?;
        }

        Ok(updated)
    }

    /// 处理生产完成时的库存联动
    ///
    /// 1. 查询产品默认BOM，扣减原材料库存（按BOM用量 × 生产数量）
    /// 2. 增加成品库存（生产数量）
    /// 3. 记录库存流水（PRODUCTION_CONSUMPTION 和 PRODUCTION_OUTPUT）
    async fn handle_production_completion_inventory(
        &self,
        order: &ProductionOrderModel,
    ) -> Result<(), AppError> {
        let stock_service =
            crate::services::inventory_stock_service::InventoryStockService::new(self.db.clone());

        // 查询默认成品仓库（取第一个激活的仓库）
        let default_warehouse = WarehouseEntity::find()
            .filter(crate::models::warehouse::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::business("未找到可用仓库，无法执行库存联动"))?;

        // 优先使用实际完成数量，否则使用计划数量
        let production_qty = order.actual_quantity.unwrap_or(order.planned_quantity);
        if production_qty.is_zero() {
            return Err(AppError::business(
                "生产数量为零，无法执行库存联动".to_string(),
            ));
        }

        // ========== 1. 扣减原材料库存 ==========
        // 查询该产品的默认BOM
        let bom = BomEntity::find()
            .filter(BomColumn::ProductId.eq(order.product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .filter(BomColumn::Status.eq("ACTIVE"))
            .one(&*self.db)
            .await?;

        if let Some(bom) = bom {
            // 查询BOM明细
            let bom_items = BomItemEntity::find()
                .filter(BomItemColumn::BomId.eq(bom.id))
                .all(&*self.db)
                .await?;

            for bom_item in bom_items {
                let consumption_qty = bom_item.quantity * production_qty;

                // 查找该原材料在默认仓库的库存记录
                let stock_record = InventoryStockEntity::find()
                    .filter(
                        crate::models::inventory_stock::Column::ProductId.eq(bom_item.material_id),
                    )
                    .filter(
                        crate::models::inventory_stock::Column::WarehouseId
                            .eq(default_warehouse.id),
                    )
                    .one(&*self.db)
                    .await?
                    .ok_or_else(|| {
                        AppError::business(format!(
                            "原材料(ID={})在默认仓库中无库存记录，无法扣减",
                            bom_item.material_id
                        ))
                    })?;

                let qty_before_meters = stock_record.quantity_meters;
                let qty_before_kg = stock_record.quantity_kg;

                // 检查库存是否充足
                if qty_before_meters < consumption_qty {
                    return Err(AppError::business(format!(
                        "原材料(ID={})库存不足，需要 {}，当前库存 {}",
                        bom_item.material_id, consumption_qty, qty_before_meters
                    )));
                }

                let qty_after_meters = qty_before_meters - consumption_qty;

                // 按比例计算公斤数扣减
                let qty_after_kg = if qty_before_meters > Decimal::ZERO {
                    qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)
                } else {
                    qty_before_kg
                };

                // 更新库存数量（带乐观锁）
                stock_service
                    .update_stock_quantity_with_optimistic_lock(
                        stock_record.id,
                        qty_after_meters,
                        qty_after_kg,
                        stock_record.version,
                    )
                    .await?;

                // 记录库存流水：生产消耗
                stock_service
                    .record_transaction(
                        "PRODUCTION_CONSUMPTION".to_string(),
                        bom_item.material_id,
                        default_warehouse.id,
                        stock_record.batch_no.clone(),
                        stock_record.color_no.clone(),
                        stock_record.dye_lot_no.clone(),
                        stock_record.grade.clone(),
                        consumption_qty,
                        Decimal::ZERO,
                        Some("production_order".to_string()),
                        Some(order.order_no.clone()),
                        Some(order.id),
                        Some(qty_before_meters),
                        Some(qty_before_kg),
                        Some(qty_after_meters),
                        Some(qty_after_kg),
                        Some(format!("生产消耗 - 订单 {}", order.order_no)),
                        Some(order.created_by),
                    )
                    .await?;
            }
        }

        // ========== 2. 增加成品库存 ==========
        // 查询成品产品信息以获取克重和幅宽
        let product = ProductEntity::find_by_id(order.product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::business(format!("产品ID {} 不存在", order.product_id)))?;

        // 查找成品在默认仓库的已有库存记录
        let existing_stock = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.eq(order.product_id))
            .filter(crate::models::inventory_stock::Column::WarehouseId.eq(default_warehouse.id))
            .one(&*self.db)
            .await?;

        match existing_stock {
            Some(stock_record) => {
                // 更新已有库存
                let qty_before_meters = stock_record.quantity_meters;
                let qty_before_kg = stock_record.quantity_kg;
                let qty_after_meters = qty_before_meters + production_qty;

                // 计算公斤数（如果有克重和幅宽）
                let added_kg = if let (Some(gw), Some(w)) = (product.gram_weight, product.width) {
                    production_qty * gw * w / Decimal::new(100000, 0)
                } else {
                    Decimal::ZERO
                };
                let qty_after_kg = qty_before_kg + added_kg;

                stock_service
                    .update_stock_quantity_with_optimistic_lock(
                        stock_record.id,
                        qty_after_meters,
                        qty_after_kg,
                        stock_record.version,
                    )
                    .await?;

                // 记录库存流水：生产入库
                stock_service
                    .record_transaction(
                        "PRODUCTION_OUTPUT".to_string(),
                        order.product_id,
                        default_warehouse.id,
                        stock_record.batch_no.clone(),
                        stock_record.color_no.clone(),
                        stock_record.dye_lot_no.clone(),
                        stock_record.grade.clone(),
                        production_qty,
                        added_kg,
                        Some("production_order".to_string()),
                        Some(order.order_no.clone()),
                        Some(order.id),
                        Some(qty_before_meters),
                        Some(qty_before_kg),
                        Some(qty_after_meters),
                        Some(qty_after_kg),
                        Some(format!("生产入库 - 订单 {}", order.order_no)),
                        Some(order.created_by),
                    )
                    .await?;
            }
            None => {
                // 创建新的库存记录
                let kg = if let (Some(gw), Some(w)) = (product.gram_weight, product.width) {
                    production_qty * gw * w / Decimal::new(100000, 0)
                } else {
                    Decimal::ZERO
                };

                let new_stock = stock_service
                    .create_stock_fabric(
                        default_warehouse.id,
                        order.product_id,
                        order.order_no.clone(),
                        "DEFAULT".to_string(),
                        None,
                        "一等品".to_string(),
                        production_qty,
                        kg,
                        product.gram_weight,
                        product.width,
                        None,
                        None,
                        None,
                    )
                    .await?;

                // 记录库存流水：生产入库
                stock_service
                    .record_transaction(
                        "PRODUCTION_OUTPUT".to_string(),
                        order.product_id,
                        default_warehouse.id,
                        new_stock.batch_no.clone(),
                        new_stock.color_no.clone(),
                        new_stock.dye_lot_no.clone(),
                        new_stock.grade.clone(),
                        production_qty,
                        kg,
                        Some("production_order".to_string()),
                        Some(order.order_no.clone()),
                        Some(order.id),
                        Some(Decimal::ZERO),
                        Some(Decimal::ZERO),
                        Some(production_qty),
                        Some(kg),
                        Some(format!("生产入库 - 订单 {}", order.order_no)),
                        Some(order.created_by),
                    )
                    .await?;
            }
        }

        tracing::info!(
            "生产订单 {} 完成库存联动：成品入库 {}，已扣减原材料库存",
            order.order_no,
            production_qty
        );

        Ok(())
    }

    /// 提交生产订单审批
    pub async fn submit_for_approval(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        Self::validate_status_transition(&model.status, "PENDING_APPROVAL")?;

        // 更新状态为审批中
        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("PENDING_APPROVAL".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        // 启动BPM审批流程
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "production_order_approval".to_string(),
            business_type: "production_order".to_string(),
            business_id: id,
            title: format!("生产订单审批 - {}", updated.order_no),
            initiator_id: user_id,
            initiator_name: user_name.to_string(),
            initiator_department_id: None,
            priority: Some("HIGH".to_string()),
            form_data: Some(serde_json::json!({
                "order_no": updated.order_no,
                "product_id": updated.product_id,
                "planned_quantity": updated.planned_quantity,
                "work_center_id": updated.work_center_id,
            })),
            variables: None,
        };

        // 忽略找不到模板的错误，为了兼容旧数据
        let _ = bpm_service.start_process(req).await;

        Ok(updated)
    }

    /// 审批生产订单
    pub async fn approve_order(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
        approved: bool,
        opinion: Option<String>,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        let new_status = if approved { "APPROVED" } else { "REJECTED" };
        Self::validate_status_transition(&model.status, new_status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        // 完成BPM任务 - 通过process_instance关联
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());

        // 获取与该生产订单关联的流程实例
        if let Ok(Some(instance)) = bpm_service
            .get_process_by_business("production_order", id)
            .await
        {
            // 获取该实例的待处理任务
            let tasks = bpm_service
                .query_user_tasks(crate::models::dto::bpm_dto::TaskQuery {
                    user_id: Some(user_id),
                    status: Some("PENDING".to_string()),
                    page: Some(1),
                    page_size: Some(10),
                })
                .await;

            if let Ok(task_list) = tasks {
                for task in task_list.data {
                    // 只处理当前流程实例的任务
                    if task.instance_id == instance.id {
                        let _ = bpm_service
                            .approve_task(crate::models::dto::bpm_dto::ApproveTaskRequest {
                                task_id: task.id,
                                handler_id: user_id,
                                handler_name: user_name.to_string(),
                                action: if approved {
                                    "approve".to_string()
                                } else {
                                    "reject".to_string()
                                },
                                approval_opinion: opinion.clone(),
                                attachment_urls: None,
                            })
                            .await;
                    }
                }
            }
        }

        Ok(updated)
    }
}

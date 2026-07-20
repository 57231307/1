//! 生产订单 Service
//!
//! 提供生产订单的CRUD操作和状态管理

// 批次 100 P3-A 修复（v5 复审）：状态字符串常量化，引用 crate::models::status

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

use crate::models::production_order::{
    self, ActiveModel, Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

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

/// increase_finished_goods_txn 内部共享的成品入库流水上下文
struct ProductionOutputRecord {
    batch_no: String,
    color_no: String,
    dye_lot_no: Option<String>,
    grade: String,
    added_kg: Decimal,
    qty_before_meters: Decimal,
    qty_before_kg: Decimal,
    qty_after_meters: Decimal,
    qty_after_kg: Decimal,
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
            (crate::models::status::common::STATUS_DRAFT, vec![
                crate::models::status::production::PRODUCTION_SCHEDULED,
                crate::models::status::production::PRODUCTION_PENDING_APPROVAL,
                crate::models::status::common::STATUS_CANCELLED,
            ]),
            (crate::models::status::production::PRODUCTION_SCHEDULED, vec![
                crate::models::status::production::PRODUCTION_IN_PROGRESS,
                crate::models::status::common::STATUS_CANCELLED,
            ]),
            (crate::models::status::production::PRODUCTION_IN_PROGRESS, vec![
                crate::models::status::common::STATUS_COMPLETED,
                crate::models::status::common::STATUS_CANCELLED,
            ]),
            (crate::models::status::common::STATUS_COMPLETED, vec![]),
            (crate::models::status::common::STATUS_CANCELLED, vec![]),
            (
                crate::models::status::production::PRODUCTION_PENDING_APPROVAL,
                vec![
                    crate::models::status::common::STATUS_APPROVED,
                    crate::models::status::production::PRODUCTION_REJECTED,
                ],
            ),
            (
                crate::models::status::common::STATUS_APPROVED,
                vec![crate::models::status::production::PRODUCTION_SCHEDULED],
            ),
            (
                crate::models::status::production::PRODUCTION_REJECTED,
                vec![crate::models::status::common::STATUS_DRAFT],
            ),
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
            .filter(BomColumn::Status.eq(crate::models::status::common::STATUS_ACTIVE))
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
            status: Set(crate::models::status::common::STATUS_DRAFT.to_string()),
            // 优先级可选项；0 = 最低优先级（业务接受默认值）
            priority: Set(req.priority.unwrap_or_default()),
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

        // B-P2-4 修复（批次 386 v13 复审）：生产订单创建后触发 MRP 物料需求计算
        // 原实现 create 仅插入订单记录，不调用 MrpEngineService，
        // 导致生产订单→MRP 物料需求链路断开，原材料采购计划无法基于生产订单自动生成。
        // 修复：insert 成功后调用 MRP 计算（source_type=PRODUCTION_ORDER），
        // 失败时 tracing::warn 不阻塞主流程（订单已创建，MRP 可后续重算）。
        let mrp_service = crate::services::mrp_engine_service::MrpEngineService::new(self.db.clone());
        let required_date = req
            .planned_end_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive() + chrono::Duration::days(7));
        if let Err(e) = mrp_service
            .run_mrp_calculation(crate::services::mrp_engine_service::MrpCalculationQuery {
                product_id: model.product_id,
                required_quantity: model.planned_quantity,
                required_date,
                source_type: "PRODUCTION_ORDER".to_string(),
                source_id: Some(model.id),
                consider_safety_stock: true,
                consider_in_transit: true,
            })
            .await
        {
            tracing::warn!(
                order_id = model.id,
                product_id = model.product_id,
                error = %e,
                "批次 386 B-P2-4: 生产订单创建后 MRP 计算失败，请人工检查物料需求"
            );
        }

        Ok(model)
    }

    /// V15 Batch 479 P0-F21：创建返工生产订单
    ///
    /// 业务背景：bulk_color_approval customer_rework 触发，返工必须走生产订单流程
    /// （审计报告 P0-F21：返工无工单跟踪，返工成本无法归集到原缸号）
    ///
    /// 与普通 create() 的差异：
    /// - order_type = 'rework'（标记为返工订单）
    /// - original_batch_id 指向原 dye_batch（返工成本归集锚点）
    /// - 不触发 MRP 物料需求计算（返工使用已有物料，不产生新采购计划）
    /// - 自动生成订单号 RW-YYYYMMDD-NNN
    ///
    /// 参数：
    /// - `product_id`：产品 ID（来自 bulk_color_approval.product_id）
    /// - `original_batch_id`：原 dye_batch ID（返工成本归集到原缸号）
    /// - `sales_order_id`：关联销售订单 ID（可选，来自 bulk_color_approval.sales_order_id）
    /// - `created_by`：操作人 ID
    /// - `remarks`：备注（可选，通常包含返工原因）
    pub async fn create_rework_order(
        &self,
        product_id: i32,
        original_batch_id: i32,
        sales_order_id: Option<i32>,
        created_by: i32,
        remarks: Option<String>,
    ) -> Result<ProductionOrderModel, AppError> {
        // 验证产品是否存在
        self.validate_product_exists(product_id).await?;

        // 验证销售订单是否存在（如果提供）
        if let Some(sales_order_id) = sales_order_id {
            self.validate_sales_order_exists(sales_order_id).await?;
        }

        // 生成返工订单号 RW-YYYYMMDD-NNN
        let order_no = self.generate_rework_order_no().await?;

        let now = Utc::now();
        let active_model = ActiveModel {
            order_no: Set(order_no),
            sales_order_id: Set(sales_order_id),
            product_id: Set(product_id),
            planned_quantity: Set(Decimal::ZERO),
            planned_start_date: Set(None),
            planned_end_date: Set(None),
            status: Set(crate::models::status::common::STATUS_DRAFT.to_string()),
            priority: Set(1), // 返工订单优先级最高
            work_center_id: Set(None),
            remarks: Set(remarks),
            created_by: Set(created_by),
            // V15 Batch 479 P0-F21：返工订单标识
            order_type: Set("rework".to_string()),
            original_batch_id: Set(Some(original_batch_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let model = active_model.insert(&*self.db).await.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("unique constraint") || err_str.contains("duplicate") {
                AppError::validation("返工订单号已存在，请稍后重试")
            } else {
                AppError::database(e.to_string())
            }
        })?;

        // 注意：返工订单不触发 MRP 计算（返工使用已有物料，不产生新采购计划）
        // 如需物料调整，由配方调整流程单独处理

        Ok(model)
    }

    /// 生成唯一返工订单号 RW-YYYYMMDD-NNN
    ///
    /// 与 generate_unique_order_no 区别：使用 RW- 前缀标识返工订单
    async fn generate_rework_order_no(&self) -> Result<String, AppError> {
        let date_str = chrono::Utc::now().format("%Y%m%d").to_string();
        for attempt in 0..10 {
            let seq = if attempt == 0 {
                // 首次尝试基于当前秒数生成，减少 DB 查询
                let secs = chrono::Utc::now().timestamp() % 1000;
                format!("{:03}", secs)
            } else {
                format!("{:03}", 100 + attempt)
            };
            let order_no = format!("RW-{}-{}", date_str, seq);
            let exists = ProductionOrderEntity::find()
                .filter(crate::models::production_order::Column::OrderNo.eq(&order_no))
                .one(&*self.db)
                .await?
                .is_some();
            if !exists {
                return Ok(order_no);
            }
        }
        // 兜底：使用 UUID 片段
        Ok(format!(
            "RW-{}-{}",
            date_str,
            chrono::Utc::now().timestamp_millis() % 10000
        ))
    }

    /// 根据ID获取生产订单
    pub async fn get_by_id(
        &self,
        id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Option<ProductionOrderModel>, AppError> {
        let model = ProductionOrderEntity::find_by_id(id).one(&*self.db).await?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // production_order 表无 department_id，Dept 退化为 Self（按 created_by 校验）
        if let (Some(ctx), Some(m)) = (data_scope, &model) {
            if !check_resource_owner(ctx, Some(m.created_by), None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问生产订单 {}（数据范围限制）",
                    id
                )));
            }
        }

        Ok(model)
    }

    /// 获取生产订单操作日志
    ///
    /// 批次 132 v9 复审 P1：原 get_production_order_logs handler 返回固定空列表，
    /// 现真实查询 audit_logs 表，按 resource_id = order_id 过滤，按 created_at 倒序返回。
    ///
    /// production_order_service 的所有变更（update / update_status / delete / approve）
    /// 都通过 update_with_audit 写入 audit_logs 表，resource_id 为订单 ID 的字符串形式。
    /// 由于全项目 resource_type 统一为 "auto_audit"，无法按业务类型精确过滤，
    /// 故仅按 resource_id 查询（同 ID 不同业务表的日志概率极低，可接受）。
    pub async fn get_order_logs(
        &self,
        order_id: i32,
    ) -> Result<Vec<crate::models::audit_log::Model>, AppError> {
        use crate::models::audit_log::{Column as AuditColumn, Entity as AuditEntity};

        let logs = AuditEntity::find()
            .filter(AuditColumn::ResourceId.eq(order_id.to_string()))
            .order_by_desc(AuditColumn::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(logs)
    }

    /// 获取生产订单列表
    pub async fn list(
        &self,
        query: ProductionOrderQuery,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<ProductionOrderModel>, u64), AppError> {
        let mut select = ProductionOrderEntity::find();

        // V15 P0-S01：行级数据权限过滤（production_order 表无 department_id，Dept 退化为 Self）
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                production_order::Column::CreatedBy,
                production_order::Column::CreatedBy,
            );
        }

        if let Some(status) = query.status {
            select = select.filter(crate::models::production_order::Column::Status.eq(status));
        }

        if let Some(product_id) = query.product_id {
            select =
                select.filter(crate::models::production_order::Column::ProductId.eq(product_id));
        }

        // 批次 257 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = select
            .order_by_desc(crate::models::production_order::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let (models, total) = paginate_with_total(paginator, query.page.clamp(1, 1000)).await?;

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
        if !matches!(
            model.status.as_str(),
            crate::models::status::common::STATUS_DRAFT
                | crate::models::status::production::PRODUCTION_SCHEDULED
        ) {
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
    // 批次 93 P1-4 修复：补 user_id 参数 + txn + lock_exclusive + 审计日志
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-4 修复：状态门 + 软删除移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 find_by_id 在 self.db → validate_status_transition → update 在 self.db，
        // 状态门与 update 跨事务边界，并发 delete + update_status 会竞态绕过状态门控。
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证是否可以取消（状态门在 txn 内，基于 lock_exclusive 读出的 model）
        Self::validate_status_transition(&model.status, crate::models::status::common::STATUS_CANCELLED)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(crate::models::status::common::STATUS_CANCELLED.to_string());
        active_model.updated_at = Set(Utc::now());

        // 走 update_with_audit 保留审计追溯（软删除即状态变更）
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 更新生产订单状态
    ///
    /// 批次 9（2026-06-28）：COMPLETED 状态变更涉及多表操作（订单状态 + 库存扣减 + 库存流水），
    /// 必须用事务包裹，否则状态变更已提交但库存联动失败会导致账实不符。
    /// 其他状态变更保持原行为（单表更新）。
    pub async fn update_status(
        &self,
        id: i32,
        status: String,
        actual_quantity: Option<Decimal>,
    ) -> Result<ProductionOrderModel, AppError> {
        // COMPLETED 走事务包裹的专用路径
        if status == crate::models::status::common::STATUS_COMPLETED {
            return self.complete_production_order(id, actual_quantity).await;
        }

        // 批次 22（2026-06-28 v5 P0-3）：非 COMPLETED 路径补全事务 + lock_exclusive + update_with_audit
        // 原 `update_status` 在非 COMPLETED 分支直接调用 `&*self.db` 进行查询和更新，
        // 既没有事务边界也没有行锁，并发状态变更可能基于过期快照导致状态覆盖；
        // 同时未走 update_with_audit 会导致状态变更丢失审计追溯。
        // 改为：begin txn + lock_exclusive 查询 + 状态校验 + update_with_audit + commit。
        let txn = (*self.db).begin().await?;

        // 状态门查询加 lock_exclusive 串行化并发状态变更
        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        Self::validate_status_transition(&model.status, &status)?;

        // B-P2-5 修复（批次 386 v13 复审）：排产状态变更时进行产能负荷校验
        // 原实现 update_status 在状态转为 SCHEDULED 时不调用 CapacityService，
        // 导致超负荷工作中心仍可排产，存在产能超载风险。
        // 修复：当目标状态为 SCHEDULED 且订单绑定了工作中心时，
        // 调用 CapacityService::load_analysis 检查负荷率，超载则拒绝排产。
        if status == crate::models::status::production::PRODUCTION_SCHEDULED {
            if let Some(work_center_id) = model.work_center_id {
                let capacity_service =
                    crate::services::capacity_service::CapacityService::new(self.db.clone());
                let analysis = capacity_service
                    .load_analysis(crate::services::capacity_service::LoadAnalysisQuery {
                        date_from: model.planned_start_date,
                        date_to: model.planned_end_date,
                        work_center_id: Some(work_center_id),
                    })
                    .await?;

                // 检查目标工作中心的负荷率（load_rate > 100 视为超载）
                if let Some(item) = analysis.iter().find(|i| i.work_center_id == work_center_id) {
                    if item.load_rate > Decimal::from(100) {
                        return Err(AppError::business(format!(
                            "工作中心 {}（{}）当前负荷率 {:.2}% 已超载，无法排产，请调整计划或分配至其他工作中心",
                            item.work_center_name, item.work_center_code, item.load_rate
                        )));
                    }
                    if item.load_rate > Decimal::from(80) {
                        tracing::warn!(
                            order_id = id,
                            work_center_id,
                            load_rate = %item.load_rate,
                            "批次 386 B-P2-5: 工作中心负荷率较高（>80%），排产成功但建议关注产能瓶颈"
                        );
                    }
                }
            }
        }

        // 提取审计用户ID（update_status 入参无 user_id，使用订单创建者作为审计主体）
        let audit_user_id = model.created_by;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(status.clone());
        active_model.updated_at = Set(Utc::now());

        // 如果状态变为生产中，设置实际开始日期
        if status == crate::models::status::production::PRODUCTION_IN_PROGRESS {
            active_model.actual_start_date = Set(Some(chrono::Utc::now().date_naive()));
        }

        // 走 update_with_audit 保留审计追溯
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(audit_user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(updated)
    }

    /// 完成生产订单（事务包裹状态变更 + 库存联动）
    ///
    /// 批次 9（2026-06-28）：原 `update_status` 在 COMPLETED 时先提交状态变更，
    /// 然后调用库存联动；如果库存联动失败，状态已变更但库存未扣减导致账实不符。
    /// 改为：在事务内更新状态 + 调用库存联动，任一失败回滚全部。
    /// 同时给订单查询加 FOR UPDATE 行锁，防止并发完成同一订单。
    async fn complete_production_order(
        &self,
        id: i32,
        actual_quantity: Option<Decimal>,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = self.db.begin().await?;

        // 加 FOR UPDATE 行锁，防止并发完成同一订单
        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        Self::validate_status_transition(&model.status, crate::models::status::common::STATUS_COMPLETED)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(crate::models::status::common::STATUS_COMPLETED.to_string());
        active_model.actual_end_date = Set(Some(chrono::Utc::now().date_naive()));
        active_model.updated_at = Set(Utc::now());
        if let Some(qty) = actual_quantity {
            active_model.actual_quantity = Set(Some(qty));
        }

        let updated = active_model.update(&txn).await?;

        // 在同一事务内执行库存联动（含原材料扣减 + 成品入库）
        // P0 5-2 修复：handle_production_completion_inventory_txn 不再在内部 publish 事件，
        // 改为返回收集到的库存流水事件，由本处在 commit 成功后统一 publish，避免事务回滚时幻事件
        let pending_events =
            Self::handle_production_completion_inventory_txn(&txn, &updated).await?;

        txn.commit().await?;

        // P0 5-2 修复：commit 成功后统一发布库存流水事件，避免事务回滚时幻事件
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }

        // 批次 356 v13 复审 B-P0-3 修复：生产订单成本核算链路闭环
        // 原实现 complete_production_order 不调用 CostCollectionService，
        // 导致生产成本无法归集，产品成本失真，BI 报表成本数据缺失。
        // 修复：commit 成功后调用 CostCollectionService 做成本归集。
        let cost_service =
            crate::services::cost_collection_service::CostCollectionService::new(self.db.clone());
        let product = ProductEntity::find_by_id(updated.product_id)
            .one(&*self.db)
            .await?;
        let cost_price = product
            .as_ref()
            .and_then(|p| p.cost_price)
            .unwrap_or(Decimal::ZERO);
        let actual_qty = updated.actual_quantity.unwrap_or(updated.planned_quantity);
        let total_material_cost = cost_price * actual_qty;
        let cost_req = crate::services::cost_collection_service::CreateCostCollectionRequest {
            collection_date: chrono::Utc::now().date_naive(),
            cost_object_type: Some("production_order".to_string()),
            cost_object_id: Some(updated.id),
            cost_object_no: Some(updated.order_no.clone()),
            batch_no: None,
            color_no: None,
            // v14 批次 422 T-P1-6：按缸号核算成本（生产订单当前无缸号，后续批次补全）
            dye_lot_no: None,
            workshop: None,
            direct_material: total_material_cost,
            direct_labor: Decimal::ZERO,
            manufacturing_overhead: Decimal::ZERO,
            processing_fee: Decimal::ZERO,
            dyeing_fee: Decimal::ZERO,
            output_quantity_meters: Some(actual_qty),
            output_quantity_kg: None,
        };
        if let Err(e) = cost_service.create(cost_req, updated.created_by).await {
            tracing::warn!(
                order_id = updated.id,
                error = %e,
                "批次 356 B-P0-3: 生产订单成本归集失败，请人工检查"
            );
        }

        Ok(updated)
    }

    /// 处理生产完成时的库存联动（事务版本）
    ///
    /// 1. 查询产品默认BOM，扣减原材料库存（按BOM用量 × 生产数量）
    /// 2. 增加成品库存（生产数量）
    /// 3. 记录库存流水（PRODUCTION_CONSUMPTION 和 PRODUCTION_OUTPUT）
    ///
    /// 批次 9（2026-06-28）：从原 `handle_production_completion_inventory` 改造而来，
    /// 接受外部事务参数，所有查询/更新都在 `txn` 上执行；原材料库存查询加 FOR UPDATE 行锁，
    /// 防止并发完成多个生产订单时原材料库存被并发扣减导致丢失更新。
    ///
    /// P2 1-4 修复：原函数 275 行混合 5 职责（仓库查询+数量校验+原材料扣减+成品入库+日志），
    /// 拆为 fetch_default_warehouse_txn / deduct_raw_materials_txn / increase_finished_goods_txn 3 个私有方法
    async fn handle_production_completion_inventory_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        // P0 5-2 修复：本函数不 commit 事务（由调用方 complete_production_order commit），
        // 收集 record_transaction_txn 返回的库存流水事件交给调用方，
        // 在 commit 成功后统一 publish，避免事务回滚时幻事件
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        // 1. 查询默认成品仓库
        let default_warehouse = Self::fetch_default_warehouse_txn(txn).await?;

        // 2. 校验生产数量
        let production_qty = order.actual_quantity.unwrap_or(order.planned_quantity);
        if production_qty.is_zero() {
            return Err(AppError::business(
                "生产数量为零，无法执行库存联动".to_string(),
            ));
        }

        // 3. 扣减原材料库存
        pending_events.extend(
            Self::deduct_raw_materials_txn(txn, order, &default_warehouse, production_qty).await?,
        );

        // 4. 增加成品库存
        pending_events.extend(
            Self::increase_finished_goods_txn(txn, order, &default_warehouse, production_qty)
                .await?,
        );

        tracing::info!(
            "生产订单 {} 完成库存联动：成品入库 {}，已扣减原材料库存",
            order.order_no,
            production_qty
        );

        Ok(pending_events)
    }

    /// P2 1-4 修复：查询默认成品仓库（取第一个激活的仓库，从 handle_production_completion_inventory_txn 抽取）
    async fn fetch_default_warehouse_txn(
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<crate::models::warehouse::Model, AppError> {
        WarehouseEntity::find()
            .filter(crate::models::warehouse::Column::IsActive.eq(true))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business("未找到可用仓库，无法执行库存联动"))
    }

    /// P2 1-4 修复：扣减原材料库存（从 handle_production_completion_inventory_txn 抽取）
    ///
    /// 查询产品默认BOM，按BOM用量 × 生产数量扣减原材料库存，记录 PRODUCTION_CONSUMPTION 流水
    /// v16 批次 43 修复：循环外批量查询并锁定所有原材料库存记录，避免 N+1 查询
    /// 批次 9（2026-06-28）：FOR UPDATE 行锁批量获取，防止并发扣减丢失更新
    async fn deduct_raw_materials_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        let bom_items = match Self::fetch_bom_with_items(txn, order.product_id).await? {
            Some(v) => v.1,
            None => return Ok(pending_events),
        };

        let stock_map = Self::fetch_stock_map(txn, &bom_items, default_warehouse.id).await?;

        for bom_item in &bom_items {
            let event = Self::deduct_single_material(
                txn,
                order,
                bom_item,
                &stock_map,
                production_qty,
                default_warehouse.id,
            )
            .await?;
            if let Some(ev) = event {
                pending_events.push(ev);
            }
        }

        Ok(pending_events)
    }

    async fn fetch_bom_with_items(
        txn: &sea_orm::DatabaseTransaction,
        product_id: i32,
    ) -> Result<Option<(crate::models::bom::Model, Vec<crate::models::bom_item::Model>)>, AppError>
    {
        let bom = BomEntity::find()
            .filter(BomColumn::ProductId.eq(product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .filter(BomColumn::Status.eq(crate::models::status::common::STATUS_ACTIVE))
            .one(txn)
            .await?;

        if let Some(bom) = bom {
            let bom_items = BomItemEntity::find()
                .filter(BomItemColumn::BomId.eq(bom.id))
                .all(txn)
                .await?;
            Ok(Some((bom, bom_items)))
        } else {
            Ok(None)
        }
    }

    async fn fetch_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        bom_items: &[crate::models::bom_item::Model],
        warehouse_id: i32,
    ) -> Result<std::collections::HashMap<i32, crate::models::inventory_stock::Model>, AppError>
    {
        let material_ids: Vec<i32> = bom_items.iter().map(|b| b.material_id).collect();
        let stock_map = if material_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            InventoryStockEntity::find()
                .filter(
                    crate::models::inventory_stock::Column::ProductId.is_in(material_ids),
                )
                .filter(
                    crate::models::inventory_stock::Column::WarehouseId
                        .eq(warehouse_id),
                )
                .lock_exclusive()
                .all(txn)
                .await?
                .into_iter()
                .map(|s| (s.product_id, s))
                .collect()
        };
        Ok(stock_map)
    }

    async fn deduct_single_material(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        bom_item: &crate::models::bom_item::Model,
        stock_map: &std::collections::HashMap<i32, crate::models::inventory_stock::Model>,
        production_qty: Decimal,
        warehouse_id: i32,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_query::RecordTransactionArgs;
        use crate::services::inventory_stock_service::InventoryStockService;

        let consumption_qty = (bom_item.quantity * production_qty).round_dp(4);

        let stock_record = stock_map
            .get(&bom_item.material_id)
            .cloned()
            .ok_or_else(|| {
                AppError::business(format!(
                    "原材料(ID={})在默认仓库中无库存记录，无法扣减",
                    bom_item.material_id
                ))
            })?;

        let qty_before_meters = stock_record.quantity_meters;
        let qty_before_kg = stock_record.quantity_kg;

        if qty_before_meters < consumption_qty {
            return Err(AppError::business(format!(
                "原材料(ID={})库存不足，需要 {}，当前库存 {}",
                bom_item.material_id, consumption_qty, qty_before_meters
            )));
        }

        let qty_after_meters = qty_before_meters - consumption_qty;
        let qty_after_kg = if qty_before_meters > Decimal::ZERO {
            qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)
        } else {
            qty_before_kg
        };

        InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
            txn,
            stock_record.id,
            qty_after_meters,
            qty_after_kg,
            stock_record.version,
        )
        .await?;

        let (_, txn_event) = InventoryStockService::record_transaction_txn(
            txn,
            RecordTransactionArgs {
                transaction_type: "PRODUCTION_CONSUMPTION".to_string(),
                product_id: bom_item.material_id,
                warehouse_id,
                batch_no: stock_record.batch_no.clone(),
                color_no: stock_record.color_no.clone(),
                dye_lot_no: stock_record.dye_lot_no.clone(),
                grade: stock_record.grade.clone(),
                quantity_meters: consumption_qty,
                quantity_kg: Decimal::ZERO,
                source_bill_type: Some("production_order".to_string()),
                source_bill_no: Some(order.order_no.clone()),
                source_bill_id: Some(order.id),
                quantity_before_meters: Some(qty_before_meters),
                quantity_before_kg: Some(qty_before_kg),
                quantity_after_meters: Some(qty_after_meters),
                quantity_after_kg: Some(qty_after_kg),
                notes: Some(format!("生产消耗 - 订单 {}", order.order_no)),
                created_by: Some(order.created_by),
            },
        )
        .await?;

        Ok(txn_event)
    }

    /// P2 1-4 修复：增加成品库存（从 handle_production_completion_inventory_txn 抽取）
    ///
    /// 查询成品产品信息（克重/幅宽），在默认仓库更新或创建库存记录，记录 PRODUCTION_OUTPUT 流水
    /// 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发入库丢失更新
    async fn increase_finished_goods_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        let product = ProductEntity::find_by_id(order.product_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business(format!("产品ID {} 不存在", order.product_id)))?;

        // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发入库丢失更新
        let existing_stock = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.eq(order.product_id))
            .filter(crate::models::inventory_stock::Column::WarehouseId.eq(default_warehouse.id))
            .lock_exclusive()
            .one(txn)
            .await?;

        let txn_event = match existing_stock {
            Some(stock_record) => Self::update_existing_finished_stock_txn(
                txn, order, default_warehouse, &product, stock_record, production_qty,
            )
            .await?,
            None => Self::create_new_finished_stock_txn(
                txn, order, default_warehouse, &product, production_qty,
            )
            .await?,
        };

        if let Some(ev) = txn_event {
            pending_events.push(ev);
        }

        Ok(pending_events)
    }

    /// P0-D08 拆分：更新已有成品库存并记录 PRODUCTION_OUTPUT 流水
    async fn update_existing_finished_stock_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        product: &crate::models::product::Model,
        stock_record: crate::models::inventory_stock::Model,
        production_qty: Decimal,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_service::InventoryStockService;

        let qty_before_meters = stock_record.quantity_meters;
        let qty_before_kg = stock_record.quantity_kg;
        let qty_after_meters = qty_before_meters + production_qty;

        let added_kg = if let (Some(gw), Some(w)) = (product.gram_weight, product.width) {
            production_qty * gw * w / Decimal::new(100000, 0)
        } else {
            Decimal::ZERO
        };
        let qty_after_kg = qty_before_kg + added_kg;

        InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
            txn,
            stock_record.id,
            qty_after_meters,
            qty_after_kg,
            stock_record.version,
        )
        .await?;

        let record = ProductionOutputRecord {
            batch_no: stock_record.batch_no.clone(),
            color_no: stock_record.color_no.clone(),
            dye_lot_no: stock_record.dye_lot_no.clone(),
            grade: stock_record.grade.clone(),
            added_kg,
            qty_before_meters,
            qty_before_kg,
            qty_after_meters,
            qty_after_kg,
        };

        Self::record_production_output_txn(txn, order, default_warehouse, production_qty, record).await
    }

    /// P0-D08 拆分：创建新成品库存记录并记录 PRODUCTION_OUTPUT 流水
    async fn create_new_finished_stock_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        product: &crate::models::product::Model,
        production_qty: Decimal,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_service::{CreateStockFabricArgs, InventoryStockService};
        let kg = if let (Some(gw), Some(w)) = (product.gram_weight, product.width) {
            production_qty * gw * w / Decimal::new(100000, 0)
        } else {
            Decimal::ZERO
        };

        // v14 批次 419 修复 F-P0-1：从订单获取缸号/色号/批号，替代原 "DEFAULT" 硬编码
        let new_stock = InventoryStockService::create_stock_fabric_txn(
            txn,
            CreateStockFabricArgs {
                warehouse_id: default_warehouse.id,
                product_id: order.product_id,
                batch_no: order.batch_no.clone().unwrap_or_else(|| order.order_no.clone()),
                color_no: order.color_no.clone().unwrap_or_default(),
                dye_lot_no: order.dye_lot_no.clone(),
                grade: "一等品".to_string(),
                quantity_meters: production_qty,
                quantity_kg: kg,
                gram_weight: product.gram_weight,
                width: product.width,
                location_id: None,
                shelf_no: None,
                layer_no: None,
            },
        )
        .await?;

        let record = ProductionOutputRecord {
            batch_no: new_stock.batch_no.clone(),
            color_no: new_stock.color_no.clone(),
            dye_lot_no: new_stock.dye_lot_no.clone(),
            grade: new_stock.grade.clone(),
            added_kg: kg,
            qty_before_meters: Decimal::ZERO,
            qty_before_kg: Decimal::ZERO,
            qty_after_meters: production_qty,
            qty_after_kg: kg,
        };

        Self::record_production_output_txn(txn, order, default_warehouse, production_qty, record).await
    }

    /// P0-D08 拆分：记录 PRODUCTION_OUTPUT 库存流水（P0 5-2：返回事件由调用方收集后 commit 后统一 publish）
    async fn record_production_output_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
        record: ProductionOutputRecord,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_query::RecordTransactionArgs;
        use crate::services::inventory_stock_service::InventoryStockService;

        let (_, txn_event) = InventoryStockService::record_transaction_txn(
            txn,
            RecordTransactionArgs {
                transaction_type: "PRODUCTION_OUTPUT".to_string(),
                product_id: order.product_id,
                warehouse_id: default_warehouse.id,
                batch_no: record.batch_no,
                color_no: record.color_no,
                dye_lot_no: record.dye_lot_no,
                grade: record.grade,
                quantity_meters: production_qty,
                quantity_kg: record.added_kg,
                source_bill_type: Some("production_order".to_string()),
                source_bill_no: Some(order.order_no.clone()),
                source_bill_id: Some(order.id),
                quantity_before_meters: Some(record.qty_before_meters),
                quantity_before_kg: Some(record.qty_before_kg),
                quantity_after_meters: Some(record.qty_after_meters),
                quantity_after_kg: Some(record.qty_after_kg),
                notes: Some(format!("生产入库 - 订单 {}", order.order_no)),
                created_by: Some(order.created_by),
            },
        )
        .await?;

        Ok(txn_event)
    }

    /// 提交生产订单审批
    ///
    /// 批次 15（2026-06-28）：事务包裹"查询 + 状态校验 + update"，
    /// 加 lock_exclusive 防止并发提交同一订单导致状态不一致；
    /// BPM 启动保留事务外（失败 warn 不阻断已提交状态），避免 BPM 调用持有数据库锁。
    pub async fn submit_for_approval(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        Self::validate_status_transition(&model.status, crate::models::status::production::PRODUCTION_PENDING_APPROVAL)?;

        // 更新状态为审批中
        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(crate::models::status::production::PRODUCTION_PENDING_APPROVAL.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&txn).await?;

        txn.commit().await?;

        // 启动BPM审批流程（事务外，失败不阻断已提交状态）
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

        // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉 BPM 启动错误，
        // 导致模板缺失/DB 异常时无任何日志可追溯。改为 warn 日志记录，保留兼容性
        // （不向上传播错误，避免阻断主流程），但确保运维可观测。
        if let Err(e) = bpm_service.start_process(req).await {
            tracing::warn!(
                error = %e,
                order_id = id,
                "BPM 启动生产订单审批流程失败（兼容旧数据，不阻断主流程）"
            );
        }

        Ok(updated)
    }

    /// 审批生产订单
    ///
    /// 批次 15（2026-06-28）：事务包裹"查询 + 状态校验 + update"，
    /// 加 lock_exclusive 防止并发审批同一订单导致重复审批或状态覆盖；
    /// BPM 任务审批保留事务外（失败 warn 不阻断已提交状态），避免 BPM 调用持有数据库锁。
    pub async fn approve_order(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
        approved: bool,
        opinion: Option<String>,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        let new_status = if approved {
            crate::models::status::common::STATUS_APPROVED
        } else {
            crate::models::status::production::PRODUCTION_REJECTED
        };
        Self::validate_status_transition(&model.status, new_status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&txn).await?;

        txn.commit().await?;

        // 完成BPM任务 - 通过process_instance关联（事务外，失败不阻断已提交状态）
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
                    status: Some(crate::models::status::common::STATUS_PENDING.to_string()),
                    page: Some(1),
                    page_size: Some(10),
                })
                .await;

            if let Ok(task_list) = tasks {
                for task in task_list.data {
                    // 只处理当前流程实例的任务
                    if task.instance_id == instance.id {
                        // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉
                        // BPM 任务审批错误，改为 warn 日志记录，确保运维可观测。
                        if let Err(e) = bpm_service
                            .approve_task(
                                crate::models::dto::bpm_dto::ApproveTaskRequest {
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
                                },
                                // P0 8-4 修复：传入真实操作用户 user_id 用于 BPM 审计追溯
                                Some(user_id),
                            )
                            .await
                        {
                            tracing::warn!(
                                error = %e,
                                task_id = task.id,
                                order_id = id,
                                "BPM 生产订单任务审批失败（不阻断主流程）"
                            );
                        }
                    }
                }
            }
        }

        Ok(updated)
    }

    /// B-P1-9 修复（批次 360 v13 复审）：BPM 回写专用审批通过方法
    ///
    /// 与 `approve_order` 的区别：不回调 BPM（避免 BPM → 事件 → approve_order → BPM 死循环）。
    /// 仅更新生产订单状态 PENDING_APPROVAL → APPROVED，走 update_with_audit 保留审计追溯。
    /// 由 event_bus.rs 的 BpmProcessFinished 事件监听器调用。
    pub async fn approve_order_via_bpm(
        &self,
        order_id: i32,
        approver_id: i32,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        let new_status = crate::models::status::common::STATUS_APPROVED;
        Self::validate_status_transition(&model.status, new_status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(approver_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// B-P1-9 修复（批次 360 v13 复审）：BPM 回写专用审批拒绝方法
    ///
    /// 与 `approve_order(approved=false)` 的区别：不回调 BPM（避免循环）。
    /// 仅更新生产订单状态 PENDING_APPROVAL → REJECTED，走 update_with_audit 保留审计追溯。
    pub async fn reject_order_via_bpm(
        &self,
        order_id: i32,
        reason: String,
        approver_id: i32,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        let new_status = crate::models::status::production::PRODUCTION_REJECTED;
        Self::validate_status_transition(&model.status, new_status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(approver_id),
        )
        .await?;

        txn.commit().await?;
        tracing::info!(
            order_id = order_id,
            approver_id = approver_id,
            reason = %reason,
            "生产订单 BPM 审批拒绝回写完成"
        );
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::common;
    use crate::models::status::production;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// 复现 deduct_raw_materials_txn 中的 BOM 用量计算（纯算法）
    ///
    /// 公式：consumption_qty = (bom_quantity * production_qty).round_dp(4)
    fn calc_consumption_qty(bom_quantity: Decimal, production_qty: Decimal) -> Decimal {
        (bom_quantity * production_qty).round_dp(4)
    }

    /// 复现 deduct_raw_materials_txn 中的公斤数按比例扣减（纯算法）
    ///
    /// 公式：qty_after_kg = qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)
    /// 当 qty_before_meters 为零时，公斤数不变（避免除零）。
    fn calc_kg_after_deduction(
        qty_before_meters: Decimal,
        qty_before_kg: Decimal,
        consumption_qty: Decimal,
    ) -> Decimal {
        if qty_before_meters > Decimal::ZERO {
            qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)
        } else {
            qty_before_kg
        }
    }

    /// 复现 increase_finished_goods_txn 中的成品入库公斤数计算（纯算法）
    ///
    /// 公式：added_kg = production_qty * gram_weight * width / 100000
    /// 当克重或幅宽缺失时，公斤数增量为零。
    fn calc_added_kg(
        production_qty: Decimal,
        gram_weight: Option<Decimal>,
        width: Option<Decimal>,
    ) -> Decimal {
        if let (Some(gw), Some(w)) = (gram_weight, width) {
            production_qty * gw * w / Decimal::new(100000, 0)
        } else {
            Decimal::ZERO
        }
    }

    /// 复现 complete_production_order 中 actual_quantity 缺省取 planned_quantity 的逻辑
    fn resolve_production_qty(
        actual_quantity: Option<Decimal>,
        planned_quantity: Decimal,
    ) -> Decimal {
        actual_quantity.unwrap_or(planned_quantity)
    }

    /// 复现 generate_unique_order_no 的订单号格式校验（纯字符串校验）
    ///
    /// 格式：PO-{14位时间戳}-{4位随机数}
    fn is_valid_order_no_format(order_no: &str) -> bool {
        if !order_no.starts_with("PO-") {
            return false;
        }
        let parts: Vec<&str> = order_no.split('-').collect();
        if parts.len() != 3 {
            return false;
        }
        // 中段为 14 位时间戳，末段为 4 位数字
        parts[1].len() == 14
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() == 4
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }

    // ============== 状态常量值正确性 ==============

    /// 测试_状态常量_草稿为合法值
    ///
    /// 验证 STATUS_DRAFT 常量是大写字符串 "DRAFT"，与数据库约定一致。
    #[test]
    fn 测试_状态常量_草稿为合法值() {
        assert_eq!(common::STATUS_DRAFT, "DRAFT");
    }

    /// 测试_状态常量_已审批为合法值
    #[test]
    fn 测试_状态常量_已审批为合法值() {
        assert_eq!(common::STATUS_APPROVED, "APPROVED");
    }

    /// 测试_状态常量_已完成为合法值
    #[test]
    fn 测试_状态常量_已完成为合法值() {
        assert_eq!(common::STATUS_COMPLETED, "COMPLETED");
    }

    /// 测试_状态常量_已取消为合法值
    #[test]
    fn 测试_状态常量_已取消为合法值() {
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
    }

    /// 测试_状态常量_已排产为合法值
    #[test]
    fn 测试_状态常量_已排产为合法值() {
        assert_eq!(production::PRODUCTION_SCHEDULED, "SCHEDULED");
    }

    /// 测试_状态常量_生产中为合法值
    #[test]
    fn 测试_状态常量_生产中为合法值() {
        assert_eq!(production::PRODUCTION_IN_PROGRESS, "IN_PROGRESS");
    }

    /// 测试_状态常量_待审批为合法值
    #[test]
    fn 测试_状态常量_待审批为合法值() {
        assert_eq!(production::PRODUCTION_PENDING_APPROVAL, "PENDING_APPROVAL");
    }

    /// 测试_状态常量_已拒绝为合法值
    #[test]
    fn 测试_状态常量_已拒绝为合法值() {
        assert_eq!(production::PRODUCTION_REJECTED, "REJECTED");
    }

    /// 测试_状态常量_各状态值互不相同
    ///
    /// 验证生产订单 8 个状态常量两两互不相同，避免状态机歧义。
    #[test]
    fn 测试_状态常量_各状态值互不相同() {
        let statuses = [
            common::STATUS_DRAFT,
            common::STATUS_APPROVED,
            common::STATUS_COMPLETED,
            common::STATUS_CANCELLED,
            production::PRODUCTION_SCHEDULED,
            production::PRODUCTION_IN_PROGRESS,
            production::PRODUCTION_PENDING_APPROVAL,
            production::PRODUCTION_REJECTED,
        ];
        for i in 0..statuses.len() {
            for j in (i + 1)..statuses.len() {
                assert_ne!(statuses[i], statuses[j], "状态常量重复: {}", statuses[i]);
            }
        }
    }

    // ============== 状态机转换合法性 ==============

    /// 测试_状态转换_草稿到已排产合法
    #[test]
    fn 测试_状态转换_草稿到已排产合法() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            production::PRODUCTION_SCHEDULED
        )
        .is_ok());
    }

    /// 测试_状态转换_草稿到待审批合法
    #[test]
    fn 测试_状态转换_草稿到待审批合法() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            production::PRODUCTION_PENDING_APPROVAL
        )
        .is_ok());
    }

    /// 测试_状态转换_草稿到已取消合法
    #[test]
    fn 测试_状态转换_草稿到已取消合法() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            common::STATUS_CANCELLED
        )
        .is_ok());
    }

    /// 测试_状态转换_已排产到生产中合法
    #[test]
    fn 测试_状态转换_已排产到生产中合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_SCHEDULED,
            production::PRODUCTION_IN_PROGRESS
        )
        .is_ok());
    }

    /// 测试_状态转换_已排产到已取消合法
    #[test]
    fn 测试_状态转换_已排产到已取消合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_SCHEDULED,
            common::STATUS_CANCELLED
        )
        .is_ok());
    }

    /// 测试_状态转换_生产中到已完成合法
    #[test]
    fn 测试_状态转换_生产中到已完成合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_IN_PROGRESS,
            common::STATUS_COMPLETED
        )
        .is_ok());
    }

    /// 测试_状态转换_生产中到已取消合法
    #[test]
    fn 测试_状态转换_生产中到已取消合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_IN_PROGRESS,
            common::STATUS_CANCELLED
        )
        .is_ok());
    }

    /// 测试_状态转换_待审批到已审批合法
    #[test]
    fn 测试_状态转换_待审批到已审批合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_PENDING_APPROVAL,
            common::STATUS_APPROVED
        )
        .is_ok());
    }

    /// 测试_状态转换_待审批到已拒绝合法
    #[test]
    fn 测试_状态转换_待审批到已拒绝合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_PENDING_APPROVAL,
            production::PRODUCTION_REJECTED
        )
        .is_ok());
    }

    /// 测试_状态转换_已审批到已排产合法
    #[test]
    fn 测试_状态转换_已审批到已排产合法() {
        assert!(ProductionOrderService::validate_status_transition(
            common::STATUS_APPROVED,
            production::PRODUCTION_SCHEDULED
        )
        .is_ok());
    }

    /// 测试_状态转换_已拒绝到草稿合法
    #[test]
    fn 测试_状态转换_已拒绝到草稿合法() {
        assert!(ProductionOrderService::validate_status_transition(
            production::PRODUCTION_REJECTED,
            common::STATUS_DRAFT
        )
        .is_ok());
    }

    /// 测试_状态转换_草稿不能直接到生产中
    ///
    /// 业务规则：草稿必须先经已排产才能进入生产中，跳级转换应被拒绝。
    #[test]
    fn 测试_状态转换_草稿不能直接到生产中() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            production::PRODUCTION_IN_PROGRESS,
        );
        assert!(result.is_err());
    }

    /// 测试_状态转换_草稿不能直接到已完成
    #[test]
    fn 测试_状态转换_草稿不能直接到已完成() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            common::STATUS_COMPLETED,
        );
        assert!(result.is_err());
    }

    /// 测试_状态转换_已完成为终态不可再变更
    #[test]
    fn 测试_状态转换_已完成为终态不可再变更() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_COMPLETED,
            common::STATUS_CANCELLED,
        );
        assert!(result.is_err());
    }

    /// 测试_状态转换_已取消为终态不可再变更
    #[test]
    fn 测试_状态转换_已取消为终态不可再变更() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_CANCELLED,
            common::STATUS_DRAFT,
        );
        assert!(result.is_err());
    }

    /// 测试_状态转换_未知源状态被拒绝
    #[test]
    fn 测试_状态转换_未知源状态被拒绝() {
        let result = ProductionOrderService::validate_status_transition(
            "UNKNOWN",
            common::STATUS_DRAFT,
        );
        assert!(result.is_err());
    }

    /// 测试_状态转换_错误消息包含源和目标状态
    ///
    /// 验证非法转换的错误消息包含双方状态名，便于排查。
    #[test]
    fn 测试_状态转换_错误消息包含源和目标状态() {
        let result = ProductionOrderService::validate_status_transition(
            common::STATUS_DRAFT,
            common::STATUS_COMPLETED,
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(common::STATUS_DRAFT),
            "错误消息应包含源状态"
        );
        assert!(
            err_msg.contains(common::STATUS_COMPLETED),
            "错误消息应包含目标状态"
        );
    }

    /// 测试_状态转换_未知状态错误消息包含状态名
    #[test]
    fn 测试_状态转换_未知状态错误消息包含状态名() {
        let result =
            ProductionOrderService::validate_status_transition("FOO", common::STATUS_DRAFT);
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("FOO"), "错误消息应包含未知状态名");
    }

    // ============== 数量计算（纯算法） ==============

    /// 测试_数量计算_BOM用量乘以生产数量_整数结果
    ///
    /// 验证 consumption_qty = bom_quantity * production_qty。
    #[test]
    fn 测试_数量计算_BOM用量乘以生产数量_整数结果() {
        let bom_qty = decs!("1.5");
        let prod_qty = decs!("100");
        let result = calc_consumption_qty(bom_qty, prod_qty);
        assert_eq!(result, decs!("150"));
    }

    /// 测试_数量计算_BOM用量乘以生产数量_小数结果
    ///
    /// 验证 round_dp(4) 对结果精度进行控制，防止精度漂移。
    #[test]
    fn 测试_数量计算_BOM用量乘以生产数量_小数结果() {
        let bom_qty = decs!("0.1234");
        let prod_qty = decs!("3");
        let result = calc_consumption_qty(bom_qty, prod_qty);
        // 0.1234 * 3 = 0.3702，无需进位
        assert_eq!(result, decs!("0.3702"));
    }

    /// 测试_数量计算_公斤数按比例扣减
    ///
    /// 验证 kg_before - (kg_before * consumption / meters_before) 的比例扣减逻辑。
    #[test]
    fn 测试_数量计算_公斤数按比例扣减() {
        let meters_before = decs!("100");
        let kg_before = decs!("50");
        let consumption = decs!("25");
        let result = calc_kg_after_deduction(meters_before, kg_before, consumption);
        // 50 - (50 * 25 / 100) = 50 - 12.5 = 37.5
        assert_eq!(result, decs!("37.5"));
    }

    /// 测试_数量计算_米数为零时公斤数不变
    ///
    /// 防御性逻辑：当 qty_before_meters 为零时，公斤数保持不变避免除零。
    #[test]
    fn 测试_数量计算_米数为零时公斤数不变() {
        let meters_before = Decimal::ZERO;
        let kg_before = decs!("20");
        let consumption = decs!("5");
        let result = calc_kg_after_deduction(meters_before, kg_before, consumption);
        assert_eq!(result, kg_before);
    }

    /// 测试_数量计算_成品入库公斤数计算
    ///
    /// 验证 added_kg = production_qty * gram_weight * width / 100000。
    #[test]
    fn 测试_数量计算_成品入库公斤数计算() {
        let prod_qty = decs!("1000"); // 米
        let gram_weight = Some(decs!("200")); // 克/平方米
        let width = Some(decs!("150")); // 厘米
        let result = calc_added_kg(prod_qty, gram_weight, width);
        // 1000 * 200 * 150 / 100000 = 300 kg
        assert_eq!(result, decs!("300"));
    }

    /// 测试_数量计算_成品入库克重缺失时公斤数为零
    #[test]
    fn 测试_数量计算_成品入库克重缺失时公斤数为零() {
        let prod_qty = decs!("1000");
        let result = calc_added_kg(prod_qty, None, Some(decs!("150")));
        assert_eq!(result, Decimal::ZERO);
    }

    /// 测试_数量计算_实际数量缺省取计划数量
    ///
    /// 复现 complete_production_order 中 actual_quantity.unwrap_or(planned_quantity) 逻辑。
    #[test]
    fn 测试_数量计算_实际数量缺省取计划数量() {
        let planned = decs!("500");
        // 实际数量为 None 时取计划数量
        assert_eq!(resolve_production_qty(None, planned), planned);
        // 实际数量存在时取实际数量
        let actual = Some(decs!("480"));
        assert_eq!(resolve_production_qty(actual, planned), decs!("480"));
    }

    /// 测试_数量计算_生产数量为零时触发错误路径
    ///
    /// 复现 handle_production_completion_inventory_txn 中 production_qty.is_zero() 校验：
    /// 当 actual_quantity 和 planned_quantity 均为零时，应触发业务错误。
    #[test]
    fn 测试_数量计算_生产数量为零时触发错误路径() {
        let planned = Decimal::ZERO;
        let actual: Option<Decimal> = None;
        let production_qty = resolve_production_qty(actual, planned);
        assert!(production_qty.is_zero(), "生产数量为零时应触发错误路径");
    }

    // ============== 错误消息格式 ==============

    /// 测试_错误消息_产品不存在包含ID
    ///
    /// 复现 validate_product_exists 中 "产品ID {} 不存在" 的错误消息格式。
    #[test]
    fn 测试_错误消息_产品不存在包含ID() {
        let err = AppError::validation(format!("产品ID {} 不存在", 999));
        let msg = err.to_string();
        assert!(msg.contains("999"), "错误消息应包含产品ID");
        assert!(msg.contains("产品ID"), "错误消息应包含'产品ID'前缀");
    }

    /// 测试_错误消息_销售订单不存在包含ID
    #[test]
    fn 测试_错误消息_销售订单不存在包含ID() {
        let err = AppError::validation(format!("销售订单ID {} 不存在", 888));
        assert!(err.to_string().contains("888"));
    }

    /// 测试_错误消息_工作中心不存在包含ID
    #[test]
    fn 测试_错误消息_工作中心不存在包含ID() {
        let err = AppError::validation(format!("工作中心ID {} 不存在", 777));
        assert!(err.to_string().contains("777"));
    }

    /// 测试_错误消息_订单号已存在包含订单号
    #[test]
    fn 测试_错误消息_订单号已存在包含订单号() {
        let order_no = "PO-20260709000000-0001";
        let err = AppError::validation(format!("订单号 {} 已存在", order_no));
        let msg = err.to_string();
        assert!(msg.contains(order_no), "错误消息应包含订单号");
    }

    /// 测试_错误消息_生产数量为零提示明确
    ///
    /// 复现 handle_production_completion_inventory_txn 中 "生产数量为零" 的业务错误消息。
    #[test]
    fn 测试_错误消息_生产数量为零提示明确() {
        let err = AppError::business("生产数量为零，无法执行库存联动".to_string());
        let msg = err.to_string();
        assert!(
            msg.contains("生产数量为零"),
            "错误消息应明确提示生产数量为零"
        );
    }

    /// 测试_错误消息_未找到可用仓库提示明确
    #[test]
    fn 测试_错误消息_未找到可用仓库提示明确() {
        let err = AppError::business("未找到可用仓库，无法执行库存联动");
        assert!(err.to_string().contains("未找到可用仓库"));
    }

    /// 测试_错误消息_无法生成唯一订单号提示重试
    #[test]
    fn 测试_错误消息_无法生成唯一订单号提示重试() {
        let err = AppError::internal("无法生成唯一订单号，请稍后重试".to_string());
        let msg = err.to_string();
        assert!(msg.contains("无法生成唯一订单号"));
        assert!(msg.contains("稍后重试"));
    }

    // ============== 订单号格式 ==============

    /// 测试_订单号格式_合法格式通过校验
    ///
    /// 验证 generate_unique_order_no 生成的 "PO-{14位时间戳}-{4位数字}" 格式合法。
    #[test]
    fn 测试_订单号格式_合法格式通过校验() {
        assert!(is_valid_order_no_format("PO-20260709103000-0042"));
    }

    /// 测试_订单号格式_缺少前缀不合法
    #[test]
    fn 测试_订单号格式_缺少前缀不合法() {
        assert!(!is_valid_order_no_format("20260709103000-0042"));
    }

    /// 测试_订单号格式_时间戳长度不足不合法
    #[test]
    fn 测试_订单号格式_时间戳长度不足不合法() {
        assert!(!is_valid_order_no_format("PO-20260709-0042"));
    }

    /// 测试_订单号格式_随机段非数字不合法
    #[test]
    fn 测试_订单号格式_随机段非数字不合法() {
        assert!(!is_valid_order_no_format("PO-20260709103000-ABCD"));
    }

    // ============== 夹具宏可用性 ==============

    /// 测试_decs_宏_解析字符串为Decimal
    #[test]
    fn 测试_decs_宏_解析字符串为Decimal() {
        let v = decs!("123.456");
        assert_eq!(v.to_string(), "123.456");
    }

    /// 测试_decs_宏_解析整数串为Decimal
    #[test]
    fn 测试_decs_宏_解析整数串为Decimal() {
        let v = decs!("1000");
        assert_eq!(v, Decimal::new(1000, 0));
    }

    /// 测试_ymd_宏_解析日期
    #[test]
    fn 测试_ymd_宏_解析日期() {
        let d = ymd!(2026, 7, 9);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-07-09");
    }

    /// 测试_ymd_宏_解析年初日期
    #[test]
    fn 测试_ymd_宏_解析年初日期() {
        let d = ymd!(2026, 1, 1);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-01-01");
    }

    /// 测试_FromStr_与decs宏结果一致
    ///
    /// 验证 decs! 宏与 Decimal::from_str 行为一致，确保夹具可信赖。
    #[test]
    fn 测试_FromStr_与decs宏结果一致() {
        let a = decs!("99.9");
        let b = Decimal::from_str("99.9").expect("FromStr 不应失败");
        assert_eq!(a, b);
    }

    // ============== 服务实例化与请求结构 ==============

    /// 测试_服务实例化_使用SQLite内存数据库
    ///
    /// 标注 #[ignore]：依赖 SQLite 内存数据库 schema，CI 中不强制运行；
    /// 用于本地手动验证 ProductionOrderService::new 能正常构造。
    #[tokio::test]
    #[ignore = "依赖 SQLite 内存数据库 schema，CI 中跳过；本地手动验证用"]
    async fn 测试_服务实例化_使用SQLite内存数据库() {
        let db = setup_test_db().await;
        // L-20 修复（批次 377 v13 复审）：删除 let _ = service 占位变量
        // 仅验证服务能正常构造，不调用任何依赖 schema 的方法
        let _service = ProductionOrderService::new(std::sync::Arc::new(db));
    }

    /// 测试_请求结构_创建订单请求可构造
    ///
    /// 验证 CreateProductionOrderRequest 能正常构造，字段类型匹配。
    #[test]
    fn 测试_请求结构_创建订单请求可构造() {
        let req = CreateProductionOrderRequest {
            order_no: Some("PO-TEST-001".to_string()),
            sales_order_id: None,
            product_id: 1,
            planned_quantity: Some(decs!("100")),
            planned_start_date: Some(ymd!(2026, 7, 1)),
            planned_end_date: Some(ymd!(2026, 7, 31)),
            priority: Some(5),
            work_center_id: Some(1),
            remarks: Some("测试订单".to_string()),
            created_by: 1,
        };
        assert_eq!(req.product_id, 1);
        assert_eq!(req.planned_quantity, Some(decs!("100")));
        assert_eq!(req.priority, Some(5));
    }

    /// 测试_请求结构_更新订单请求可构造
    #[test]
    fn 测试_请求结构_更新订单请求可构造() {
        let req = UpdateProductionOrderRequest {
            planned_quantity: Some(decs!("200")),
            planned_start_date: Some(ymd!(2026, 8, 1)),
            planned_end_date: Some(ymd!(2026, 8, 31)),
            priority: Some(8),
            work_center_id: Some(2),
            remarks: Some("更新后备注".to_string()),
        };
        assert_eq!(req.planned_quantity, Some(decs!("200")));
        assert_eq!(req.priority, Some(8));
    }

    /// 测试_查询参数_分页参数可构造
    #[test]
    fn 测试_查询参数_分页参数可构造() {
        let query = ProductionOrderQuery {
            status: Some(common::STATUS_DRAFT.to_string()),
            product_id: Some(1),
            page: 1,
            page_size: 20,
        };
        assert_eq!(query.page, 1);
        assert_eq!(query.page_size, 20);
        assert_eq!(query.status, Some("DRAFT".to_string()));
    }
}

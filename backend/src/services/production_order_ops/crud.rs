//! 生产订单-CRUD 子模块（production_order_ops/crud）
//!
//! 批次 488 D10-2 拆分：从原 `production_order_service.rs` L92-624 迁移。
//! 包含 14 个 CRUD 与状态校验方法：
//! - validate_product_exists / validate_sales_order_exists / validate_work_center_exists（私有 &self）
//! - generate_unique_order_no / generate_rework_order_no（私有 &self）
//! - validate_status_transition（`pub(crate)` associated function，测试 + approval 子模块跨 impl 块调用）
//! - create / create_rework_order（公开 API）
//! - get_by_id / get_order_logs / list（公开 API）
//! - update / delete / update_status（公开 API）
//!
//! 业务规则：
//! - 创建订单后触发 MRP 物料需求计算（失败 warn 不阻塞）
//! - 返工订单使用 RW- 前缀，不触发 MRP
//! - 状态转换校验基于状态机白名单（validate_status_transition）
//! - COMPLETED 状态走 complete_production_order 专用路径（completion 子模块）
//! - 排产状态变更走 check_capacity_for_scheduling 产能校验（completion 子模块）
//! - delete 软删除（状态改为 CANCELLED），走 update_with_audit 保留审计

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::models::production_order::{
    self, ActiveModel, Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

use crate::models::bom::{Column as BomColumn, Entity as BomEntity};
use crate::models::product::Entity as ProductEntity;
use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::work_center::Entity as WorkCenterEntity;

use super::types::{CreateProductionOrderRequest, ProductionOrderQuery, UpdateProductionOrderRequest};
use crate::services::production_order_service::ProductionOrderService;

impl ProductionOrderService {
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
    ///
    /// `pub(crate)` 可见性：测试模块（facade）与 approval 子模块跨 impl 块调用。
    pub(crate) fn validate_status_transition(
        current_status: &str,
        new_status: &str,
    ) -> Result<(), AppError> {
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
        self.check_capacity_for_scheduling(&model, id, &status)
            .await?;
        let audit_user_id = model.created_by;
        let updated = Self::apply_status_update_with_audit(&txn, model, status, audit_user_id)
            .await?;
        txn.commit().await?;
        Ok(updated)
    }
}

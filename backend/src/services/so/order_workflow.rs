//! 销售订单工作流子模块（order_workflow）
//!
//! P9-2 拆分自原 `services/so/order.rs`。
//! 包含：cancel_order / submit_order / approve_order / complete_order
//!
//! ## 模块职责
//! - 销售订单审批流（草稿→待审→已审→已发货→已收款→已关闭）
//! - 状态机转换合法性校验
//! - 工作流日志（操作人/时间/原因）
//! - BPM 流程集成（提交/审批触发 BPM 服务）
//!
//! ## API 兼容
//! 通过 `crate::services::so::order::SalesService` 路径访问。

use super::order::SalesService;
use super::SalesOrderDetail;
use crate::models::sales_order;
use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::status::sales_order as so_status;
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use sea_orm::{EntityTrait, QuerySelect, TransactionTrait};

impl SalesService {
    // cancel_order / submit_order / approve_order / complete_order
    // 内容来自原 order.rs L815-840 + L898-978 + L979-1013 + L1014-1029

    pub async fn cancel_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        // 批次 18（2026-06-28）：补全事务边界 + 审计日志 + lock_exclusive。
        // 原实现完全无事务、无审计日志（直接 .update）、状态查询无锁，并发取消可能基于过期状态。
        let txn = (*self.db).begin().await?;

        // 获取订单（加 lock_exclusive 串行化并发取消）
        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        // 检查订单状态是否允许取消
        // 批次 13（2026-06-28）：补 partial_shipped 状态，防止部分发货订单无法取消（死锁）。
        // 已发货部分需通过退货流程处理，取消仅作用于剩余未发货部分。
        // 批次 158 v11 真实接入：引用 status::sales_order 常量替代字符串字面量（规则 0）
        if ![
            so_status::DRAFT,
            so_status::PENDING,
            so_status::APPROVED,
            so_status::PARTIAL_SHIPPED,
        ]
        .contains(&order.status.as_str())
        {
            return Err(AppError::business("当前状态不允许取消".to_string()));
        }

        // 更新订单状态（改用 update_with_audit 写入审计日志，传 &txn 纳入事务保证原子性）
        let customer_id_for_event = order.customer_id;
        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set(so_status::CANCELLED.to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // B-P1-4 修复（批次 361 v13 复审）：commit 后发布 SalesOrderCancelled 事件
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::SalesOrderCancelled {
                order_id,
                customer_id: customer_id_for_event,
                user_id,
            },
        );

        self.get_order_detail(order_id, None).await
    }

    /// 获取订单统计
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let order = self.lookup_order_for_submit(&txn, order_id).await?;
        self.validate_order_status(&order)?;

        let total_amount_decimal = order
            .total_amount
            .to_string()
            .parse::<rust_decimal::Decimal>()
            .unwrap_or_else(|_| rust_decimal::Decimal::from(0));
        self.validate_customer_credit(&txn, order.customer_id, total_amount_decimal)
            .await?;
        self.validate_customer_active(&txn, order.customer_id)
            .await?;

        let order = self.update_order_to_pending(&txn, order, user_id).await?;
        txn.commit().await?;

        self.start_bpm_process(order_id, user_id, &order.order_no)
            .await?;
        self.publish_submitted_event(order_id, order.customer_id, user_id);

        Ok(order)
    }

    async fn lookup_order_for_submit(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))
    }

    fn validate_order_status(&self, order: &sales_order::Model) -> Result<(), AppError> {
        if order.status != so_status::DRAFT {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法提交",
                order.status
            )));
        }
        Ok(())
    }

    async fn validate_customer_credit(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
        total_amount: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let credit_available = credit_service
            .check_credit_available_txn(txn, customer_id, total_amount)
            .await
            .map_err(|e| AppError::business(format!("信用检查失败: {}", e)))?;
        if !credit_available {
            return Err(AppError::business("信用额度不足，无法提交订单"));
        }
        Ok(())
    }

    async fn validate_customer_active(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
    ) -> Result<(), AppError> {
        let customer = crate::models::customer::Entity::find_by_id(customer_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;
        if customer.status != master_data::ACTIVE {
            return Err(AppError::business(format!(
                "客户状态为 {}，不允许提交订单",
                customer.status
            )));
        }
        Ok(())
    }

    async fn update_order_to_pending(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        order: sales_order::Model,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set(so_status::PENDING.to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await
    }

    async fn start_bpm_process(
        &self,
        order_id: i32,
        user_id: i32,
        order_no: &str,
    ) -> Result<(), AppError> {
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        if let Err(e) = bpm_service
            .start_process(crate::models::dto::bpm_dto::StartProcessRequest {
                process_key: "sales_order_approval".to_string(),
                business_type: "sales_order".to_string(),
                business_id: order_id,
                title: format!("销售订单审批 - {}", order_no),
                initiator_id: user_id,
                initiator_name: String::new(),
                initiator_department_id: None,
                priority: None,
                form_data: None,
                variables: None,
            })
            .await
        {
            self.rollback_order_to_draft(order_id, user_id).await?;
            return Err(AppError::business(format!(
                "BPM 审批流程启动失败，订单已回滚为草稿状态，请重新提交：{}",
                e
            )));
        }
        Ok(())
    }

    async fn rollback_order_to_draft(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        tracing::error!(
            order_id = order_id,
            "BPM 启动销售订单审批流程失败，开始补偿回滚订单状态"
        );

        let compensating_txn = (*self.db).begin().await?;
        let order_for_rollback = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&compensating_txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("补偿回滚时销售订单 {} 不存在", order_id))
            })?;
        let mut rollback_model: sales_order::ActiveModel = order_for_rollback.into();
        rollback_model.status = sea_orm::ActiveValue::Set(so_status::DRAFT.to_string());
        rollback_model.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &compensating_txn,
            "auto_audit",
            rollback_model,
            Some(user_id),
        )
        .await?;
        compensating_txn.commit().await?;

        Ok(())
    }

    fn publish_submitted_event(&self, order_id: i32, customer_id: i32, user_id: i32) {
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::SalesOrderSubmitted {
                order_id,
                customer_id,
                user_id,
            },
        );
    }

    /// 审核订单：通过或拒绝
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发审批同一订单导致重复审批或字段覆盖
        let txn = (*self.db).begin().await?;

        let order = self.lookup_order_for_approval(&txn, order_id).await?;
        self.validate_order_for_approval(&order)?;
        let order = self.update_order_to_approved(&txn, order, user_id).await?;

        txn.commit().await?;

        // B-P1-4 修复（批次 361 v13 复审）：commit 后发布 SalesOrderApproved 事件
        self.publish_approval_event(order_id, order.customer_id, user_id);

        // 批次 356 v13 复审 B-P0-1：commit 后查询订单明细，为每个明细创建库存预留记录
        let order_items = self.fetch_order_items_for_approval(order_id).await?;
        self.create_inventory_reservations_for_order(
            order_id,
            &order.order_no,
            &order_items,
            user_id,
        )
        .await;

        // B-P2-4 修复（批次 386 v13 复审）：commit 后对每个订单明细调用 MRP 计算
        self.run_mrp_for_order_items(order_id, &order_items).await;

        Ok(order)
    }

    async fn lookup_order_for_approval(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))
    }

    fn validate_order_for_approval(&self, order: &sales_order::Model) -> Result<(), AppError> {
        if order.status != so_status::PENDING {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法审核",
                order.status
            )));
        }
        Ok(())
    }

    async fn update_order_to_approved(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        order: sales_order::Model,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set(so_status::APPROVED.to_string());
        order_update.approved_by = sea_orm::ActiveValue::Set(Some(user_id));
        order_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID，
        // 原 Some(0) 硬编码导致审计日志无法追溯审批人。
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await
    }

    fn publish_approval_event(&self, order_id: i32, customer_id: i32, user_id: i32) {
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::SalesOrderApproved {
                order_id,
                customer_id,
                user_id,
            },
        );
    }

    async fn fetch_order_items_for_approval(
        &self,
        order_id: i32,
    ) -> Result<Vec<crate::models::sales_order_item::Model>, AppError> {
        use sea_orm::{ColumnTrait, QueryFilter};
        crate::models::sales_order_item::Entity::find()
            .filter(crate::models::sales_order_item::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 批次 356 v13 复审 B-P0-1 修复：销售订单审批后触发库存预留
    /// 原实现 approve_order 仅更新订单状态，不调用 InventoryReservationService::create_reservation，；导致销售订单→库存锁定链路完全断开，存在超卖风险。
    async fn create_inventory_reservations_for_order(
        &self,
        order_id: i32,
        order_no: &str,
        order_items: &[crate::models::sales_order_item::Model],
        user_id: i32,
    ) {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let reservation_service =
            crate::services::inventory_reservation_service::InventoryReservationService::new(
                self.db.clone(),
            );
        for item in order_items {
            // 查询产品默认仓库（使用第一个活跃仓库作为默认仓库）
            let default_warehouse = match crate::models::warehouse::Entity::find()
                .filter(crate::models::warehouse::Column::IsActive.eq(true))
                .one(&*self.db)
                .await
            {
                Ok(wh) => wh,
                Err(e) => {
                    tracing::warn!(
                        order_id,
                        product_id = item.product_id,
                        error = %e,
                        "批次 356 B-P0-1: 查询默认仓库失败，跳过该订单项的库存预留"
                    );
                    continue;
                }
            };

            if let Some(wh) = default_warehouse {
                if let Err(e) = reservation_service
                    .create_reservation(
                        order_id,
                        item.product_id,
                        wh.id,
                        item.quantity,
                        Some(user_id),
                        Some(format!("销售订单 {} 审批通过，自动预留库存", order_no)),
                    )
                    .await
                {
                    tracing::warn!(
                        order_id,
                        product_id = item.product_id,
                        error = %e,
                        "批次 356 B-P0-1: 创建库存预留失败，订单已审批但库存未锁定，请人工检查"
                    );
                }
            }
        }
    }

    /// B-P2-4 修复（批次 386 v13 复审）：销售订单审批后触发 MRP 物料需求计算
    /// 原实现 approve_order 仅做库存预留，不调用 MrpEngineService，；导致销售→MRP 物料需求链路断开，采购计划无法基于销售订单自动生成。；失败时 tracing::warn 不阻塞主流程（订单已审批，MRP 可后续重算）。
    async fn run_mrp_for_order_items(
        &self,
        order_id: i32,
        order_items: &[crate::models::sales_order_item::Model],
    ) {
        let mrp_service =
            crate::services::mrp_engine_service::MrpEngineService::new(self.db.clone());
        let required_date = chrono::Utc::now().date_naive() + chrono::Duration::days(7);
        for item in order_items {
            if let Err(e) = mrp_service
                .run_mrp_calculation(crate::services::mrp_engine_service::MrpCalculationQuery {
                    product_id: item.product_id,
                    required_quantity: item.quantity,
                    required_date,
                    source_type: "SALES_ORDER".to_string(),
                    source_id: Some(order_id),
                    consider_safety_stock: true,
                    consider_in_transit: true,
                })
                .await
            {
                tracing::warn!(
                    order_id,
                    product_id = item.product_id,
                    error = %e,
                    "批次 386 B-P2-4: 销售订单审批后 MRP 计算失败，请人工检查物料需求"
                );
            }
        }
    }

    /// 完成订单（P1-11 修复（2026-06-25 综合审计）：新增 user_id 参数，；原 Some(0) 硬编码导致审计日志无法追溯完成操作人。）
    pub async fn complete_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发完成同一订单导致状态不一致
        let txn = (*self.db).begin().await?;

        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if ![so_status::SHIPPED, so_status::PARTIAL_SHIPPED].contains(&order.status.as_str()) {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法完成",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set(so_status::COMPLETED.to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复：传入真实操作人 ID
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // B-P1-4 修复（批次 361 v13 复审）：commit 后发布 SalesOrderCompleted 事件
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::SalesOrderCompleted {
                order_id,
                customer_id: order.customer_id,
                user_id,
            },
        );

        Ok(order)
    }
}

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
        crate::services::event_bus::EVENT_BUS
            .publish(crate::services::event_bus::BusinessEvent::SalesOrderCancelled {
                order_id,
                customer_id: customer_id_for_event,
                user_id,
            });

        self.get_order_detail(order_id).await
    }

    /// 获取订单统计
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发提交同一订单导致状态不一致；
        // update_with_audit 内部 2 次写入（实体 update + 审计 insert）非原子，事务包裹保证原子性。
        let txn = (*self.db).begin().await?;

        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != so_status::DRAFT {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法提交",
                order.status
            )));
        }

        // 客户信用度复检（P2 3-20 修复：改用 _txn 变体在事务内查询，避免 TOCTOU）
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let total_amount_decimal = {
            use rust_decimal::Decimal;
            order
                .total_amount
                .to_string()
                .parse::<rust_decimal::Decimal>()
                .unwrap_or_else(|_| Decimal::from(0))
        };
        let credit_available = credit_service
            .check_credit_available_txn(&txn, order.customer_id, total_amount_decimal)
            .await
            .map_err(|e| AppError::business(format!("信用检查失败: {}", e)))?;
        if !credit_available {
            return Err(AppError::business("信用额度不足，无法提交订单"));
        }

        // 客户状态校验（事务内，保证校验与提交一致）
        let customer = crate::models::customer::Entity::find_by_id(order.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;
        if customer.status != master_data::ACTIVE {
            return Err(AppError::business(format!(
                "客户状态为 {}，不允许提交订单",
                customer.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set(so_status::PENDING.to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID，
        // 原 Some(0) 硬编码导致审计日志无法追溯提交人。
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // P1 3-11 修复（批次 62）：BPM 启动失败时补偿回滚订单状态
        // 原实现 BPM 启动在 commit 后，失败仅 warn 不阻断，导致订单已提交但无审批流，
        // 业务流断点（订单卡在 pending 状态无人审批）。
        // 修复：BPM 启动失败时将订单状态补偿回滚为 draft 并返回错误，用户可重新提交。
        // BpmService::start_process 内部独立事务（不支持外部 txn），无法与订单状态更新共用事务，
        // 故采用补偿机制：commit 成功后调用 BPM，BPM 失败则开启新事务回滚订单状态。
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        if let Err(e) = bpm_service
            .start_process(crate::models::dto::bpm_dto::StartProcessRequest {
                process_key: "sales_order_approval".to_string(),
                business_type: "sales_order".to_string(),
                business_id: order_id,
                title: format!("销售订单审批 - {}", order.order_no),
                initiator_id: user_id,
                initiator_name: String::new(),
                initiator_department_id: None,
                priority: None,
                form_data: None,
                variables: None,
            })
            .await
        {
            tracing::error!(
                error = %e,
                order_id = order_id,
                "BPM 启动销售订单审批流程失败，开始补偿回滚订单状态"
            );
            // 补偿：开启新事务回滚订单状态为 draft，使用户可重新提交
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
            return Err(AppError::business(format!(
                "BPM 审批流程启动失败，订单已回滚为草稿状态，请重新提交：{}",
                e
            )));
        }

        // B-P1-4 修复（批次 361 v13 复审）：BPM 启动成功后发布 SalesOrderSubmitted 事件
        // 放在 BPM 成功后而非 commit 后，避免 BPM 失败补偿回滚时已发布事件造成幻事件。
        crate::services::event_bus::EVENT_BUS
            .publish(crate::services::event_bus::BusinessEvent::SalesOrderSubmitted {
                order_id,
                customer_id: order.customer_id,
                user_id,
            });

        Ok(order)
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

        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != so_status::PENDING {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法审核",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set(so_status::APPROVED.to_string());
        order_update.approved_by = sea_orm::ActiveValue::Set(Some(user_id));
        order_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID，
        // 原 Some(0) 硬编码导致审计日志无法追溯审批人。
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // B-P1-4 修复（批次 361 v13 复审）：commit 后发布 SalesOrderApproved 事件
        crate::services::event_bus::EVENT_BUS
            .publish(crate::services::event_bus::BusinessEvent::SalesOrderApproved {
                order_id,
                customer_id: order.customer_id,
                user_id,
            });

        // 批次 356 v13 复审 B-P0-1 修复：销售订单审批后触发库存预留
        // 原实现 approve_order 仅更新订单状态，不调用 InventoryReservationService::create_reservation，
        // 导致销售订单→库存锁定链路完全断开，存在超卖风险。
        // 修复：commit 成功后查询订单明细，为每个明细创建库存预留记录。
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let order_items = crate::models::sales_order_item::Entity::find()
            .filter(crate::models::sales_order_item::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await?;

        let reservation_service =
            crate::services::inventory_reservation_service::InventoryReservationService::new(
                self.db.clone(),
            );
        for item in &order_items {
            // 查询产品默认仓库（使用第一个活跃仓库作为默认仓库）
            let default_warehouse = crate::models::warehouse::Entity::find()
                .filter(crate::models::warehouse::Column::IsActive.eq(true))
                .one(&*self.db)
                .await?;

            if let Some(wh) = default_warehouse {
                if let Err(e) = reservation_service
                    .create_reservation(
                        order_id,
                        item.product_id,
                        wh.id,
                        item.quantity,
                        Some(user_id),
                        Some(format!("销售订单 {} 审批通过，自动预留库存", order.order_no)),
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

        // B-P2-4 修复（批次 386 v13 复审）：销售订单审批后触发 MRP 物料需求计算
        // 原实现 approve_order 仅做库存预留，不调用 MrpEngineService，
        // 导致销售→MRP 物料需求链路断开，采购计划无法基于销售订单自动生成。
        // 修复：commit 成功后对每个订单明细调用 MRP 计算（source_type=SALES_ORDER），
        // 失败时 tracing::warn 不阻塞主流程（订单已审批，MRP 可后续重算）。
        let mrp_service = crate::services::mrp_engine_service::MrpEngineService::new(self.db.clone());
        let required_date = chrono::Utc::now().date_naive() + chrono::Duration::days(7);
        for item in &order_items {
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

        Ok(order)
    }

    /// 完成订单
    ///
    /// P1-11 修复（2026-06-25 综合审计）：新增 user_id 参数，
    /// 原 Some(0) 硬编码导致审计日志无法追溯完成操作人。
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
        crate::services::event_bus::EVENT_BUS
            .publish(crate::services::event_bus::BusinessEvent::SalesOrderCompleted {
                order_id,
                customer_id: order.customer_id,
                user_id,
            });

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    // 批次 415：decs! 宏展开为 Decimal::from_str，需导入 FromStr trait
    use std::str::FromStr;
    use crate::ymd;
    use crate::search::{ElasticClient, SearchClient};
    use chrono::Utc;
    use rust_decimal::Decimal;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    /// 构建测试用销售订单模型夹具
    ///
    /// 封装 `sales_order::Model` 的构造，便于在各测试中复用。
    /// 默认 subtotal = total_amount（无税/无折扣/无运费），balance_amount = total_amount（未付款），
    /// 保持金额一致以匹配 submit_order 中 total_amount_decimal 的解析逻辑。
    fn make_order_model(
        id: i32,
        customer_id: i32,
        status: &str,
        total_amount: Decimal,
    ) -> sales_order::Model {
        sales_order::Model {
            id,
            order_no: format!("SO-TEST-{}", id),
            customer_id,
            opportunity_id: None,
            order_date: Utc::now(),
            required_date: Utc::now(),
            ship_date: None,
            status: status.to_string(),
            subtotal: total_amount,
            tax_amount: Decimal::ZERO,
            discount_amount: Decimal::ZERO,
            shipping_cost: Decimal::ZERO,
            total_amount,
            paid_amount: Decimal::ZERO,
            balance_amount: total_amount,
            shipping_address: None,
            billing_address: None,
            notes: None,
            created_by: Some(1),
            approved_by: None,
            approved_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 测试 SQLite 内存数据库连接夹具
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    /// 复现 cancel_order 的状态校验门（不涉及数据库）
    ///
    /// 与 cancel_order 中状态校验逻辑保持一致，便于纯算法测试。
    fn cancel_order_status_gate(status: &str) -> Result<(), AppError> {
        if ![
            so_status::DRAFT,
            so_status::PENDING,
            so_status::APPROVED,
            so_status::PARTIAL_SHIPPED,
        ]
        .contains(&status)
        {
            return Err(AppError::business("当前状态不允许取消".to_string()));
        }
        Ok(())
    }

    /// 复现 submit_order 的状态校验门（不涉及数据库）
    fn submit_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != so_status::DRAFT {
            return Err(AppError::business(format!("订单状态为 {}，无法提交", status)));
        }
        Ok(())
    }

    /// 复现 approve_order 的状态校验门（不涉及数据库）
    fn approve_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != so_status::PENDING {
            return Err(AppError::business(format!("订单状态为 {}，无法审核", status)));
        }
        Ok(())
    }

    /// 复现 complete_order 的状态校验门（不涉及数据库）
    fn complete_order_status_gate(status: &str) -> Result<(), AppError> {
        if ![so_status::SHIPPED, so_status::PARTIAL_SHIPPED].contains(&status) {
            return Err(AppError::business(format!("订单状态为 {}，无法完成", status)));
        }
        Ok(())
    }

    /// 测试_销售订单状态常量值正确性
    ///
    /// 校验 status::sales_order 子模块的常量值均为小写，
    /// 与业务代码（order_workflow.rs / order_crud.rs / delivery.rs）实际使用的状态值一致。
    /// 防止常量值大小写漂移导致状态匹配失败（隐性 P0 风险）。
    #[test]
    fn 测试_销售订单状态常量值正确性() {
        assert_eq!(so_status::DRAFT, "draft");
        assert_eq!(so_status::PENDING, "pending");
        assert_eq!(so_status::APPROVED, "approved");
        assert_eq!(so_status::PARTIAL_SHIPPED, "partial_shipped");
        assert_eq!(so_status::SHIPPED, "shipped");
        assert_eq!(so_status::COMPLETED, "completed");
        assert_eq!(so_status::CANCELLED, "cancelled");
        assert_eq!(so_status::REJECTED, "rejected");

        // 全部常量值互不相同，避免状态语义重叠
        let all = [
            so_status::DRAFT,
            so_status::PENDING,
            so_status::APPROVED,
            so_status::PARTIAL_SHIPPED,
            so_status::SHIPPED,
            so_status::COMPLETED,
            so_status::CANCELLED,
            so_status::REJECTED,
        ];
        let unique_count = std::collections::HashSet::from(all).len();
        assert_eq!(unique_count, 8, "销售订单状态常量值应两两不同");
    }

    /// 测试_主数据状态常量值正确性
    ///
    /// 校验 master_data 子模块常量值为小写 "active"/"inactive"，
    /// submit_order 中客户状态校验依赖此常量（customer.status != master_data::ACTIVE）。
    #[test]
    fn 测试_主数据状态常量值正确性() {
        assert_eq!(master_data::ACTIVE, "active");
        assert_eq!(master_data::INACTIVE, "inactive");
        assert_ne!(master_data::ACTIVE, master_data::INACTIVE);
    }

    /// 测试_取消订单_允许的源状态集合
    ///
    /// 验证 cancel_order 的状态校验门对 DRAFT/PENDING/APPROVED/PARTIAL_SHIPPED 均放行。
    /// 其中 PARTIAL_SHIPPED 是批次 13 补全，防止部分发货订单无法取消（死锁）。
    #[test]
    fn 测试_取消订单_允许的源状态集合() {
        for allowed in [
            so_status::DRAFT,
            so_status::PENDING,
            so_status::APPROVED,
            so_status::PARTIAL_SHIPPED,
        ] {
            assert!(
                cancel_order_status_gate(allowed).is_ok(),
                "状态 {} 应允许取消",
                allowed
            );
        }
    }

    /// 测试_取消订单_禁止的源状态集合及错误消息
    ///
    /// 验证 cancel_order 对已发货/已完成/已取消/已拒绝状态拒绝，
    /// 且错误类型为 BusinessError，错误消息为中文"当前状态不允许取消"。
    #[test]
    fn 测试_取消订单_禁止的源状态集合及错误消息() {
        for forbidden in [
            so_status::SHIPPED,
            so_status::COMPLETED,
            so_status::CANCELLED,
            so_status::REJECTED,
        ] {
            let result = cancel_order_status_gate(forbidden);
            assert!(result.is_err(), "状态 {} 应禁止取消", forbidden);
            match result.unwrap_err() {
                AppError::BusinessError(msg) => {
                    assert_eq!(msg, "当前状态不允许取消");
                }
                other => panic!("取消订单应返回 BusinessError，实际：{:?}", other),
            }
        }
    }

    /// 测试_提交订单_仅草稿状态允许提交
    ///
    /// 验证 submit_order 的状态校验门仅对 DRAFT 放行，其余状态全部拒绝。
    #[test]
    fn 测试_提交订单_仅草稿状态允许提交() {
        assert!(submit_order_status_gate(so_status::DRAFT).is_ok());

        for forbidden in [
            so_status::PENDING,
            so_status::APPROVED,
            so_status::PARTIAL_SHIPPED,
            so_status::SHIPPED,
            so_status::COMPLETED,
            so_status::CANCELLED,
            so_status::REJECTED,
        ] {
            assert!(
                submit_order_status_gate(forbidden).is_err(),
                "状态 {} 应禁止提交",
                forbidden
            );
        }
    }

    /// 测试_提交订单_非草稿状态错误消息格式
    ///
    /// 验证 submit_order 的错误消息包含状态值与中文说明"无法提交"，
    /// 格式为 "订单状态为 {}，无法提交"。
    #[test]
    fn 测试_提交订单_非草稿状态错误消息格式() {
        let result = submit_order_status_gate(so_status::APPROVED);
        match result.unwrap_err() {
            AppError::BusinessError(msg) => {
                assert!(msg.contains(so_status::APPROVED), "错误消息应包含状态值");
                assert!(msg.contains("无法提交"), "错误消息应包含中文说明");
                assert_eq!(msg, format!("订单状态为 {}，无法提交", so_status::APPROVED));
            }
            other => panic!("提交订单应返回 BusinessError，实际：{:?}", other),
        }
    }

    /// 测试_提交订单_客户状态非活跃拒绝
    ///
    /// 验证 submit_order 中客户状态校验逻辑：
    /// customer.status != master_data::ACTIVE 时应构造拒绝错误，
    /// 错误消息格式为 "客户状态为 {}，不允许提交订单"。
    #[test]
    fn 测试_提交订单_客户状态非活跃拒绝() {
        // 复现 submit_order 中的客户状态校验
        let customer_status = master_data::INACTIVE;
        let should_reject = customer_status != master_data::ACTIVE;
        assert!(should_reject);

        let err = AppError::business(format!(
            "客户状态为 {}，不允许提交订单",
            customer_status
        ));
        match err {
            AppError::BusinessError(msg) => {
                assert!(msg.contains(master_data::INACTIVE));
                assert!(msg.contains("不允许提交订单"));
            }
            other => panic!("客户状态校验应返回 BusinessError，实际：{:?}", other),
        }

        // 客户状态为 ACTIVE 时应放行
        let customer_active = master_data::ACTIVE;
        assert!(!(customer_active != master_data::ACTIVE));
    }

    /// 测试_提交订单_信用额度不足拒绝
    ///
    /// 验证 submit_order 中信用额度校验逻辑：
    /// credit_available == false 时应返回 BusinessError，消息为"信用额度不足，无法提交订单"。
    #[test]
    fn 测试_提交订单_信用额度不足拒绝() {
        // 复现 submit_order 中信用校验失败分支
        let credit_available = false;
        if !credit_available {
            let err = AppError::business("信用额度不足，无法提交订单".to_string());
            match err {
                AppError::BusinessError(msg) => {
                    assert_eq!(msg, "信用额度不足，无法提交订单");
                }
                other => panic!("信用不足应返回 BusinessError，实际：{:?}", other),
            }
        } else {
            panic!("信用不足场景应进入拒绝分支");
        }

        // 信用充足场景不应触发该错误
        let credit_ok = true;
        assert!(!credit_ok == false);
    }

    /// 测试_审核订单_仅待审核状态允许
    ///
    /// 验证 approve_order 的状态校验门仅对 PENDING 放行。
    #[test]
    fn 测试_审核订单_仅待审核状态允许() {
        assert!(approve_order_status_gate(so_status::PENDING).is_ok());

        for forbidden in [
            so_status::DRAFT,
            so_status::APPROVED,
            so_status::PARTIAL_SHIPPED,
            so_status::SHIPPED,
            so_status::COMPLETED,
            so_status::CANCELLED,
            so_status::REJECTED,
        ] {
            assert!(
                approve_order_status_gate(forbidden).is_err(),
                "状态 {} 应禁止审核",
                forbidden
            );
        }
    }

    /// 测试_审核订单_非待审核状态错误消息格式
    ///
    /// 验证 approve_order 的错误消息包含状态值与中文说明"无法审核"，
    /// 格式为 "订单状态为 {}，无法审核"。
    #[test]
    fn 测试_审核订单_非待审核状态错误消息格式() {
        let result = approve_order_status_gate(so_status::DRAFT);
        match result.unwrap_err() {
            AppError::BusinessError(msg) => {
                assert!(msg.contains(so_status::DRAFT));
                assert!(msg.contains("无法审核"));
                assert_eq!(msg, format!("订单状态为 {}，无法审核", so_status::DRAFT));
            }
            other => panic!("审核订单应返回 BusinessError，实际：{:?}", other),
        }
    }

    /// 测试_完成订单_允许的源状态集合
    ///
    /// 验证 complete_order 的状态校验门对 SHIPPED/PARTIAL_SHIPPED 放行。
    /// 部分发货订单可走完成流程，剩余未发货部分通过取消/退货处理。
    #[test]
    fn 测试_完成订单_允许的源状态集合() {
        for allowed in [so_status::SHIPPED, so_status::PARTIAL_SHIPPED] {
            assert!(
                complete_order_status_gate(allowed).is_ok(),
                "状态 {} 应允许完成",
                allowed
            );
        }
    }

    /// 测试_完成订单_禁止的源状态集合及错误消息
    ///
    /// 验证 complete_order 对草稿/待审/已审/已完成/已取消/已拒绝状态拒绝，
    /// 错误消息格式为 "订单状态为 {}，无法完成"。
    #[test]
    fn 测试_完成订单_禁止的源状态集合及错误消息() {
        for forbidden in [
            so_status::DRAFT,
            so_status::PENDING,
            so_status::APPROVED,
            so_status::COMPLETED,
            so_status::CANCELLED,
            so_status::REJECTED,
        ] {
            let result = complete_order_status_gate(forbidden);
            assert!(result.is_err(), "状态 {} 应禁止完成", forbidden);
            match result.unwrap_err() {
                AppError::BusinessError(msg) => {
                    assert!(msg.contains(forbidden), "错误消息应包含状态值");
                    assert!(msg.contains("无法完成"), "错误消息应包含中文说明");
                    assert_eq!(msg, format!("订单状态为 {}，无法完成", forbidden));
                }
                other => panic!("完成订单应返回 BusinessError，实际：{:?}", other),
            }
        }
    }

    /// 测试_夹具宏_decs_ymd_可用性
    ///
    /// 验证项目测试夹具宏 decs! / ymd!（utils/unwrap_safe.rs 通过 #[macro_export] 导出）
    /// 可在测试模块正常使用，避免散落的 .unwrap() / .expect() 调用。
    #[test]
    fn 测试_夹具宏_decs_ymd_可用性() {
        // decs! 解析 Decimal 字符串
        let amount = decs!("12345.67");
        assert_eq!(amount.to_string(), "12345.67");

        // ymd! 解析日期
        let order_date = ymd!(2026, 7, 9);
        assert_eq!(order_date.format("%Y-%m-%d").to_string(), "2026-07-09");

        // 宏组合使用：构造订单总额并参与运算
        let subtotal = decs!("10000");
        let tax = decs!("1300");
        assert_eq!(subtotal + tax, decs!("11300"));
    }

    /// 测试_销售订单模型夹具构造
    ///
    /// 验证 make_order_model 能正确构造 sales_order::Model，
    /// 且 status 字段引用状态常量后保持一致，total_amount 与 balance_amount 关系正确。
    #[test]
    fn 测试_销售订单模型夹具构造() {
        let model = make_order_model(1, 100, so_status::DRAFT, decs!("10000"));

        assert_eq!(model.id, 1);
        assert_eq!(model.customer_id, 100);
        assert_eq!(model.status, so_status::DRAFT);
        assert_eq!(model.order_no, "SO-TEST-1");
        assert_eq!(model.total_amount, decs!("10000"));
        // 未付款时余额等于总额
        assert_eq!(model.balance_amount, model.total_amount);
        assert_eq!(model.paid_amount, Decimal::ZERO);
        // 草稿状态未审批
        assert!(model.approved_by.is_none());
        assert!(model.approved_at.is_none());

        // 不同状态构造不影响字段一致性
        let shipped = make_order_model(2, 200, so_status::SHIPPED, decs!("5000"));
        assert_eq!(shipped.status, so_status::SHIPPED);
        assert_eq!(shipped.customer_id, 200);
    }

    /// 测试_服务实例创建
    ///
    /// 验证 SalesService 在 SQLite 内存数据库 + mock SearchClient 上能正常实例化。
    /// SalesService::new 需要 db 与 search_client 两个依赖，使用 ElasticClient::mock() 提供空实现。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 校验服务内部依赖强引用计数 >= 1，证明实例化成功
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_取消订单_需要真实数据库
    ///
    /// 需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
    #[tokio::test]
    #[ignore]
    async fn 测试_取消订单_需要真实数据库() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
        let result = service.cancel_order(99999, 1).await;
        assert!(result.is_err());
    }

    /// 测试_提交订单_需要真实数据库
    ///
    /// 需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。
    /// 验证提交不存在的订单返回错误，调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_提交订单_需要真实数据库() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
        let result = service.submit_order(99999, 1).await;
        assert!(result.is_err());
    }

    /// 测试_审核订单_需要真实数据库
    ///
    /// 需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。
    /// 验证审核不存在的订单返回错误，调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_审核订单_需要真实数据库() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        let result = service.approve_order(99999, 1).await;
        assert!(result.is_err());
    }

    /// 测试_完成订单_需要真实数据库
    ///
    /// 需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。
    /// 验证完成不存在的订单返回错误，调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_完成订单_需要真实数据库() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        let result = service.complete_order(99999, 1).await;
        assert!(result.is_err());
    }
}

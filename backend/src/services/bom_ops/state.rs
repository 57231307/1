//! BOM Service 状态流转子模块（bom_ops/state）
//!
//! 批次 D10 拆分：从原 `bom_service.rs` 迁移。
//! 包含 `BomService` 的 BOM 状态机方法（提交审核 / 审批）：
//! - `submit`：DRAFT/INACTIVE → PENDING（lock_exclusive 串行化并发提交）
//! - `approve`：PENDING → ACTIVE/INACTIVE（lock_exclusive 串行化并发审批）
//!
//! 状态变更加 lock_exclusive 串行化并发，事务包裹"查询 + 状态检查 + update_with_audit"。
//! 状态写入复用 `AuditLogService::update_with_audit` 落审计日志，本模块不直接调用
//! `ActiveModel::update`，故无需导入 `ActiveModelTrait`。

use chrono::Utc;
use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};
use tracing::info;

use crate::models::bom::{ActiveModel, BomStatus, Entity as BomEntity, Model as BomModel};
use crate::services::bom_service::BomService;
use crate::utils::error::AppError;

impl BomService {
    /// 提交BOM审核：将状态由 DRAFT/INACTIVE 流转为 PENDING
    pub async fn submit(&self, id: i32, user_id: i32) -> Result<BomModel, AppError> {
        info!("提交BOM审核，ID：{}", id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发提交同一 BOM 导致状态不一致
        let txn = self.db.begin().await?;

        let bom = BomEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        // 仅允许从草稿/失效状态提交
        if bom.status == BomStatus::Pending.to_string() {
            return Err(AppError::validation("BOM已处于审核中状态"));
        }

        let mut active: ActiveModel = bom.into();
        active.status = Set(BomStatus::Pending.to_string());
        active.updated_at = Set(Utc::now());

        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 审核BOM：通过 approved 决定最终状态（ACTIVE/INACTIVE）
    pub async fn approve(
        &self,
        id: i32,
        approved: bool,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<BomModel, AppError> {
        info!("审核BOM，ID：{}，通过：{}", id, approved);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发审批同一 BOM 导致状态不一致
        let txn = self.db.begin().await?;

        let bom = BomEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        if bom.status != BomStatus::Pending.to_string() {
            return Err(AppError::validation("仅审核中状态的BOM可以审批"));
        }

        let mut active: ActiveModel = bom.into();
        let new_status = if approved {
            BomStatus::Active
        } else {
            BomStatus::Inactive
        };
        active.status = Set(new_status.to_string());
        if let Some(r) = remark {
            active.remarks = Set(Some(r));
        }
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }
}

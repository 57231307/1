//! 报价单生命周期 impl 子模块（quotation_ops/lifecycle）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移生命周期相关方法。
//! 包含 cancel（取消报价单）+ generate_quotation_no（生成报价单号）。

use chrono::Utc;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::sales_quotation::{self, ActiveModel as QuotationActive, Entity as QuotationEntity};
use crate::models::status::quotation as quotation_status;
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::error::AppError;

impl QuotationService {
    /// 取消报价单（任意非 converted 状态可取消）
    /// 批次 26 v6 P1：状态门+update 移入 txn+lock_exclusive 串行化并发；批次 94 用 update_with_audit 记审计
    pub async fn cancel(
        &self,
        id: i64,
        user_id: i64,
    ) -> Result<sales_quotation::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;
        if existing.status == "converted" {
            return Err(AppError::validation("当前状态不允许此操作".to_string()));
        }
        if existing.status == quotation_status::CANCELLED {
            return Ok(existing);
        }

        let mut active: QuotationActive = existing.into();
        active.status = Set(quotation_status::CANCELLED.to_string());
        active.updated_at = Set(Utc::now());
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "quotation",
            active,
            Some(user_id as i32),
        )
        .await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 生成报价单号：QT + YYYYMMDD + 4 位当日序号
    pub(crate) async fn generate_quotation_no(&self) -> Result<String, ServiceError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let pattern = format!("QT{}%", today);
        let count = QuotationEntity::find()
            .filter(sales_quotation::Column::QuotationNo.like(pattern))
            .count(&*self.db)
            .await?;
        Ok(format!("QT{}{:04}", today, count + 1))
    }
}

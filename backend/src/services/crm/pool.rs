//! CRM 公海服务（crm/pool）
//!
//! 公海池基于 crm_lead 实现（status="pool" 状态），
//! 因为客户主表（customers）没有 owner_id 字段。
//! 拆分自原 `crm_service.rs`。

use crate::models::crm_lead;
use crate::utils::error::AppError;
use sea_orm::{EntityTrait, Set};

use super::cust::CrmService;

impl CrmService {
    /// 从公海领取线索
    /// 返回成功领取的数量
    pub async fn claim_pool_customers(
        &self,
        lead_ids: Vec<i32>,
        user_id: i32,
        _operator_name: &str,
    ) -> Result<usize, AppError> {
        if lead_ids.is_empty() {
            return Ok(0);
        }

        let mut claimed = 0;
        for lid in lead_ids {
            // 验证线索存在且在公海
            let lead = crm_lead::Entity::find_by_id(lid).one(&*self.db).await?;

            if let Some(l) = lead {
                if l.lead_status.as_deref() != Some("pool") {
                    tracing::warn!("线索 {} 不在公海中", lid);
                    continue;
                }

                // 领取：更新状态为 new，并更新 owner_id
                let mut lead_active: crm_lead::ActiveModel = l.into();
                lead_active.lead_status = Set(Some("new".to_string()));
                lead_active.owner_id = Set(user_id);
                lead_active.owner_name = Set(format!("用户{}", user_id));
                lead_active.updated_at = Set(Some(chrono::Utc::now()));
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &*self.db,
                    "auto_audit",
                    lead_active,
                    Some(0),
                )
                .await?;
                claimed += 1;
            } else {
                tracing::warn!("线索 {} 不存在", lid);
            }
        }

        Ok(claimed)
    }
}

//! CRM 公海服务（crm/pool）
//!
//! 公海池基于 crm_lead 实现（status="pool" 状态），
//! 因为客户主表（customers）没有 owner_id 字段。
//! 拆分自原 `crm_service.rs`。

use crate::models::crm_lead;
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;
use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

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

        // v11 批次 37 修复：批量查询所有线索，避免循环内逐个 find_by_id（N+1 查询）
        let leads = crm_lead::Entity::find()
            .filter(crm_lead::Column::Id.is_in(lead_ids.clone()))
            .all(&*self.db)
            .await?;
        let lead_map: std::collections::HashMap<i32, crm_lead::Model> =
            leads.into_iter().map(|l| (l.id, l)).collect();

        let mut claimed = 0;
        for lid in lead_ids {
            // 优先从批量查询结果中取
            let lead = match lead_map.get(&lid) {
                Some(l) => l.clone(),
                None => {
                    tracing::warn!("线索 {} 不存在", lid);
                    continue;
                }
            };

            if lead.lead_status.as_deref() != Some(lead_status::POOL) {
                tracing::warn!("线索 {} 不在公海中", lid);
                continue;
            }

            // 领取：更新状态为 new，并更新 owner_id
            // 注：update_with_audit 需逐条执行以生成审计日志，此处保留循环
            let mut lead_active: crm_lead::ActiveModel = lead.into();
            lead_active.lead_status = Set(Some(lead_status::NEW.to_string()));
            lead_active.owner_id = Set(user_id);
            lead_active.owner_name = Set(format!("用户{}", user_id));
            lead_active.updated_at = Set(Some(chrono::Utc::now()));
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &*self.db,
                "auto_audit",
                lead_active,
                // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                Some(user_id),
            )
            .await?;
            claimed += 1;
        }

        Ok(claimed)
    }
}

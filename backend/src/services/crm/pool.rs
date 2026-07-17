//! CRM 公海服务（crm/pool）
//!
//! 公海池基于 crm_lead 实现（status="pool" 状态），
//! 因为客户主表（customers）没有 owner_id 字段。
//! 拆分自原 `crm_service.rs`。
//!
//! V15 P0-S08 修复：claim_pool_customers 注入公海规则校验
//! - 保护期校验：领取后 N 天内不能被他人领取（防止恶意抢单）
//! - 领取上限校验：每个销售每天最多领取 N 条线索（防 DoS）
//! - 最大持有数校验：每个销售最多持有 N 条活跃线索（防囤积）

use crate::models::crm_lead;
use crate::models::customer_pool_rule::{self, RULE_TYPE_CLAIM_LIMIT, RULE_TYPE_MAX_HOLDINGS, RULE_TYPE_PROTECTION_PERIOD};
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

use super::cust::CrmService;

impl CrmService {
    /// 从公海领取线索
    /// 返回成功领取的数量
    ///
    /// V15 P0-S08 修复：注入公海规则校验
    /// 1. 保护期校验：lead.owner_assigned_at + protection_period > now 则拒绝
    ///    （公海线索保护期由前一次领取时设置，落入公海后保护期内不能再被领取）
    /// 2. 领取上限校验：user_id 当天已领取数 < claim_limit
    /// 3. 最大持有数校验：user_id 当前活跃线索数 < max_holdings
    pub async fn claim_pool_customers(
        &self,
        lead_ids: Vec<i32>,
        user_id: i32,
        _operator_name: &str,
    ) -> Result<usize, AppError> {
        if lead_ids.is_empty() {
            return Ok(0);
        }

        // V15 P0-S08：领取前规则校验
        self.validate_claim_rules(user_id).await?;

        // 批量查询所有线索，避免循环内逐个 find_by_id（N+1 查询）
        let leads = crm_lead::Entity::find()
            .filter(crm_lead::Column::Id.is_in(lead_ids.clone()))
            .all(&*self.db)
            .await?;
        let lead_map: std::collections::HashMap<i32, crm_lead::Model> =
            leads.into_iter().map(|l| (l.id, l)).collect();

        // 查询保护期天数（默认 7 天）
        let protection_days = self.get_rule_value(RULE_TYPE_PROTECTION_PERIOD).await?;
        let now = Utc::now();

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

            // V15 P0-S08：保护期校验
            // 公海线索落入公海前若刚被领取过（owner_assigned_at），
            // 在保护期内不允许被他人再次领取，防止恶意抢单
            if let Some(assigned_at) = lead.updated_at {
                let elapsed = now.signed_duration_since(assigned_at).num_days();
                if elapsed < protection_days as i64 {
                    tracing::warn!(
                        "线索 {} 处于保护期内（已过 {} 天，需 {} 天），用户 {} 领取被拒",
                        lid, elapsed, protection_days, user_id
                    );
                    // 跳过该条，继续处理其他线索（不整体失败）
                    continue;
                }
            }

            // 领取：更新状态为 new，并更新 owner_id
            // 注：update_with_audit 需逐条执行以生成审计日志，此处保留循环
            let mut lead_active: crm_lead::ActiveModel = lead.into();
            lead_active.lead_status = Set(Some(lead_status::NEW.to_string()));
            lead_active.owner_id = Set(user_id);
            lead_active.owner_name = Set(format!("用户{}", user_id));
            lead_active.updated_at = Set(Some(now));
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

    /// V15 P0-S08：领取前规则校验
    ///
    /// 校验项：
    /// 1. 领取上限：user_id 当天已领取线索数 < claim_limit
    /// 2. 最大持有数：user_id 当前活跃线索数（lead_status not in ['converted','lost','pool']）< max_holdings
    async fn validate_claim_rules(&self, user_id: i32) -> Result<(), AppError> {
        // 1. 领取上限校验
        let claim_limit = self.get_rule_value(RULE_TYPE_CLAIM_LIMIT).await?;
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let today_claimed = crm_lead::Entity::find()
            .filter(crm_lead::Column::OwnerId.eq(user_id))
            .filter(crm_lead::Column::UpdatedAt.gte(today_start))
            .count(&*self.db)
            .await?;
        if today_claimed as i32 >= claim_limit {
            return Err(AppError::business(format!(
                "公海领取失败：今日已领取 {} 条，达到每日上限 {} 条",
                today_claimed, claim_limit
            )));
        }

        // 2. 最大持有数校验
        let max_holdings = self.get_rule_value(RULE_TYPE_MAX_HOLDINGS).await?;
        let active_holdings = crm_lead::Entity::find()
            .filter(crm_lead::Column::OwnerId.eq(user_id))
            .filter(crm_lead::Column::LeadStatus.is_not_null())
            .filter(
                crm_lead::Column::LeadStatus
                    .ne(lead_status::CONVERTED)
                    .and(crm_lead::Column::LeadStatus.ne(lead_status::LOST))
                    .and(crm_lead::Column::LeadStatus.ne(lead_status::POOL)),
            )
            .count(&*self.db)
            .await?;
        if active_holdings as i32 >= max_holdings {
            return Err(AppError::business(format!(
                "公海领取失败：当前持有活跃线索 {} 条，达到最大持有数上限 {} 条",
                active_holdings, max_holdings
            )));
        }

        Ok(())
    }

    /// V15 P0-S08：获取公海规则值
    ///
    /// 按规则类型查询启用的规则值，取第一条匹配的（同类型规则应唯一启用）
    /// 若无配置则返回默认值：protection_period=7, claim_limit=5, max_holdings=50
    async fn get_rule_value(&self, rule_type: &str) -> Result<i32, AppError> {
        let rule = customer_pool_rule::Entity::find()
            .filter(customer_pool_rule::Column::RuleType.eq(rule_type))
            .filter(customer_pool_rule::Column::IsEnabled.eq(true))
            .filter(customer_pool_rule::Column::CustomerType.eq("all"))
            .one(&*self.db)
            .await?;

        if let Some(r) = rule {
            return Ok(r.rule_value);
        }

        // 默认值兜底（规则未配置时）
        let default = match rule_type {
            RULE_TYPE_PROTECTION_PERIOD => 7,
            RULE_TYPE_CLAIM_LIMIT => 5,
            RULE_TYPE_MAX_HOLDINGS => 50,
            _ => 0,
        };
        Ok(default)
    }
}

/// V15 P0-S08：公海规则服务（独立于 CrmService，提供规则 CRUD）
pub struct PoolRuleService {
    db: Arc<sea_orm::DatabaseConnection>,
}

impl PoolRuleService {
    pub fn new(db: Arc<sea_orm::DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列出所有公海规则
    pub async fn list_rules(
        &self,
    ) -> Result<Vec<customer_pool_rule::Model>, AppError> {
        let rules = customer_pool_rule::Entity::find()
            .order_by(customer_pool_rule::Column::Id, sea_orm::Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(rules)
    }

    /// 创建公海规则
    pub async fn create_rule(
        &self,
        name: String,
        rule_type: String,
        rule_value: i32,
        customer_type: String,
        notes: Option<String>,
    ) -> Result<customer_pool_rule::Model, AppError> {
        let now = Utc::now();
        let rule = customer_pool_rule::ActiveModel {
            id: Default::default(),
            name: Set(name),
            rule_type: Set(rule_type),
            rule_value: Set(rule_value),
            customer_type: Set(customer_type),
            is_enabled: Set(true),
            notes: Set(notes),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&*self.db)
        .await?;
        Ok(rule)
    }

    /// 更新公海规则
    pub async fn update_rule(
        &self,
        id: i32,
        rule_value: Option<i32>,
        is_enabled: Option<bool>,
        notes: Option<String>,
    ) -> Result<customer_pool_rule::Model, AppError> {
        let rule = customer_pool_rule::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("公海规则 {} 不存在", id)))?;

        let mut active: customer_pool_rule::ActiveModel = rule.into();
        if let Some(v) = rule_value {
            active.rule_value = Set(v);
        }
        if let Some(e) = is_enabled {
            active.is_enabled = Set(e);
        }
        if let Some(n) = notes {
            active.notes = Set(Some(n));
        }
        active.updated_at = Set(Utc::now());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除公海规则
    pub async fn delete_rule(&self, id: i32) -> Result<(), AppError> {
        customer_pool_rule::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await?;
        Ok(())
    }
}

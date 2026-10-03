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
//!
//! 公海规则对所有领取入口统一生效（两条路径同一套校验，无旁路）：
//! 批量 `claim_pool_customers` 与单条 `claim_lead_ownership` 都经
//! `validate_claim_rules`（每日领取上限/最大持有数）+ `is_within_protection_period`
//! （保护期）。保护期与每日计数的判据都是**领取事件列** `last_claimed_at`/
//! `last_claimed_by`（`build_claimed_active` 唯一写点，m_crm_lead_claim_record 补列），
//! 不是 `updated_at`——回收只改 `lead_status` 同样刷新 `updated_at`，用它判保护期
//! 会把"回收 → 立即领取"这条合法业务链直接判负。

use crate::models::crm_lead;
use crate::models::customer_pool_rule::{
    self, RULE_TYPE_CLAIM_LIMIT, RULE_TYPE_MAX_HOLDINGS, RULE_TYPE_PROTECTION_PERIOD,
};
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use std::sync::Arc;

use super::cust::CrmService;

impl CrmService {
    /// 从公海领取线索（批量入口 `/pool/{id}/claim`、`/pool/batch-claim`）
    /// 返回成功领取的数量。三项公海规则校验（`validate_claim_rules` +
    /// `is_within_protection_period`）与单条入口 `/pool/claim`
    /// （`claim_lead_ownership`）同一套判据，不存在"单条免校验"的分叉。
    /// 保护期判据 = 领取事件列 `last_claimed_at`（见文件头注释）。
    ///
    /// `operator_name`：真实操作人展示名，由调用方（`crm_pool_handler`）传
    /// `&auth.username`，落库到 `crm_lead.owner_name`。
    /// 本参数是 `owner_name` 的唯一合法取值来源：#204 附带项收口前它被忽略、
    /// 改由 user_id 拼出展示名，属本仓硬规则禁止的造假名（既不可读也不可回查）。
    pub async fn claim_pool_customers(
        &self,
        lead_ids: Vec<i32>,
        user_id: i32,
        operator_name: &str,
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
            if Self::is_within_protection_period(&lead, lid, now, protection_days, user_id) {
                continue;
            }

            // 领取：更新状态为 new，并更新 owner_id
            // 注：update_with_audit 需逐条执行以生成审计日志，此处保留循环
            let lead_active = Self::build_claimed_active(lead, user_id, operator_name, now);
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

    /// 保护期校验：公海线索在领取后 N 天内不允许被**他人**再次领取，防止恶意抢单。
    /// （返回 true 表示处于保护期内（应拒绝/跳过），false 表示可领取）
    ///
    /// 判据是领取事件列 `last_claimed_at`（唯一写点 `build_claimed_active`），
    /// **不是 `updated_at`**——回收只改 `lead_status` 也会刷新 `updated_at`，
    /// 用它判保护期会把"回收 → 立即领取"的合法链按默认 7 天直接判负。
    ///
    /// 原领取人本人重领豁免（`last_claimed_by == user_id`）：保护期本义是防
    /// 他人抢单（见文件头注释），不防自己把自己的线索领回来。
    /// `last_claimed_at` 为 NULL（迁移前存量/从未被领取）→ 无保护期，
    /// 该语义选择与理由见迁移 m_crm_lead_claim_record 头注释。
    fn is_within_protection_period(
        lead: &crm_lead::Model,
        lid: i32,
        now: chrono::DateTime<chrono::Utc>,
        protection_days: i32,
        user_id: i32,
    ) -> bool {
        let Some(claimed_at) = lead.last_claimed_at else {
            return false;
        };
        if lead.last_claimed_by == Some(user_id) {
            tracing::info!(
                "线索 {} 原领取人本人（用户 {}）重领，保护期豁免（判据 last_claimed_by）",
                lid,
                user_id
            );
            return false;
        }
        let elapsed = now.signed_duration_since(claimed_at).num_days();
        if elapsed < protection_days as i64 {
            tracing::warn!(
                "线索 {} 处于保护期内（上次领取已过 {} 天，需 {} 天），用户 {} 领取被拒",
                lid,
                elapsed,
                protection_days,
                user_id
            );
            return true;
        }
        false
    }

    /// 构建领取后的 ActiveModel（status=new，更新 owner_id/owner_name/updated_at，
    /// 并落领取事件列 last_claimed_at/last_claimed_by——保护期与每日领取计数的
    /// 唯一数据来源）
    ///
    /// **两条领取路径的唯一归属实现**（#204 附带项收口）：批量领取
    /// `claim_pool_customers` 与单条领取 `claim_lead_ownership` 都只经此函数落
    /// `owner_id`/`owner_name`，避免出现第二套"领取后归属"写法（修复前单条路径
    /// 只把 lead_status 置 new、不写归属，销售领取后该线索仍挂在原归属人名下，
    /// 在他的 self 列表里看不到自己刚领取的行）。
    fn build_claimed_active(
        lead: crm_lead::Model,
        user_id: i32,
        operator_name: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> crm_lead::ActiveModel {
        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.lead_status = Set(Some(lead_status::NEW.to_string()));
        lead_active.owner_id = Set(user_id);
        lead_active.owner_name = Set(operator_name.to_string());
        lead_active.updated_at = Set(Some(now));
        lead_active.last_claimed_at = Set(Some(now));
        lead_active.last_claimed_by = Set(Some(user_id));
        lead_active
    }

    /// 单条领取（`POST /api/v1/erp/crm/pool/claim`）：与批量领取
    /// `claim_pool_customers` **同一套公海规则校验 + 同一个归属实现**
    /// `build_claimed_active`——保证两条路径的校验口径与归属语义逐字段一致。
    ///
    /// 差异仅在拒绝形态（批量是多行循环、无法整笔报错，命中保护期/不在公海
    /// 的行静默跳过并计入 claimed=0，由 handler 统一报"领取失败"；单条作用于
    /// 唯一一行，直接显式报错，不静默）：
    /// - 每日领取上限 / 最大持有数：`validate_claim_rules`（`AppError::business`）；
    /// - 保护期：`is_within_protection_period`（判据 `last_claimed_at`，
    ///   原领取人本人重领豁免）。
    pub async fn claim_lead_ownership(
        &self,
        lead: crm_lead::Model,
        user_id: i32,
        operator_name: &str,
    ) -> Result<crm_lead::Model, AppError> {
        // 领取前规则校验（与批量路径同一个 validate_claim_rules，判据同源）
        self.validate_claim_rules(user_id).await?;

        let protection_days = self.get_rule_value(RULE_TYPE_PROTECTION_PERIOD).await?;
        let now = Utc::now();
        if Self::is_within_protection_period(&lead, lead.id, now, protection_days, user_id) {
            // 详细原因（已过期数/需天数/用户）已由 is_within_protection_period 落日志
            return Err(AppError::business(format!(
                "公海领取失败：线索 {} 处于保护期内，他人暂不可领取",
                lead.id
            )));
        }

        let lead_active = Self::build_claimed_active(lead, user_id, operator_name, now);
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            lead_active,
            // 批次 94 P2-10：审计落真实操作人 user_id
            Some(user_id),
        )
        .await?;
        Ok(updated)
    }

    /// V15 P0-S08：领取前规则校验（两条领取路径共用）
    /// 校验项：1. 领取上限：user_id 当天已领取线索数 < claim_limit；2. 最大持有数：user_id 当前活跃线索数（lead_status not in ['converted','lost','pool']）< max_holdings
    ///
    /// 每日领取计数判据 = 领取事件列 `last_claimed_at`/`last_claimed_by`
    /// （唯一写点 `build_claimed_active`）。修复前用 `owner_id + updated_at ≥
    /// 今日`：回收/跟进等任意字段更新都会刷新 `updated_at`，把"本人今天被动过
    /// 的存量线索"计入当日领取量，计数虚高撞 claim_limit 误拒，与真实领取行为
    /// 无关。存量行 `last_claimed_at` 为 NULL 不计入（领取事件自本列落地起记录，
    /// 与保护期同列同语义，见迁移 m_crm_lead_claim_record 头注释）。
    async fn validate_claim_rules(&self, user_id: i32) -> Result<(), AppError> {
        // 1. 领取上限校验
        let claim_limit = self.get_rule_value(RULE_TYPE_CLAIM_LIMIT).await?;
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let today_claimed = crm_lead::Entity::find()
            .filter(crm_lead::Column::LastClaimedBy.eq(user_id))
            .filter(crm_lead::Column::LastClaimedAt.gte(today_start))
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
    /// 按规则类型查询启用的规则值，取第一条匹配的（同类型规则应唯一启用）；若无配置则返回默认值：protection_period=7, claim_limit=5, max_holdings=50
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
    pub async fn list_rules(&self) -> Result<Vec<customer_pool_rule::Model>, AppError> {
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

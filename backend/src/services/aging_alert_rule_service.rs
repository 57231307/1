//! 账龄预警规则服务
//!
//! 提供预警规则的 CRUD 和查询功能

use crate::models::aging_alert_rule;
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// 创建预警规则请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub rule_name: String,
    pub rule_code: String,
    pub aging_bucket: String,
    pub threshold_days: i32,
    pub threshold_amount: Option<rust_decimal::Decimal>,
    pub alert_level: String,
    pub notify_method: String,
    pub notify_roles: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub remarks: Option<String>,
}

/// 更新预警规则请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAlertRuleRequest {
    pub rule_name: Option<String>,
    pub aging_bucket: Option<String>,
    pub threshold_days: Option<i32>,
    pub threshold_amount: Option<Option<rust_decimal::Decimal>>,
    pub alert_level: Option<String>,
    pub notify_method: Option<String>,
    pub notify_roles: Option<Option<Vec<String>>>,
    pub is_active: Option<bool>,
    pub remarks: Option<Option<String>>,
}

/// 查询参数
#[derive(Debug, Clone, Default)]
pub struct AlertRuleQueryParams {
    pub aging_bucket: Option<String>,
    pub alert_level: Option<String>,
    pub is_active: Option<bool>,
    pub page: u64,
    pub page_size: u64,
}

pub struct AgingAlertRuleService {
    db: Arc<DatabaseConnection>,
}

impl AgingAlertRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建预警规则
    pub async fn create(
        &self,
        req: CreateAlertRuleRequest,
        user_id: i32,
    ) -> Result<aging_alert_rule::Model, AppError> {
        info!("用户 {} 正在创建账龄预警规则：{}", user_id, req.rule_name);

        let active = aging_alert_rule::ActiveModel {
            rule_name: Set(req.rule_name),
            rule_code: Set(req.rule_code),
            aging_bucket: Set(req.aging_bucket),
            threshold_days: Set(req.threshold_days),
            threshold_amount: Set(req.threshold_amount),
            alert_level: Set(req.alert_level),
            notify_method: Set(req.notify_method),
            notify_roles: Set(req.notify_roles),
            is_active: Set(req.is_active.unwrap_or(true)),
            remarks: Set(req.remarks),
            ..Default::default()
        };

        let rule = active.insert(&*self.db).await?;
        info!("账龄预警规则创建成功：ID {}", rule.id);
        Ok(rule)
    }

    /// 查询规则列表
    pub async fn list(
        &self,
        params: AlertRuleQueryParams,
    ) -> Result<(Vec<aging_alert_rule::Model>, u64), AppError> {
        let mut query = aging_alert_rule::Entity::find();

        if let Some(ref bucket) = params.aging_bucket {
            query = query.filter(aging_alert_rule::Column::AgingBucket.eq(bucket.as_str()));
        }
        if let Some(ref level) = params.alert_level {
            query = query.filter(aging_alert_rule::Column::AlertLevel.eq(level.as_str()));
        }
        if let Some(is_active) = params.is_active {
            query = query.filter(aging_alert_rule::Column::IsActive.eq(is_active));
        }

        let total = query.clone().count(&*self.db).await?;

        let rules = query
            .order_by(aging_alert_rule::Column::Id, Order::Desc)
            .offset(params.page * params.page_size)
            .limit(params.page_size)
            .all(&*self.db)
            .await?;

        Ok((rules, total))
    }

    /// 获取规则详情
    pub async fn get_by_id(&self, id: i32) -> Result<aging_alert_rule::Model, AppError> {
        let rule = aging_alert_rule::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("账龄预警规则不存在：{}", id)))?;
        Ok(rule)
    }

    /// 更新规则
    pub async fn update(
        &self,
        id: i32,
        req: UpdateAlertRuleRequest,
    ) -> Result<aging_alert_rule::Model, AppError> {
        let rule = self.get_by_id(id).await?;

        let mut active: aging_alert_rule::ActiveModel = rule.into();

        if let Some(v) = req.rule_name {
            active.rule_name = Set(v);
        }
        if let Some(v) = req.aging_bucket {
            active.aging_bucket = Set(v);
        }
        if let Some(v) = req.threshold_days {
            active.threshold_days = Set(v);
        }
        if let Some(v) = req.threshold_amount {
            active.threshold_amount = Set(v);
        }
        if let Some(v) = req.alert_level {
            active.alert_level = Set(v);
        }
        if let Some(v) = req.notify_method {
            active.notify_method = Set(v);
        }
        if let Some(v) = req.notify_roles {
            active.notify_roles = Set(v);
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(v);
        }

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除规则
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let rule = self.get_by_id(id).await?;

        let active: aging_alert_rule::ActiveModel = rule.into();
        active.delete(&*self.db).await?;

        info!("账龄预警规则 {} 已删除", id);
        Ok(())
    }
}

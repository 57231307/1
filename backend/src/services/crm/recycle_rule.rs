//! CRM 公海回收规则服务（crm/recycle_rule）
//!
//! 批次 23 v5 P0-4 修复：将原本存于 `handlers/missing_handlers.rs` 的
//! `static RECYCLE_RULES: LazyLock<RwLock<Vec<RecycleRule>>>` 内存存储
//! 迁移至数据库 `crm_recycle_rules` 表，避免进程重启后丢失运行时修改。
//!
//! 提供 list/create/update/delete 四个 CRUD 方法，使用 SeaORM 操作数据库。

use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Order, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::crm_recycle_rule::{self, Entity as RecycleRuleEntity};
use crate::utils::error::AppError;

/// CRM 回收规则 DTO（与数据库表字段一一对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycleRule {
    pub id: i32,
    pub name: String,
    /// 未跟进超过 N 天后自动回收到公海
    pub days: i32,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建回收规则请求
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateRecycleRulePayload {
    #[validate(length(min = 1, max = 100, message = "规则名称不能为空"))]
    pub name: String,
    #[validate(range(min = 1, max = 365, message = "回收天数必须在 1-365 之间"))]
    pub days: i32,
    pub is_enabled: Option<bool>,
}

/// 更新回收规则请求
#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpdateRecycleRulePayload {
    #[validate(length(min = 1, max = 100, message = "规则名称不能为空"))]
    pub name: Option<String>,
    #[validate(range(min = 1, max = 365, message = "回收天数必须在 1-365 之间"))]
    pub days: Option<i32>,
    pub is_enabled: Option<bool>,
}

/// CRM 公海回收规则服务
pub struct RecycleRuleService {
    db: Arc<DatabaseConnection>,
}

impl RecycleRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 将 ORM Model 转换为对外 DTO
    fn to_dto(model: crate::models::crm_recycle_rule::Model) -> RecycleRule {
        RecycleRule {
            id: model.id,
            name: model.name,
            days: model.days,
            is_enabled: model.is_enabled,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }

    /// 获取回收规则列表（按 id 升序）
    pub async fn list_rules(&self) -> Result<Vec<RecycleRule>, AppError> {
        let rules = RecycleRuleEntity::find()
            .order_by(crm_recycle_rule::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(rules.into_iter().map(Self::to_dto).collect())
    }

    /// 创建回收规则
    pub async fn create_rule(
        &self,
        payload: CreateRecycleRulePayload,
    ) -> Result<RecycleRule, AppError> {
        let now = Utc::now();
        let active = crm_recycle_rule::ActiveModel {
            id: Default::default(),
            name: Set(payload.name),
            days: Set(payload.days),
            is_enabled: Set(payload.is_enabled.unwrap_or(true)),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active.insert(&*self.db).await?;
        Ok(Self::to_dto(model))
    }

    /// 更新回收规则（部分更新）
    pub async fn update_rule(
        &self,
        id: i32,
        payload: UpdateRecycleRulePayload,
    ) -> Result<RecycleRule, AppError> {
        let existing = RecycleRuleEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("回收规则 {} 不存在", id)))?;

        let mut active: crm_recycle_rule::ActiveModel = existing.into();
        if let Some(name) = payload.name {
            active.name = Set(name);
        }
        if let Some(days) = payload.days {
            active.days = Set(days);
        }
        if let Some(is_enabled) = payload.is_enabled {
            active.is_enabled = Set(is_enabled);
        }
        active.updated_at = Set(Utc::now());

        let model = active.update(&*self.db).await?;
        Ok(Self::to_dto(model))
    }

    /// 删除回收规则
    pub async fn delete_rule(&self, id: i32) -> Result<(), AppError> {
        let result = RecycleRuleEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!("回收规则 {} 不存在", id)));
        }
        Ok(())
    }
}

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.1-D5: 线索分配规则 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "lead_allocation_rule")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 规则名称
    pub rule_name: String,

    /// 规则类型：round_robin/weighted/source_based/industry_based
    pub rule_type: String,

    /// 适用来源过滤
    pub source_filter: Option<String>,

    /// 适用行业过滤
    pub industry_filter: Option<String>,

    /// 适用区域过滤
    pub region_filter: Option<String>,

    /// 分配用户ID列表
    pub assigned_user_ids: Option<serde_json::Value>,

    /// 权重配置
    pub weights: Option<serde_json::Value>,

    /// 每日分配上限
    pub daily_limit: i32,

    /// 规则优先级
    pub priority: i32,

    /// 是否启用
    pub is_active: bool,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,

    /// 更新时间
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

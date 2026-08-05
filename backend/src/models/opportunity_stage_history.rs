use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.2-D5: 商机阶段变更历史 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opportunity_stage_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 商机ID
    pub opportunity_id: i32,

    /// 原阶段
    pub from_stage: Option<String>,

    /// 新阶段
    pub to_stage: String,

    /// 变更时间
    pub changed_at: DateTime<Utc>,

    /// 变更人
    pub changed_by: Option<i32>,

    /// 在原阶段停留天数
    pub duration_days: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

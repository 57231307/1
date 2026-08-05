use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.2-D6: 商机-竞争对手关联 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opportunity_competitor")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 商机ID
    pub opportunity_id: i32,

    /// 竞争对手ID
    pub competitor_id: i32,

    /// 威胁级别：low/medium/high
    pub threat_level: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

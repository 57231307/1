use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.2-D6: 竞争对手 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "competitor")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 竞争对手名称
    pub name: String,

    /// 优势
    pub strengths: Option<String>,

    /// 劣势
    pub weaknesses: Option<String>,

    /// 官网
    pub website: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,

    /// 更新时间
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

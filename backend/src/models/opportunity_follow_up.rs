use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.2-D7: 商机跟进记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opportunity_follow_up")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 商机ID
    pub opportunity_id: i32,

    /// 跟进方式：phone/email/visit/meeting/wechat
    pub follow_up_type: String,

    /// 跟进内容
    pub content: String,

    /// 跟进时间
    pub follow_up_time: DateTime<Utc>,

    /// 下次跟进日期
    pub next_follow_up_date: Option<NaiveDate>,

    /// 跟进人ID
    pub user_id: i32,

    /// 跟进人姓名
    pub user_name: String,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

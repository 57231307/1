#![allow(dead_code)]
//! 特种设备操作证管理模型（V15 P2 B08-P2-9）
//!
//! 依据：《安全生产法》《特种作业人员安全技术培训考核管理规定》
//! 业务：染缸/定型机/烘干机等特种设备操作证到期预警

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "operation_certificates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub certificate_no: String,
    pub equipment_type: String,
    pub issuing_authority: Option<String>,
    pub issue_date: NaiveDate,
    pub expiry_date: NaiveDate,
    pub certificate_url: Option<String>,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}

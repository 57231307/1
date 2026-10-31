#![allow(dead_code)]
//! 送检项目清单模型（按人群、接触方式与市场推出的应检项目集）
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_safety_submission_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub submission_id: i32,
    pub item_id: i32,
    /// 清单行归因（GB18401、GB31701、EXPORT、CUSTOMER），
    /// 取值域 `models::status::textile_item_source`；CUSTOMER 行不被自动重生成冲掉
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

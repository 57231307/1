#![allow(dead_code)]
//! 环评文件存档模型（V15 P2 B08-20）
//!
//! 依据：《环境影响评价法》《建设项目环境保护管理条例》
//! 业务：环评报告/环评批复/竣工环保验收文件存档
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "environmental_assessment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub doc_type: String,
    pub doc_name: String,
    pub doc_url: String,
    pub approval_date: Option<NaiveDate>,
    pub approval_authority: Option<String>,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

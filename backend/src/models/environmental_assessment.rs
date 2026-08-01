#![allow(dead_code)]
//! 环境影响评价存档模型（V15 P2 B08-P2-8）
//!
//! 依据：《环境影响评价法》《建设项目环境保护管理条例》
//! 业务：环评报告/环评批复/竣工环保验收文件存档

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "environmental_assessments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub project_name: String,
    pub file_type: String,
    pub document_no: Option<String>,
    pub approval_date: Option<NaiveDate>,
    pub assessment_agency: Option<String>,
    pub file_url: Option<String>,
    pub valid_until: Option<NaiveDate>,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

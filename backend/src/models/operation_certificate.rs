#![allow(dead_code)]
//! 特种设备操作证管理模型（V15 P2 B08-25）
//!
//! 依据：《安全生产法》《特种作业人员安全技术培训考核管理规定》
//! 业务：染缸/定型机/烘干机等特种设备操作证到期预警
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "operation_certificate")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub worker_id: i32,
    pub certificate_no: String,
    pub certificate_type: String,
    pub equipment_name: Option<String>,
    pub issue_date: NaiveDate,
    pub expiry_date: NaiveDate,
    pub issuing_authority: Option<String>,
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

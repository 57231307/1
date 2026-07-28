#![allow(dead_code)]
//! 职业健康体检档案模型（occupational_health_exams 表）
//!
//! V15 P1 batch-08 缺陷 24：职业健康合规
//! 依据：《职业病防治法》第35条 上岗前/在岗期间/离岗时体检

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 职业健康体检档案模型（管理工人职业健康体检档案，在岗期间每年一次体检到期提醒）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "occupational_health_exams")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工人ID
    pub worker_id: i32,
    /// 体检类型：pre_employment(上岗前) / in_service(在岗期间) / resignation(离岗时)
    pub exam_type: String,
    /// 体检日期
    pub exam_date: chrono::NaiveDate,
    /// 下次体检日期
    pub next_exam_date: Option<chrono::NaiveDate>,
    /// 体检机构
    pub exam_organization: Option<String>,
    /// 体检结果：normal(正常) / abnormal(异常) / contraindication(禁忌)
    pub exam_result: String,
    /// 危害暴露史（JSON）
    pub hazard_exposure: Option<serde_json::Value>,
    /// 禁忌症
    pub contraindications: Option<String>,
    /// 体检报告URL
    pub report_url: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

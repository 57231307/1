#![allow(dead_code)]
//! 第三方报告逐项目实测结果模型（结构化承载：存值、单位、检测方法，判定器出三态）
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_report_item_result")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub report_id: i32,
    pub item_id: i32,
    /// 数值与等级型实测值；等级型存半级编码数值（3-4 级记 3.5）
    pub measured_value: Option<Decimal>,
    /// 感官与布尔证据型 flag（none、detected）与方法补充说明文本
    pub measured_text: Option<String>,
    /// 实测单位快照，判定器按数值判定不换算单位
    pub measured_unit: Option<String>,
    /// 报告所载检测方法（标准号或方法名）
    pub test_method: Option<String>,
    /// 逐项目判定三态加待判定，取值域 `models::status::textile_verdict`
    pub verdict: String,
    pub judged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

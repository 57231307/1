#![allow(dead_code)]
//! GB 31701 童装加考核查登记与判定模型（绳带、锐利物、附件、标签位置）
//!
//! 设计红线：measurable 行有实测值走判定器自动出三态；manual 行只登记条目与人工结论位，
//! 判定器绝不产出自动结论，搬运人工结论时也必须逐行有值，不留静默放行。
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_child_safety_check")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub submission_id: i32,
    pub item_id: i32,
    /// 核查方式快照，取值域 `models::status::textile_check_kind`
    pub check_kind: String,
    /// 可测项实测值（铅、镉、邻苯、燃烧性能、腰部绳带伸出长度 mm 等）
    pub measured_value: Option<Decimal>,
    pub measured_unit: Option<String>,
    /// boolean 与感官可测项实测 flag（none、detected），must_none 判定行消费本列
    pub measured_flag: Option<String>,
    /// 核查描述（绳带位置、附件形态、标签缝制位置等现场记录文字）
    pub check_desc: Option<String>,
    /// 人工结论位，取值域 `models::status::textile_verdict::OVERALL`；
    /// measurable 行为 NULL 由判定器写 verdict，manual 行无本列取值时整单判定拒绝完成
    pub manual_conclusion: Option<String>,
    /// 核查判定态，取值域 `models::status::textile_verdict`
    /// （measurable 行由判定器产出，manual 行在整单判定搬运 manual_conclusion）
    pub verdict: String,
    pub judged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

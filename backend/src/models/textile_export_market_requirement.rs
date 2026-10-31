#![allow(dead_code)]
//! 出口市场合规要求登记模型（OEKO-TEX 分级、REACH、CPSIA、日本自主管控）
//!
//! 登记位可被报价与订单侧只读消费；限值细则不进判定矩阵，法规数值以官方现行文本为准。
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_export_market_requirement")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub requirement_code: String,
    /// 所属出口市场（eu、us、jp、other，内销 dom 不挂出口要求）
    pub market: String,
    /// 要求类型，取值域 `models::status::export_requirement_type`
    pub requirement_type: String,
    /// OEKO-TEX 分级位（该要求指向的具体分级时填写），取值域 `models::status::oeko_tex_class`
    pub oeko_tex_class: Option<String>,
    /// 依据法规或标准名号
    pub standard_ref: String,
    /// 要求细则文字（登记位说明，不是判定源）
    pub detail: String,
    pub mandatory: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

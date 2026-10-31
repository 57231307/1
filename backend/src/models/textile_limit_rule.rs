#![allow(dead_code)]
//! 纺织品安全限值判定表模型（四维矩阵：项目乘安全类别乘产品形态乘条件标志）
//!
//! 判定器唯一限值来源，行数据可维护，比较器与上下界全部由行表达，不在代码里硬编码。
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_limit_rule")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub item_id: i32,
    /// 安全类别维（GB18401_A、GB18401_B、GB18401_C 或通配 any），
    /// 取值域 `models::status::textile_limit_scope::CATEGORIES`
    pub safety_category: String,
    /// 产品形态维（final、intermediate 或通配 any），
    /// 取值域 `models::status::textile_limit_scope::FORMS`
    pub product_form: String,
    /// 条件标志维（none 或四枚豁免标签），
    /// 取值域 `models::status::textile_limit_scope::CONDITION_TAGS`
    pub condition_tag: String,
    /// 比较器，取值域 `models::status::textile_comparator`
    pub comparator: String,
    /// 下限（between 与 gte 消费；等级型存半级编码数值）
    pub limit_low: Option<Decimal>,
    /// 上限（between 与 lte 消费）
    pub limit_high: Option<Decimal>,
    /// 依据标准号（只存标准号，细则文字进 remark）
    pub clause_ref: String,
    pub remark: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

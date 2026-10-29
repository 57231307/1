#![allow(dead_code)]
//! 纺织品安全送检单模型（取样批次口径为同品种同色号同工艺）
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_safety_submission")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub submission_no: String,
    /// 品种维（只读关联 products，产品侧字段写入归产品车道）
    pub product_id: i32,
    /// 生产批次号（客户按批次送检时的对账键），未指定为 NULL
    pub batch_no: Option<String>,
    /// 色号维，同批判定要素
    pub color_no: String,
    /// 工艺维，同批判定要素
    pub process_no: String,
    /// 色号送检代表性，取值域 `models::status::sampling_color_role`
    pub color_role: String,
    /// 终端人群，取值域 `models::status::textile_end_user_group`
    pub end_user_group: String,
    /// 接触方式，取值域 `models::status::textile_contact_type`
    pub contact_type: String,
    /// 申报安全类别稳定码，取值域 `models::status::textile_safety_category`
    pub safety_category: String,
    /// 产品形态，取值域 `models::status::textile_product_form`
    pub product_form: String,
    /// 目标市场，取值域 `models::status::export_market`
    pub target_market: String,
    /// 豁免标签逗号串（无空格），逐 token 取值域 `models::status::textile_exemption_tag`
    pub exemption_tags: Option<String>,
    /// 状态机，取值域 `models::status::submission_status`
    pub status: String,
    /// 整单结论（pass、fail、na），判定完成前恒 NULL
    pub overall_verdict: Option<String>,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

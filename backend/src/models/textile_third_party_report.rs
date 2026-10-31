#![allow(dead_code)]
//! 第三方检测报告模型（完整性门：CMA、CNAS 资质位、六项要素、有效期、按批次一致性）
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_third_party_report")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub report_no: String,
    pub submission_id: i32,
    pub inspection_agency: String,
    /// 是否带 CMA 资质认定标志（完整性门要素，缺位受理拒绝点名 cma）
    pub has_cma: bool,
    /// 是否带 CNAS 认可标志（完整性门要素，缺位受理拒绝点名 cnas）
    pub has_cnas: bool,
    pub sample_name: Option<String>,
    pub material_composition: Option<String>,
    /// 报告侧色号，与送检单不一致时点名 batch_mismatch
    pub color_no: Option<String>,
    /// 报告侧工艺，与送检单不一致时点名 batch_mismatch
    pub process_no: Option<String>,
    /// 检验依据原文形态（如 "GB 18401-2010 B类"），受理门做存在性与格式校验
    pub test_basis: Option<String>,
    pub conclusion: Option<String>,
    /// OEKO-TEX 分级位，取值域 `models::status::oeko_tex_class`
    pub oeko_tex_class: Option<String>,
    pub issue_date: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    /// 受理态，取值域 `models::status::report_acceptance_status`
    pub acceptance_status: String,
    /// 缺项代号逗号串，逐 token 取值域 `models::status::report_element_code`
    pub rejection_detail: String,
    pub report_file_url: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

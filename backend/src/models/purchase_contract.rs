#![allow(dead_code)]
//! 采购合同 Entity
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_contracts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub contract_no: String,
    pub contract_name: String,
    pub contract_type: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub signed_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub payment_terms: Option<String>,
    pub payment_method: Option<String>,
    pub delivery_date: Option<NaiveDate>,
    pub delivery_location: Option<String>,
    /// 备注（m0016 迁移补列，可空）
    pub remark: Option<String>,
    pub status: String,
    /// 合同审批通过理由（m0079 加列，TEXT 可空；NULL=历史行未采集）
    pub approval_reason: Option<String>,
    /// 合同审批拒绝理由（m0079 加列，TEXT 可空；cancel 动作不使用本列）
    pub rejected_reason: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 质量条款文本，可空。空值表示模板前签约未固化条款
    pub quality_terms: Option<String>,
    /// 违约责任条款文本，可空。空值表示模板前签约未固化条款
    pub breach_liability: Option<String>,
    /// 争议解决条款文本，可空。空值表示模板前签约未固化条款
    pub dispute_resolution: Option<String>,
    /// 履行期限条款文本，可空。空值表示模板前签约未固化条款
    pub performance_period: Option<String>,
    /// 电子签章时间，可空。空值表示未走电子签
    pub signed_at: Option<DateTime<Utc>>,
    /// 签章人用户主键，可空
    pub signed_by_user_id: Option<i32>,
    /// 合同内容哈希，可空
    pub signature_hash: Option<String>,
    /// 电子签章图片地址，可空
    pub signature_image_url: Option<String>,
    /// CA 证书内容，可空
    pub signature_certificate: Option<String>,
    /// 印花税金额，可空。空值表示未采集，与 0 元是两回事
    pub stamp_tax_amount: Option<Decimal>,
    /// 内外销标记，可空，取值见 models::status::contract_market。空值表示模板前签约未采集
    pub contract_market: Option<String>,
    /// 含税标记，可空三态。真为含税，假为不含税，空为未采集
    pub tax_inclusive: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联创建人（created_by -> users.id）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}
impl ActiveModelBehavior for ActiveModel {}

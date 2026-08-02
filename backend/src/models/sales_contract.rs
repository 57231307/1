#![allow(dead_code)]
//! 销售合同 Entity
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_contracts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub contract_no: String,
    pub contract_name: String,
    pub contract_type: Option<String>,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub signed_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub payment_terms: Option<String>,
    pub payment_method: Option<String>,
    pub delivery_date: Option<NaiveDate>,
    pub delivery_location: Option<String>,
    pub status: String,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// V15 P1-08-10：电子签章时间（《电子签名法》合规）
    pub signed_at: Option<DateTime<Utc>>,
    /// V15 P1-08-10：签章人用户ID
    pub signed_by_user_id: Option<i32>,
    /// V15 P1-08-10：合同内容哈希（SHA-256，防篡改）
    pub signature_hash: Option<String>,
    /// V15 P1-08-10：电子签章图片URL
    pub signature_image_url: Option<String>,
    /// V15 P1-08-10：CA证书内容（PEM格式）
    pub signature_certificate: Option<String>,
    /// V15 P2 B08-P2-4：质量条款（《民法典》合同编必备）
    pub quality_terms: Option<String>,
    /// V15 P2 B08-P2-4：违约责任条款
    pub breach_liability: Option<String>,
    /// V15 P2 B08-P2-4：争议解决条款
    pub dispute_resolution: Option<String>,
    /// V15 P2 B08-P2-4：履行期限
    pub performance_period: Option<String>,
    /// V15 P2 B08-P2-7：印花税金额（购销合同 0.3‰/加工承揽合同 0.5‰）
    pub stamp_tax_amount: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

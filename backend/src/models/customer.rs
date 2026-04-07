#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 客户 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "customers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 客户编码（唯一）
    #[sea_orm(unique)]
    pub customer_code: String,

    /// 客户名称
    pub customer_name: String,

    /// 联系人
    pub contact_person: Option<String>,

    /// 联系电话
    pub contact_phone: Option<String>,

    /// 联系邮箱
    pub contact_email: Option<String>,

    /// 地址
    pub address: Option<String>,

    /// 城市
    pub city: Option<String>,

    /// 省份
    pub province: Option<String>,

    /// 国家
    pub country: Option<String>,

    /// 邮编
    pub postal_code: Option<String>,

    /// 信用额度
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub credit_limit: Decimal,

    /// 账期（天）
    pub payment_terms: i32,

    /// 税号
    pub tax_id: Option<String>,

    /// 开户行
    pub bank_name: Option<String>,

    /// 银行账号
    pub bank_account: Option<String>,

    /// 状态：active-活跃，inactive-停用，blacklist-黑名单
    pub status: String,

    /// 客户类型：retail-零售，wholesale-批发，vip-VIP
    pub customer_type: String,

    /// 备注
    pub notes: Option<String>,

    /// 客户行业：服装/家纺/产业用
    pub customer_industry: Option<String>,

    /// 主营产品
    pub main_products: Option<String>,

    /// 年采购量（万米）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub annual_purchase: Option<Decimal>,

    /// 质量要求：一般/较高/严格
    pub quality_requirement: Option<String>,

    /// 检验标准偏好
    pub inspection_standard: Option<String>,

    /// 创建人
    pub created_by: Option<i32>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 客户 Relation
#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
}

impl ActiveModelBehavior for ActiveModel {}

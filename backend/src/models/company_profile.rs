#![allow(dead_code)]
//! 公司档案模型
//!
//! 映射 `company_profile` 表(见 `migration/src/domain/company_profile/`):全库唯一的公司
//! 抬头主数据,单例形态——整表仅 `id=1` 一行(`ck_company_profile_singleton` 固化)。
//! 字段可空性与 DDL 逐列一致(NOT NULL 列为非 Option,可空列为 Option;`registered_capital`
//! 为 DECIMAL/Decimal,`establishment_date` 为 DATE/NaiveDate,`updated_at` 为
//! TIMESTAMPTZ/DateTimeWithTimeZone)。
//!
//! 序列化口径:`#[serde]` 键名即本结构体 snake_case 字段名;敏感列(credit_code /
//! legal_representative / tax_registration_number / bank_name / bank_account /
//! taxpayer_type)只允许经全量读写面(`/company-profile/full`)出入,公开投影
//! `CompanyProfilePublic` 从类型层不含这些字段。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 公司档案 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "company_profile")]
pub struct Model {
    /// 单例主键,恒为 1
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 公司名称(NOT NULL,打印页眉与公开投影的唯一来源)
    pub name: String,

    /// 公司简称(可空)
    pub short_name: Option<String>,

    /// 统一社会信用代码(敏感列,可空)
    pub credit_code: Option<String>,

    /// 法定代表人(敏感列,可空)
    pub legal_representative: Option<String>,

    /// 税务登记号(敏感列,可空)
    pub tax_registration_number: Option<String>,

    /// 开户银行(敏感列,可空)
    pub bank_name: Option<String>,

    /// 银行账号(敏感列,可空)
    pub bank_account: Option<String>,

    /// 纳税人类型(敏感列,可空)
    pub taxpayer_type: Option<String>,

    /// 电话(可空)
    pub phone: Option<String>,

    /// 传真(可空)
    pub fax: Option<String>,

    /// 邮箱(可空)
    pub email: Option<String>,

    /// 网址(可空)
    pub website: Option<String>,

    /// 地址(可空,TEXT)
    pub address: Option<String>,

    /// 注册资本(DECIMAL(18,2),可空)
    pub registered_capital: Option<Decimal>,

    /// 成立日期(DATE,可空)
    pub establishment_date: Option<chrono::NaiveDate>,

    /// 公司 logo:base64 编码 PNG 字节的裸串(无 data: 前缀;写入侧校验 PNG 魔数与 512KB 上限)
    pub logo: Option<String>,

    /// 备注(可空,TEXT)
    pub description: Option<String>,

    /// 最后更新时间(TIMESTAMPTZ,NOT NULL)
    pub updated_at: DateTimeWithTimeZone,

    /// 最后编辑人 user_id(0=播种后从未被编辑)
    pub updated_by: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#![allow(dead_code)]
//! 条款库 Entity
//!
//! 映射 `clause_library` 表，一枚条款码对应一枚可复用条款正文，供多枚模板引用同一行，
//! 改一处多处生效，而生效只影响新签合同。字段可空性与 DDL 逐列一致。
//!
//! 表由 `migration/src/domain/contract_template_schema/m0115_create_clause_library.rs` 建立。
//! `clause_code` 全表唯一且取值落在 `models::status::contract_template::contract_clause_code` 的闭合集内，
//! `body` NOT NULL 且库侧 CHECK 拒绝空串，不给自称是条款而正文为空的行落地的通道。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 条款库 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default, Serialize, Deserialize)]
#[sea_orm(table_name = "clause_library")]
pub struct Model {
    /// 条款主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 条款码，全表唯一，取值见 `models::status::contract_template::contract_clause_code`
    pub clause_code: String,

    /// 条款标题
    pub title: String,

    /// 条款正文，含 `{{占位符}}`，由模板内容域写入，建表迁移不播种
    pub body: String,

    /// 专章条款适用的合同类别，空值表示通用条款
    pub applies_contract_type: Option<String>,

    /// 条款生命周期，取值见 `models::status::contract_template::contract_template_status`
    pub status: String,

    /// 备注
    pub remark: Option<String>,

    /// 创建人用户主键
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

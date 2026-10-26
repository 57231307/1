#![allow(dead_code)]
//! 合同模板 Entity
//!
//! 映射 `contract_templates` 表，存合同模板的类别、我方立场、版本号、标题、生命周期状态、
//! 生效日与创建与法务复核留痕。字段可空性与 DDL 逐列一致，NOT NULL 列为非 Option，
//! 可空列为 Option，时间列为 TIMESTAMPTZ。
//!
//! 表由 `migration/src/domain/contract_template_schema/m0114_create_contract_templates.rs` 建立。
//! 同一 (contract_type, party_side) 同时只允许一行为 active，由部分唯一索引
//! `uq_contract_templates_active_type_side` 在库侧钉住；status 置 active 时 approved_by 不得为空。

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 合同模板 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default, Serialize, Deserialize)]
#[sea_orm(table_name = "contract_templates")]
pub struct Model {
    /// 模板主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 模板业务编码，与 version 组合唯一，改版沿用同码递增版本
    pub template_code: String,

    /// 合同类别，取值见 `models::status::contract_type`
    pub contract_type: String,

    /// 我方立场，取值见 `models::status::contract_party_side`
    pub party_side: String,

    /// 模板版本号，签约固化时随条款正文一起写进合同条款快照
    pub version: i32,

    /// 模板标题，合同表头与打印抬头展示用
    pub title: String,

    /// 生命周期状态，取值见 `models::status::contract_template_status`
    pub status: String,

    /// 模板生效日，空值表示尚未定档生效
    pub effective_from: Option<NaiveDate>,

    /// 创建人用户主键
    pub created_by: i32,

    /// 法务复核通过人，status 为 active 时不得为空
    pub approved_by: Option<i32>,

    /// 法务复核通过时间
    pub approved_at: Option<DateTime<Utc>>,

    /// 备注
    pub remark: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

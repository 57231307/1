#![allow(dead_code)]
//! 合同条款快照 Entity
//!
//! 映射 `contract_clauses` 表，存签约那一刻固化到合同实例的条款正文原文、标题、顺序、
//! 来源模板与模板版本号。本表是“约定内容”的唯一真相源，合同详情面、打印面的条款摘要区
//! 都只读本表，不回读模板条款项。
//!
//! 表由 `migration/src/domain/contract_template_schema/m0117_create_contract_clauses.rs` 建立，
//! 来源指针列 `template_clause_id` 由同域 `m0119_add_contract_clause_snapshot_provenance.rs` 追加。
//! 为什么必须存固化原文而不只存 template_id 参见该迁移头注，模板改版不得追溯改变已签合同的约定内容。
//! 本 Entity 与模板条款项之间**没有** Relation，也没有外键，建了就等于让快照跟着模板行走，
//! 正是本表要消除的追溯改写。`template_clause_id` 只是溯源指针：新固化行必写，
//! 列可由 (template_id, clause_code) 单义回查必填标记；历史行为空值，读侧对空值走
//! 唯一行回落、多义时如实返回空值，不伪造。`contract_kind` 与 `contract_id` 组合指向
//! 两侧合同主表之一，两族主键各自独立自增，故该引用是多态形态，不建外键，也不建 Relation。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 合同条款快照 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default, Serialize, Deserialize)]
#[sea_orm(table_name = "contract_clauses")]
pub struct Model {
    /// 快照行主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 合同侧别，取值见 `models::status::contract_template::contract_side_kind`
    pub contract_kind: String,

    /// 所属合同主键，与 `contract_kind` 组合定位
    pub contract_id: i32,

    /// 条款码，取值见 `models::status::contract_template::contract_clause_code`
    pub clause_code: String,

    /// 固化当时的条款标题，模板后续改名不回溯改写本列
    pub title: String,

    /// 固化当时的条款正文成品文本，占位符已填。库侧 CHECK 拒空串并拒残留占位符起始串
    pub body_rendered: String,

    /// 来源模板主键，外键指向 `contract_templates.id`
    pub template_id: i32,

    /// 固化当时的模板版本号，与 `template_id` 一起回答当时签的是哪一版
    pub template_version: i32,

    /// 固化当时的条款顺序，模板后续调序不回溯改写本列
    pub sort: i16,

    /// 固化当时引用的模板条款项行主键（m0119 追加，可空）。新固化行必写，
    /// 历史行为空值；详情腿据此回查条款项取必填标记
    pub template_clause_id: Option<i32>,

    /// 创建时间，即固化时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 来源模板（template_id 指向 contract_templates.id）
    #[sea_orm(
        belongs_to = "super::contract_template::Entity",
        from = "Column::TemplateId",
        to = "super::contract_template::Column::Id"
    )]
    Template,
}

impl ActiveModelBehavior for ActiveModel {}

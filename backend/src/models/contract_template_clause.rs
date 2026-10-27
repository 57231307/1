#![allow(dead_code)]
//! 模板条款项 Entity
//!
//! 映射 `contract_template_clauses` 表，存一枚模板下的条款项：条款库引用或模板内自带正文、
//! 顺序、必填标记、条件出条款标记与逐条法务复核留痕。字段可空性与 DDL 逐列一致。
//!
//! 表由 `migration/src/domain/contract_template_schema/m0116_create_contract_template_clauses.rs` 建立。
//! `library_clause_id` 与 `body` 恰好一个非空，由库侧 CHECK `chk_ctc_text_source` 钉住，
//! 两者都空或两者都给都写不进库。同模板同条款码同适用条件的位置唯一由
//! `uq_ctc_template_clause_position` 钉住。法务复核状态非 pending 时复核人不得为空。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 模板条款项 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default, Serialize, Deserialize)]
#[sea_orm(table_name = "contract_template_clauses")]
pub struct Model {
    /// 条款项主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 所属模板主键，外键指向 `contract_templates.id`，删除被签约引用的模板被库侧拒绝
    pub template_id: i32,

    /// 条款码，取值见 `models::status::contract_template::contract_clause_code`
    pub clause_code: String,

    /// 条款标题，模板侧展示名，可与条款库标题不同
    pub title: String,

    /// 模板内自带正文，用于不复用条款库的定制条款，与 `library_clause_id` 恰好一个非空
    pub body: Option<String>,

    /// 引用的条款库行主键，与 `body` 恰好一个非空
    pub library_clause_id: Option<i32>,

    /// 必填标记，置真的条款在合同录入面不可删
    pub required: bool,

    /// 条款顺序，签约固化时随正文一起写进合同条款快照
    pub sort: i16,

    /// 条件出条款标记，空值表示通用条款，取值见 `models::status::contract_template::contract_applies_when`
    pub applies_when: Option<String>,

    /// 逐条法务复核状态，取值见 `models::status::contract_template::contract_legal_review_status`
    pub legal_review_status: String,

    /// 复核人用户主键，复核状态非 pending 时不得为空
    pub legal_reviewed_by: Option<i32>,

    /// 复核时间
    pub legal_reviewed_at: Option<DateTime<Utc>>,

    /// 复核意见
    pub legal_review_remark: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 所属模板（template_id 指向 contract_templates.id）
    #[sea_orm(
        belongs_to = "super::contract_template::Entity",
        from = "Column::TemplateId",
        to = "super::contract_template::Column::Id"
    )]
    Template,

    /// 引用的条款库行（library_clause_id 指向 clause_library.id）
    #[sea_orm(
        belongs_to = "super::clause_library::Entity",
        from = "Column::LibraryClauseId",
        to = "super::clause_library::Column::Id"
    )]
    LibraryClause,
}

impl ActiveModelBehavior for ActiveModel {}

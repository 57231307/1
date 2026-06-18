#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 销售报价单贸易条款实体
//!
//! 存储报价单中各类贸易条款：物流（logistics）/付款（payment）/样品（sample）/检验（inspection）。
//! 关联计划：[2026-06-17-p12-batch1-quotation-port-plan.md](../../../../../docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md) PR-1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售报价单贸易条款实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_quotation_terms")]
pub struct Model {
    /// 条款 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 报价单 ID（外键 sales_quotations.id，级联删除）
    pub quotation_id: i32,
    /// 条款类型（logistics/payment/sample/inspection）
    pub term_type: String,
    /// 条款键
    pub term_key: String,
    /// 条款值
    pub term_value: String,
    /// 排序号
    pub sequence: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_quotation::Entity",
        from = "Column::QuotationId",
        to = "super::sales_quotation::Column::Id"
    )]
    Quotation,
}

impl Related<super::sales_quotation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quotation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

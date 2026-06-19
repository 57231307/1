#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售报价单贸易条款实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_quotation_terms")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub quotation_id: i64,
    pub term_type: String,
    pub term_key: String,
    pub term_value: String,
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

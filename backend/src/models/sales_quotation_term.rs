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

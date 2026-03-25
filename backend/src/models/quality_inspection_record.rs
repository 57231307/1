use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quality_inspection_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "inspection_no")]
    pub inspection_no: String,
    #[sea_orm(column_name = "inspection_type")]
    pub inspection_type: String,
    #[sea_orm(column_name = "related_type")]
    pub related_type: Option<String>,
    #[sea_orm(column_name = "related_id")]
    pub related_id: Option<i32>,
    #[sea_orm(column_name = "product_id")]
    pub product_id: i32,
    #[sea_orm(column_name = "batch_no")]
    pub batch_no: Option<String>,
    #[sea_orm(column_name = "supplier_id")]
    pub supplier_id: Option<i32>,
    #[sea_orm(column_name = "customer_id")]
    pub customer_id: Option<i32>,
    #[sea_orm(column_name = "inspection_date")]
    pub inspection_date: chrono::NaiveDate,
    #[sea_orm(column_name = "inspector_id")]
    pub inspector_id: Option<i32>,
    #[sea_orm(column_name = "total_qty")]
    pub total_qty: Decimal,
    #[sea_orm(column_name = "inspected_qty")]
    pub inspected_qty: Decimal,
    #[sea_orm(column_name = "qualified_qty")]
    pub qualified_qty: Option<Decimal>,
    #[sea_orm(column_name = "unqualified_qty")]
    pub unqualified_qty: Option<Decimal>,
    #[sea_orm(column_name = "qualification_rate")]
    pub qualification_rate: Option<Decimal>,
    #[sea_orm(column_name = "inspection_result")]
    pub inspection_result: String,
    pub remark: Option<String>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: chrono::NaiveDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use rust_decimal::Decimal;
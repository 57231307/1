//! 缸号管理模型（染色批次管理）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dye_batch")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub batch_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<Decimal>,
    pub status: String,
    pub production_date: Option<DateTimeUtc>,
    pub completion_date: Option<DateTimeUtc>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

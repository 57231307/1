#![allow(dead_code)]

//! 缸号映射模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dye_lot_mapping")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub dye_batch_id: i32,
    pub lot_no: String,
    pub is_deleted: bool,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

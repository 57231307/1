use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_dict")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub dict_type: String, // e.g., "FABRIC_TYPE", "PRINT_TEMPLATE"
    pub dict_label: String, // e.g., "针织", "默认A4"
    pub dict_value: String, // e.g., "KNIT", "A4_DEFAULT"
    pub is_default: bool,
    pub status: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

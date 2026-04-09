use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "color_card_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub color_card_id: i32,
    pub customer_id: i32,
    pub issue_date: DateTime<Utc>,
    pub return_date: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::color_card::Entity",
        from = "Column::ColorCardId",
        to = "super::color_card::Column::Id"
    )]
    ColorCard,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::color_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ColorCard.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

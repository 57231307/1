#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 定制订单主表实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "custom_orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub order_no: String,
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub spec: String,
    pub quantity: Decimal,
    pub unit: String,
    pub custom_requirements: Json,
    pub yarn_spec: Option<String>,
    pub dye_method: Option<String>,
    pub finishing_method: Option<String>,
    pub status: String,
    pub expected_delivery_date: Option<NaiveDate>,
    pub actual_delivery_date: Option<NaiveDate>,
    pub sales_order_id: Option<i64>,
    pub total_amount: Option<Decimal>,
    pub currency: String,
    pub tenant_id: i64,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::process_node::Entity")]
    ProcessNodes,
    #[sea_orm(has_many = "super::quality_issue::Entity")]
    QualityIssues,
    #[sea_orm(has_many = "super::after_sales::Entity")]
    AfterSalesList,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::product_color::Entity",
        from = "Column::ColorId",
        to = "super::product_color::Column::Id"
    )]
    ProductColor,
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::SalesOrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
}

impl Related<super::process_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessNodes.def()
    }
}

impl Related<super::quality_issue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QualityIssues.def()
    }
}

impl Related<super::after_sales::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AfterSalesList.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

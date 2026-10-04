#![allow(dead_code)]
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_prices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Decimal,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Decimal,
    pub price_type: String,
    pub price_level: Option<String>,
    #[sea_orm(column_type = "Date")]
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联产品（product_id -> products.id）：仅供列表 `keyword` 按"产品名称"过滤时
    /// LeftJoin 使用（照 purchase_price.rs:31-38 先例）；不追加 SELECT 列，响应仍为整 Model。
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,

    /// 关联客户（customer_id -> customers.id）：仅供 `keyword` 按"客户名称"过滤时
    /// LeftJoin 使用；标准价行 customer_id 为 NULL，LEFT JOIN 下该行仅不被客户名命中。
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl ActiveModelBehavior for ActiveModel {}

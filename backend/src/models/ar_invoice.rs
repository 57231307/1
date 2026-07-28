//! 应收单 Entity
//!
//! 对应数据库表：ar_invoices

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_invoices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub invoice_no: String,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,

    // 客户信息
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub customer_code: Option<String>,

    // 来源单据
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,

    // 面料行业字段
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub sales_order_no: Option<String>,

    // 金额
    pub invoice_amount: Decimal,
    pub received_amount: Decimal,
    pub unpaid_amount: Decimal,
    pub tax_amount: Option<Decimal>,

    // 双计量单位
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub unit_price: Option<Decimal>,

    // 状态
    pub status: String,
    pub approval_status: String,

    // 审核
    pub created_by: i32,
    pub reviewed_by: Option<i32>,
    pub reviewed_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

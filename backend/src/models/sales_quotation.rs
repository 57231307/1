#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售报价单主表实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_quotations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub quotation_no: String,
    pub customer_id: i64,
    pub sales_user_id: i64,
    pub quotation_date: NaiveDate,
    pub valid_until: NaiveDate,

    /// 货币
    pub currency: String,
    pub exchange_rate: Decimal,
    pub base_currency: String,

    /// 价格条款（Incoterms 2020）
    pub price_terms: String,
    pub incoterms_version: Option<String>,
    pub incoterm_location: Option<String>,

    /// 税务
    pub tax_inclusive: bool,
    pub tax_rate: Decimal,

    /// 业务参数
    pub moq: Option<Decimal>,
    pub lead_time_days: Option<i32>,
    pub customer_level: Option<String>,

    /// 金额
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,

    /// 状态
    pub status: String,

    /// BPM 审批
    pub approval_instance_id: Option<i64>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    /// 转换
    pub converted_sales_order_id: Option<i64>,
    pub converted_at: Option<DateTime<Utc>>,

    /// 元数据
    pub notes: Option<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sales_quotation_item::Entity")]
    Items,
    #[sea_orm(has_many = "super::sales_quotation_term::Entity")]
    Terms,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::SalesUserId",
        to = "super::user::Column::Id"
    )]
    SalesUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::ConvertedSalesOrderId",
        to = "super::sales_order::Column::Id"
    )]
    ConvertedSalesOrder,
}

impl Related<super::sales_quotation_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::sales_quotation_term::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Terms.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesUser.def()
    }
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConvertedSalesOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

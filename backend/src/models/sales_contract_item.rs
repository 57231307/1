#![allow(dead_code)]
//! 销售合同明细行 Entity
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_contract_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 合同 ID
    pub contract_id: i32,
    /// 产品 ID
    pub product_id: Option<i32>,
    /// 产品名称
    pub product_name: String,
    /// 产品规格
    pub product_spec: Option<String>,
    /// 单位
    pub unit: String,
    /// 数量
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub quantity: Decimal,
    /// 单价
    #[sea_orm(column_type = "Decimal(Some((15, 4)))")]
    pub unit_price: Decimal,
    /// 金额
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub amount: Decimal,
    /// 交货日期
    pub delivery_date: Option<NaiveDate>,
    /// 备注
    pub remarks: Option<String>,
    /// 排序
    pub sort_order: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 明细行 - 合同（多对一）
    #[sea_orm(
        belongs_to = "super::sales_contract::Entity",
        from = "Column::ContractId",
        to = "super::sales_contract::Column::Id"
    )]
    SalesContract,
}

impl Related<super::sales_contract::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesContract.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#![allow(dead_code, unused_imports, unused_variables)]
//! 采购合同执行 Model
//!
//! 采购合同执行模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 采购合同执行 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_contract_executions")]
pub struct Model {
    /// 执行单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 执行单号
    #[sea_orm(unique)]
    pub execution_no: String,

    /// 采购合同 ID（外键）
    pub contract_id: i32,

    /// 执行日期
    pub execution_date: NaiveDate,

    /// 执行类型：PARTIAL=部分执行，COMPLETE=完成执行
    pub execution_type: String,

    /// 执行数量
    pub quantity: Decimal,

    /// 执行金额
    pub amount: Decimal,

    /// 执行状态：DRAFT=草稿，CONFIRMED=已确认，COMPLETED=已完成
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 采购合同执行关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 执行 - 采购合同（多对一）
    #[sea_orm(
        belongs_to = "super::purchase_contract::Entity",
        from = "Column::ContractId",
        to = "super::purchase_contract::Column::Id"
    )]
    PurchaseContract,
}

impl Related<super::purchase_contract::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseContract.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

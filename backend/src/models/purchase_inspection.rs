//! 采购质检 Model
//!
//! 采购质检模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 采购质检 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_inspections")]
pub struct Model {
    /// 质检单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 质检单号
    #[sea_orm(unique)]
    pub inspection_no: String,

    /// 采购订单 ID（外键）
    pub purchase_order_id: Option<i32>,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 质检日期
    pub inspection_date: NaiveDate,

    /// 质检员 ID（外键）
    pub inspector_id: Option<i32>,

    /// 质检结果：PENDING=待检，PASSED=合格，FAILED=不合格
    pub result: String,

    /// 合格数量
    pub qualified_quantity: Decimal,

    /// 不合格数量
    pub unqualified_quantity: Decimal,

    /// 不合格原因
    pub unqualified_reason: Option<String>,

    /// 质检类型
    pub inspection_type: Option<String>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 采购质检关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 采购质检 - 供应商（多对一）
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,

    /// 采购质检 - 用户（质检员，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::InspectorId",
        to = "super::user::Column::Id"
    )]
    Inspector,

    /// 采购质检 - 用户（创建人，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,

    /// 采购质检 - 采购订单（多对一）
    #[sea_orm(
        belongs_to = "super::purchase_order::Entity",
        from = "Column::PurchaseOrderId",
        to = "super::purchase_order::Column::Id"
    )]
    PurchaseOrder,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inspector.def()
    }
}

impl Related<super::purchase_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

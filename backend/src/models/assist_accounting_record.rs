//! 辅助核算记录 Model
//!
//! 记录每笔业务的辅助核算信息

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// 辅助核算记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "assist_accounting_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 业务类型：PURCHASE, SALES, INVENTORY, PRODUCTION
    pub business_type: String,

    /// 业务单号
    pub business_no: String,

    /// 业务单 ID
    pub business_id: i32,

    /// 会计科目 ID
    pub account_subject_id: i32,

    /// 借方金额
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub debit_amount: Decimal,

    /// 贷方金额
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub credit_amount: Decimal,

    /// 五维 ID
    pub five_dimension_id: String,

    /// 产品 ID
    pub product_id: i32,

    /// 批次号
    pub batch_no: String,

    /// 色号
    pub color_no: String,

    /// 缸号
    pub dye_lot_no: Option<String>,

    /// 等级
    pub grade: String,

    /// 车间 ID（可选）
    pub workshop_id: Option<i32>,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 客户 ID（可选）
    pub customer_id: Option<i32>,

    /// 供应商 ID（可选）
    pub supplier_id: Option<i32>,

    /// 数量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_meters: Decimal,

    /// 数量（公斤）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_kg: Decimal,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 创建人 ID
    pub created_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account_subject::Entity",
        from = "Column::AccountSubjectId",
        to = "super::account_subject::Column::Id"
    )]
    AccountSubject,
}

impl Related<super::account_subject::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountSubject.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

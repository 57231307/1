#![allow(dead_code)]
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_inspection")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub inspection_no: String,
    pub receipt_id: Option<i32>,
    pub order_id: Option<i32>,
    pub supplier_id: i32,
    pub inspection_date: NaiveDate,
    pub inspector_id: Option<i32>,
    pub inspection_type: Option<String>,
    pub sample_size: Option<Decimal>,
    pub defect_count: Option<i32>,
    pub pass_quantity: Option<Decimal>,
    pub reject_quantity: Option<Decimal>,
    pub inspection_status: Option<String>,
    pub inspection_result: Option<String>,
    pub quality_score: Option<Decimal>,
    pub defect_description: Option<String>,
    pub attachment_urls: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联入库单（receipt_id -> purchase_receipts.id）
    #[sea_orm(
        belongs_to = "super::purchase_receipt::Entity",
        from = "Column::ReceiptId",
        to = "super::purchase_receipt::Column::Id"
    )]
    Receipt,

    /// 关联供应商（supplier_id -> suppliers.id）
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,

    /// 关联质检员（inspector_id -> users.id）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::InspectorId",
        to = "super::user::Column::Id"
    )]
    Inspector,
}

impl ActiveModelBehavior for ActiveModel {}

/// 采购质检列表页统计卡聚合（GET /purchase/inspections/stats 出参）。
///
/// 四键与列表端点**同一套筛选条件、同一条基础查询**构造（见
/// `services/purchase_inspection_service.rs::base_filtered_query`），
/// 保证分母同源；分桶取值全部引用权威词表常量
/// `models::status::purchase_inventory::{purchase_inspection, purchase_inspection_result}`，
/// 与写入侧逐字符同源，禁止内联字符串。
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseInspectionStats {
    /// 与列表同筛选条件下的全量行数（分页不影响）
    pub total: u64,
    /// inspection_status = PENDING 的行数
    pub pending: u64,
    /// inspection_result = PASS 的行数
    pub passed: u64,
    /// inspection_result IN (FAIL, PARTIAL) 的行数
    /// （partial 属不合格侧：既有裁定 `to_receipt_inspection_status` 把
    /// fail/partial 一并回写为入库单 REJECTED，统计分桶与之同源）
    pub failed: u64,
}

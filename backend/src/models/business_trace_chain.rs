//! 业务追溯链模型

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

/// 业务追溯链 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "business_trace_chain")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 追溯链 ID（全局唯一）
    pub trace_chain_id: String,

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

    /// 当前环节类型
    pub current_stage: String,

    /// 当前环节业务单类型
    pub current_bill_type: String,

    /// 当前环节业务单号
    pub current_bill_no: String,

    /// 当前环节业务单 ID
    pub current_bill_id: i32,

    /// 上一环节 ID
    pub previous_trace_id: Option<i32>,

    /// 下一环节 ID
    pub next_trace_id: Option<i32>,

    /// 数量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_meters: Decimal,

    /// 数量（公斤）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_kg: Decimal,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 供应商 ID（采购环节）
    pub supplier_id: Option<i32>,

    /// 客户 ID（销售环节）
    pub customer_id: Option<i32>,

    /// 车间 ID（生产环节）
    pub workshop_id: Option<i32>,

    /// 追溯状态：ACTIVE - 活跃，COMPLETED - 已完成，SUSPENDED - 暂停
    pub trace_status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 创建人 ID
    pub created_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

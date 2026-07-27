#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
//! 批次追溯日志 Model
//!
//! 批次追溯日志模块（V15 P1 扩展：dye_lot_no/color_no/product_id 字段 + 全链路 operation_type）

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 批次追溯日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "batch_trace_log")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号
    pub batch_no: String,

    /// V15 P1-2: 染色批号（dye_lot_no），面料行业四维标识之一
    pub dye_lot_no: Option<String>,

    /// V15 P1-2: 色号（color_no），按色号追溯
    pub color_no: Option<String>,

    /// V15 P1-2: 产品 ID（product_id），按产品追溯
    pub product_id: Option<i32>,

    /// 操作类型：CREATE/DYE/INSPECT/GRADE/SHIP/REWORK/TRANSFER/ADJUST/MERGE/SPLIT
    pub operation_type: String,

    /// 源单据类型
    pub source_type: Option<String>,

    /// 源单据 ID
    pub source_id: Option<i32>,

    /// 源单据号
    pub source_no: Option<String>,

    /// 操作数量
    pub quantity: Option<rust_decimal::Decimal>,

    /// 操作前库存
    pub quantity_before: Option<rust_decimal::Decimal>,

    /// 操作后库存
    pub quantity_after: Option<rust_decimal::Decimal>,

    /// V15 P1-2: 流转前状态（from_status）
    pub from_status: Option<String>,

    /// V15 P1-2: 流转后状态（to_status）
    pub to_status: Option<String>,

    /// 备注
    pub remarks: Option<String>,

    /// 操作人 ID
    pub operated_by: Option<i32>,

    /// 操作时间
    pub operated_at: DateTime<Utc>,
}

/// 批次追溯日志关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

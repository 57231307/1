#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quality_inspection_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "inspection_no")]
    pub inspection_no: String,
    #[sea_orm(column_name = "inspection_type")]
    pub inspection_type: String,
    #[sea_orm(column_name = "related_type")]
    pub related_type: Option<String>,
    #[sea_orm(column_name = "related_id")]
    pub related_id: Option<i32>,
    #[sea_orm(column_name = "product_id")]
    pub product_id: i32,
    #[sea_orm(column_name = "batch_no")]
    pub batch_no: Option<String>,
    #[sea_orm(column_name = "supplier_id")]
    pub supplier_id: Option<i32>,
    #[sea_orm(column_name = "customer_id")]
    pub customer_id: Option<i32>,
    #[sea_orm(column_name = "inspection_date")]
    pub inspection_date: chrono::NaiveDate,
    #[sea_orm(column_name = "inspector_id")]
    pub inspector_id: Option<i32>,
    #[sea_orm(column_name = "total_qty")]
    pub total_qty: Decimal,
    #[sea_orm(column_name = "inspected_qty")]
    pub inspected_qty: Decimal,
    #[sea_orm(column_name = "qualified_qty")]
    pub qualified_qty: Option<Decimal>,
    #[sea_orm(column_name = "unqualified_qty")]
    pub unqualified_qty: Option<Decimal>,
    #[sea_orm(column_name = "qualification_rate")]
    pub qualification_rate: Option<Decimal>,
    #[sea_orm(column_name = "inspection_result")]
    pub inspection_result: String,
    pub remark: Option<String>,
    // v14 批次 421 T-P1-4：面料行业质检分级 A 级（合格）/B 级（让步接收，降级销售）/C 级（不合格，返工或报废）
    // 依据：fabric-industry-research.md §4.7 质量检验模块
    #[sea_orm(column_name = "grade")]
    pub grade: Option<String>,
    // v14 批次 421：按缸号追溯质检结果，依据 fabric-industry-research.md §2.1 四层级联关系
    #[sea_orm(column_name = "color_no")]
    pub color_no: Option<String>,
    #[sea_orm(column_name = "dye_lot_no")]
    pub dye_lot_no: Option<String>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use rust_decimal::Decimal;

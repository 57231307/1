//! 业务追溯快照模型

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value as Json;

/// 业务追溯快照 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "business_trace_snapshot")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    
    /// 追溯链 ID
    pub trace_chain_id: String,
    
    /// 五维 ID
    pub five_dimension_id: String,
    
    /// 产品 ID
    pub product_id: i32,
    
    /// 批次号
    pub batch_no: String,
    
    /// 色号
    pub color_no: String,
    
    /// 等级
    pub grade: String,
    
    /// 当前环节
    pub current_stage: String,
    
    /// 当前仓库 ID
    pub warehouse_id: i32,
    
    /// 当前数量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub current_quantity_meters: Decimal,
    
    /// 当前数量（公斤）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub current_quantity_kg: Decimal,
    
    /// 供应商名称
    pub supplier_name: Option<String>,
    
    /// 客户名称
    pub customer_name: Option<String>,
    
    /// 追溯路径（JSON 数组）
    pub trace_path: Json,
    
    /// 快照时间
    pub snapshot_time: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

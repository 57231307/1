//! 业务追溯查询视图模型

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// 业务追溯视图 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "v_business_trace_view")]
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
    
    /// 起始环节（采购）
    pub start_stage: String,
    
    /// 当前环节
    pub current_stage: String,
    
    /// 环节数量
    pub stage_count: i32,
    
    /// 总入库米数
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_in_meters: Decimal,
    
    /// 总出库米数
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_out_meters: Decimal,
    
    /// 当前库存米数
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub current_stock_meters: Decimal,
    
    /// 供应商名称
    pub supplier_name: Option<String>,
    
    /// 客户名称
    pub customer_name: Option<String>,
    
    /// 创建时间
    pub created_at: DateTime<Utc>,
    
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

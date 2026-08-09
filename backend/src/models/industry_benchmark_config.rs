use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 行业基准配置实体 - batch-15 P3: 行业基准配置化
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "industry_benchmark_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 基准名称（如：纺织行业平均、化纤行业平均）
    pub benchmark_name: String,
    /// 行业类型
    pub industry_type: String,
    /// 指标名称（如：流动比率、速动比率、资产负债率）
    pub metric_name: String,
    /// 指标值
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub metric_value: rust_decimal::Decimal,
    /// 单位
    pub unit: Option<String>,
    /// 数据来源
    pub data_source: Option<String>,
    /// 数据年份
    pub data_year: Option<i32>,
    /// 是否启用
    pub is_active: bool,
    /// 备注
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 创建行业基准 DTO
#[derive(Deserialize)]
pub struct CreateIndustryBenchmarkDto {
    pub benchmark_name: String,
    pub industry_type: String,
    pub metric_name: String,
    pub metric_value: rust_decimal::Decimal,
    pub unit: Option<String>,
    pub data_source: Option<String>,
    pub data_year: Option<i32>,
    pub is_active: Option<bool>,
    pub remark: Option<String>,
}

/// 更新行业基准 DTO
#[derive(Deserialize)]
pub struct UpdateIndustryBenchmarkDto {
    pub benchmark_name: Option<String>,
    pub industry_type: Option<String>,
    pub metric_name: Option<String>,
    pub metric_value: Option<rust_decimal::Decimal>,
    pub unit: Option<String>,
    pub data_source: Option<String>,
    pub data_year: Option<i32>,
    pub is_active: Option<bool>,
    pub remark: Option<String>,
}

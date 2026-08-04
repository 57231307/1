#![allow(dead_code)]
//! 报表模板 Model
//!
//! 存储用户自定义的报表模板配置

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 报表模板 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "report_templates")]
pub struct Model {
    /// 模板 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 模板名称
    pub name: String,

    /// 模板编码
    pub code: String,

    /// 报表类型（sales/purchase/inventory/financial/custom）
    pub report_type: String,

    /// 业务模板 ID（兼容旧报表引擎，UUID 或语义化字符串）
    pub template_id: Option<String>,

    /// 模板分类
    pub category: Option<String>,

    /// 数据源标识
    pub data_source: Option<String>,

    /// 列定义（JSON格式）
    pub columns: Json,

    /// 筛选条件（JSON格式）
    pub filters: Option<Json>,

    /// 报表参数（JSON格式）
    pub parameters: Option<Json>,

    /// 支持的导出格式（JSON格式）
    pub supported_formats: Option<Json>,

    /// 排序字段
    pub sort_by: Option<String>,

    /// 排序方式（asc/desc）
    pub sort_order: Option<String>,

    /// 数据源SQL（自定义报表使用）
    pub data_source_sql: Option<String>,

    /// 描述
    pub description: Option<String>,

    /// 是否公开（true=所有用户可见，false=仅创建者可见）
    pub is_public: bool,

    /// 状态（ACTIVE/INACTIVE）
    pub status: String,

    /// 模板版本号（缺陷 1.1 修复：每次 update 前递增，支持回滚到历史版本）
    pub version: i32,

    /// 必需权限码（缺陷 1.2 修复：如 report:sales:view；为空表示按 is_public/created_by 过滤）
    pub required_permission: Option<String>,

    /// 刷新策略（REALTIME/HOURLY/DAILY，缺陷 1.3 修复）
    pub refresh_strategy: Option<String>,

    /// 缓存 TTL 秒数（缺陷 1.3 修复：execute_custom_report 根据策略选择是否走缓存）
    pub cache_ttl_seconds: Option<i32>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 报表模板关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

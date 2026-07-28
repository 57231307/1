//! 报表模板历史版本 Model
//!
//! 缺陷 1.1 修复：存储报表模板的历史版本快照，支持回滚到旧版本

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 报表模板历史版本 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "report_template_versions")]
pub struct Model {
    /// 版本记录 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联的报表模板 ID
    pub template_id: i32,

    /// 版本号（与 report_templates.version 对应）
    pub version: i32,

    /// 模板名称（快照）
    pub name: String,

    /// 模板编码（快照）
    pub code: String,

    /// 报表类型（快照）
    pub report_type: String,

    /// 模板分类（快照）
    pub category: Option<String>,

    /// 数据源标识（快照）
    pub data_source: Option<String>,

    /// 列定义 JSON（快照）
    pub columns: Json,

    /// 筛选条件 JSON（快照）
    pub filters: Option<Json>,

    /// 报表参数 JSON（快照）
    pub parameters: Option<Json>,

    /// 支持的导出格式 JSON（快照）
    pub supported_formats: Option<Json>,

    /// 排序字段（快照）
    pub sort_by: Option<String>,

    /// 排序方式（快照）
    pub sort_order: Option<String>,

    /// 数据源 SQL（快照，保留历史）
    pub data_source_sql: Option<String>,

    /// 描述（快照）
    pub description: Option<String>,

    /// 是否公开（快照）
    pub is_public: bool,

    /// 必需权限码（快照）
    pub required_permission: Option<String>,

    /// 快照创建者 ID（即当时执行 update 的用户）
    pub snapshot_by: i32,

    /// 快照时间（即 update 前的写入时间）
    pub snapshot_at: DateTime<Utc>,
}

/// 报表模板版本关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::report_template::Entity",
        from = "Column::TemplateId",
        to = "super::report_template::Column::Id"
    )]
    ReportTemplate,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::SnapshotBy",
        to = "super::user::Column::Id"
    )]
    SnapshotBy,
}

impl Related<super::report_template::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReportTemplate.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SnapshotBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

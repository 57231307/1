//! 催收模板 Model（V15 P1 17.3-D5）
//!
//! 表 collection_templates：催收话术模板，按催收类型/逾期阶段配置标准化话术
//! 用于催收任务创建时自动填充话术内容，提升催收标准化程度

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 催收模板 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "collection_templates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 模板名称
    pub name: String,
    /// 催收类型：phone / visit / email / letter
    pub task_type: String,
    /// 适用逾期阶段：early(0-30天) / middle(31-90天) / late(90+天) / all
    pub overdue_stage: String,
    /// 话术标题/主题（邮件/函件场景使用）
    pub title: Option<String>,
    /// 话术正文
    pub content: String,
    /// 是否启用
    pub is_enabled: bool,
    /// 排序（同类型同阶段按 sort_order 升序）
    pub sort_order: i32,
    /// 备注
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

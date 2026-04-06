//! OA 公告 Model
//!
//! OA 公告模块

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// OA 公告 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oa_announcement")]
pub struct Model {
    /// 公告 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 公告标题
    pub title: String,

    /// 公告内容
    pub content: String,

    /// 公告类型：NOTICE=通知，ANNOUNCEMENT=公告，NEWS=新闻
    pub announcement_type: String,

    /// 发布日期
    pub publish_date: NaiveDate,

    /// 生效日期
    pub effective_date: NaiveDate,

    /// 失效日期
    pub expiry_date: Option<NaiveDate>,

    /// 发布人 ID
    pub publisher_id: i32,

    /// 状态：DRAFT=草稿，PUBLISHED=已发布，ARCHIVED=已归档
    pub status: String,

    /// 置顶
    pub is_top: bool,

    /// 附件
    pub attachments: Option<serde_json::Value>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// OA 公告关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

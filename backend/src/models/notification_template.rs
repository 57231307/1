//! 通知模板模型（notification_templates 表）
//!
//! batch-16 P2-3：通知模板动态管理

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 通知模板模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notification_templates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 模板编码（唯一）
    #[sea_orm(unique)]
    pub code: String,

    /// 模板名称
    pub name: String,

    /// 模板类型（email/sms/system）
    pub template_type: String,

    /// 通知标题模板（支持变量替换）
    pub title_template: String,

    /// 通知内容模板（支持变量替换）
    pub content_template: String,

    /// 语言（zh-CN/en-US）
    pub language: String,

    /// 是否启用
    pub is_active: bool,

    /// 备注
    pub remarks: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

//! 用户隐私同意记录 Model（V15 P1 batch-16 缺陷 7.3）
//!
//! 表 user_consents：记录用户对行为追踪/页面访问/Cookie/营销邮件的同意与退出
//! 每次 consent 变更新增一条记录，保留审计轨迹（GDPR / 个人信息保护法合规）

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 用户隐私同意记录 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_consents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 用户 ID
    pub user_id: i32,

    /// 同意类型：behavior_tracking / page_view_tracking / cookie_usage / marketing_email
    pub consent_type: String,

    /// 是否同意：true=同意采集，false=退出
    pub consent_given: bool,

    /// 隐私政策文本版本号（如 v1.0）
    pub consent_text_version: Option<String>,

    /// 同意时间
    pub consented_at: DateTime<Utc>,

    /// 撤回时间（同类型再次同意时旧记录会被设置为当前时间）
    pub revoked_at: Option<DateTime<Utc>>,

    /// IP 地址（审计用）
    pub ip_address: Option<String>,

    /// User-Agent（审计用）
    pub user_agent: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}

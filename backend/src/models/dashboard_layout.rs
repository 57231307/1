//! 仪表板布局配置模型
//!
//! 缺陷 4.1 修复：支持用户自定义卡片配置并持久化

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 仪表板布局配置实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "dashboard_layouts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 用户 ID（每个用户独立布局）
    pub user_id: i32,
    /// 卡片配置 JSON（卡片顺序、可见性、尺寸等）
    pub card_config: Json,
    /// 是否默认布局（true 时其他用户初始可见）
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

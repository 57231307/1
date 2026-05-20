//! 字段权限模型
//!
//! 实现字段级别的读写权限控制

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 字段权限规则实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "field_permissions")]
pub struct Model {
    /// 权限 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 角色 ID
    pub role_id: i32,
    /// 资源类型（表名或业务对象，如 "product", "sales_order"）
    pub resource_type: String,
    /// 字段名
    pub field_name: String,
    /// 是否允许读取
    pub can_read: bool,
    /// 是否允许写入
    pub can_write: bool,
    /// 掩码策略：NONE-无掩码, MASK-显示为 "***", HASH-显示哈希值
    pub mask_strategy: String,
    /// 是否启用
    pub is_enabled: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::role::Entity",
        from = "Column::RoleId",
        to = "super::role::Column::Id"
    )]
    Role,
}

impl Related<super::role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

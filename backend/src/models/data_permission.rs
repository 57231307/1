//! 数据权限模型
//!
//! 实现数据范围控制和字段级权限管理

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 数据权限规则实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "data_permissions")]
pub struct Model {
    /// 权限 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 角色 ID
    pub role_id: i32,
    /// 资源类型（表名或业务对象）
    pub resource_type: String,
    /// 数据范围类型：ALL-全部, DEPT-本部门, DEPT_AND_BELOW-本部门及以下, SELF-仅本人, CUSTOM-自定义
    pub scope_type: String,
    /// 自定义数据范围条件（JSON）
    pub custom_condition: Option<serde_json::Value>,
    /// 允许的字段列表（JSON 数组，空表示全部）
    pub allowed_fields: Option<serde_json::Value>,
    /// 隐藏的字段列表（JSON 数组）
    pub hidden_fields: Option<serde_json::Value>,
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

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.4-D5: 客户字段权限配置 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_field_permission")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 角色ID
    pub role_id: i32,

    /// 字段名称
    pub field_name: String,

    /// 权限：visible/hidden/masked
    pub permission: String,

    /// 脱敏模式
    pub mask_pattern: Option<String>,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,

    /// 更新时间
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

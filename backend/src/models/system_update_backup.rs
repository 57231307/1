//! 系统备份记录 Entity（m0055 落库，替代内存存储）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "system_update_backups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 备份编号（BK-yyyyMMddHHmmss 格式）
    pub backup_code: String,
    /// 备份类型：full/incremental/database/files
    #[sea_orm(column_default = "full")]
    pub backup_type: String,
    /// 备份文件路径
    #[sea_orm(column_default = "")]
    pub file_path: String,
    /// 文件大小（字节）
    #[sea_orm(column_default = "0")]
    pub file_size: i64,
    /// 描述
    #[sea_orm(column_default = "")]
    pub description: String,
    /// 状态：creating/completed/failed
    #[sea_orm(column_default = "creating")]
    pub status: String,
    pub created_by: i32,
    pub created_by_name: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

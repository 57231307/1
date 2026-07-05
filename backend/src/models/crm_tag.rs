//! CRM 标签字典 Model
//!
//! 批次 122 v8 复审 P1 修复：替代 crm_customer_handler list_tags 硬编码 5 个标签 +
//! create_tag/delete_tag 假实现。标签字典独立存储，支持 id/name/color/category/created_at 元数据。
//! crm_lead.tags TEXT[] 数组字段保留向后兼容（add_tags handler 仍覆盖式更新该数组）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// CRM 标签字典 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_tag")]
pub struct Model {
    /// 标签 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 标签名称（唯一，长度 1-30）
    #[sea_orm(unique)]
    pub name: String,
    /// 标签颜色（HEX 格式，默认 #1890ff）
    pub color: String,
    /// 标签分类（可选，如 customer/lead/supplier）
    pub category: Option<String>,
    /// 创建者用户 ID
    pub created_by: Option<i32>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

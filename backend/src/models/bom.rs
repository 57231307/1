#![allow(dead_code)]

//! BOM物料清单 Model
//!
//! BOM（Bill of Materials）物料清单模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BOM状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum BomStatus {
    /// 生效中
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    /// 已失效
    #[sea_orm(string_value = "INACTIVE")]
    Inactive,
    /// 审核中
    #[sea_orm(string_value = "PENDING")]
    Pending,
}

/// BOM Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "boms")]
pub struct Model {
    /// BOM ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 产品 ID
    pub product_id: i32,

    /// 版本号
    pub version: i32,

    /// 是否默认版本
    pub is_default: bool,

    /// 状态
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 是否删除
    pub is_deleted: bool,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// BOM关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// BOM - BOM明细（一对多）
    #[sea_orm(has_many = "super::bom_item::Entity")]
    BomItems,
}

impl Related<super::bom_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BomItems.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

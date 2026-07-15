//! 染化料分类模型（chemical_category 表）
//!
//! v14 批次 429：染化料主数据完善
//! 依据：面料行业真实业务调研文档 §11.4 染化料主数据管理
//! 真实业务：染料/助剂/化工原料的分类树，支持多级分类
//! 分类类型：dye(染料) / auxiliary(助剂) / chemical(化工原料)

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 染化料分类模型
///
/// 真实业务要点：
/// - 支持多级树形分类（parent_id 自引用）
/// - 三种类型：染料 / 助剂 / 化工原料
/// - 排序字段 sort_order 控制前端展示顺序
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "chemical_category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 分类编码（唯一）
    pub category_code: String,
    /// 分类名称
    pub category_name: String,
    /// 父分类 ID（NULL 表示根分类）
    pub parent_id: Option<i32>,
    /// 分类类型：dye(染料) / auxiliary(助剂) / chemical(化工原料)
    pub category_type: String,
    /// 描述
    pub description: Option<String>,
    /// 排序号
    pub sort_order: i32,
    /// 是否启用
    pub is_active: bool,
    /// 软删除
    pub is_deleted: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 一对多：染化料主数据
    #[sea_orm(has_many = "super::chemical_master::Entity")]
    Chemicals,
}

impl Related<super::chemical_master::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chemicals.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

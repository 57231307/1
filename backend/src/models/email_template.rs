#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 邮件模板 Model
//!
//! 存储邮件模板配置

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 邮件模板 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "email_templates")]
pub struct Model {
    /// 模板 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 租户 ID
    pub tenant_id: i32,

    /// 模板名称
    pub name: String,

    /// 模板编码（唯一标识）
    pub code: String,

    /// 邮件主题模板
    pub subject_template: String,

    /// 邮件正文模板（HTML格式）
    pub body_template: String,

    /// 模板类型（NOTIFICATION/WORKFLOW/ALERT/MARKETING）
    pub template_type: String,

    /// 模板变量说明（JSON格式）
    pub variables: Option<Json>,

    /// 描述
    pub description: Option<String>,

    /// 是否启用
    pub is_active: bool,

    /// 状态（ACTIVE/INACTIVE）
    pub status: String,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 邮件模板关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
}

impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

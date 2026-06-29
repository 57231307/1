#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! CRM 公海回收规则 Model
//!
//! 用于持久化 CRM 公海回收规则配置。原 v5 审计批次 23 维度 13 P0-4 修复：
//! 将原本存于 `missing_handlers.rs` 的 `static RECYCLE_RULES` 内存存储迁移至数据库，
//! 避免进程重启后丢失运行时修改。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// CRM 公海回收规则 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_recycle_rules")]
pub struct Model {
    /// 规则 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 规则名称
    pub name: String,
    /// 未跟进超过 N 天后自动回收到公海
    pub days: i32,
    /// 是否启用
    pub is_enabled: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 主备隔离状态表 Model
//!
//! 记录每个功能（database / cache）当前的主备实时状态。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 主备隔离状态表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "failover_status")]
pub struct Model {
    /// 主键
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 功能名：database / cache
    #[sea_orm(unique)]
    pub function_name: String,

    /// 当前状态：primary（主调用中）/ backup（备用中）/ both_down（双不可用）
    pub current_state: String,

    /// 熔断器状态：closed / open / half_open
    pub circuit_state: String,

    /// 主调用 URL（脱敏存储）
    pub primary_url: Option<String>,

    /// 备用类型：postgres / redis / lru
    pub backup_type: Option<String>,

    /// 最近一次切换时间
    pub last_switch_at: Option<DateTime<Utc>>,

    /// 最近一次成功调用时间
    pub last_success_at: Option<DateTime<Utc>>,

    /// 连续失败次数
    pub consecutive_failures: i32,

    /// 主调用总次数
    pub total_primary_calls: i64,

    /// 备用调用总次数
    pub total_backup_calls: i64,

    /// 切换总次数
    pub total_switches: i64,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// DTO：状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverStatusDto {
    pub function_name: String,
    pub current_state: String,
    pub circuit_state: String,
    pub primary_url: Option<String>,
    pub backup_type: Option<String>,
    pub last_switch_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i32,
    pub total_primary_calls: i64,
    pub total_backup_calls: i64,
    pub total_switches: i64,
    pub updated_at: DateTime<Utc>,
}

impl From<Model> for FailoverStatusDto {
    fn from(m: Model) -> Self {
        Self {
            function_name: m.function_name,
            current_state: m.current_state,
            circuit_state: m.circuit_state,
            primary_url: m.primary_url,
            backup_type: m.backup_type,
            last_switch_at: m.last_switch_at,
            last_success_at: m.last_success_at,
            consecutive_failures: m.consecutive_failures,
            total_primary_calls: m.total_primary_calls,
            total_backup_calls: m.total_backup_calls,
            total_switches: m.total_switches,
            updated_at: m.updated_at,
        }
    }
}

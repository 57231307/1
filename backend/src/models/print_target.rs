#![allow(dead_code)]
//! 打印目标模型
//!
//! 映射 `print_targets` 表:存储打印机与打印机池两类打印目标的符号编码、显示名、
//! 按种类选用的连接字段、池成员编码数组与缺省打印配置。字段可空性与 DDL 逐列一致
//! (NOT NULL 列为非 Option,可空列为 Option;`pool_members` 为 JSONB/Json 非 Option,
//! 时间列为 TIMESTAMPTZ/DateTimeWithTimeZone,`default_copies` 为 SMALLINT/i16,
//! `port` 为 INTEGER/i32)。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 打印目标 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "print_targets")]
pub struct Model {
    /// 目标主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 目标业务唯一符号编码(模板与任务经它引用目标)
    pub target_code: String,

    /// 目标显示名
    pub display_name: String,

    /// 打印连接种类(cups/ipp/network/system_command)
    pub kind: String,

    /// cups 种类所用的打印队列名(可空)
    pub queue_name: Option<String>,

    /// network 种类所用的主机地址(可空)
    pub host: Option<String>,

    /// network 种类所用的端口(可空,如 9100)
    pub port: Option<i32>,

    /// ipp 种类所用的 IPP 端点 URL(可空)
    pub endpoint_url: Option<String>,

    /// system_command 种类所用的占位符命令模板(可空)
    pub command_template: Option<String>,

    /// 是否打印机池(池行由成员编码数组路由到各单台目标)
    pub is_pool: bool,

    /// 池成员 target_code 字符串数组(is_pool=true 时必为非空数组)
    pub pool_members: Json,

    /// 发起打印时的缺省份数
    pub default_copies: i16,

    /// 是否启用
    pub enabled: bool,

    /// 创建时间
    pub created_at: DateTimeWithTimeZone,

    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

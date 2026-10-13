#![allow(dead_code)]
//! 打印任务模型
//!
//! 映射 `print_tasks` 表:记录每次打印请求的发起人、来源单据、目标设备/池、打印参数快照、
//! 后端类型与状态流转结果。字段可空性与 DDL 逐列一致(NOT NULL 列为非 Option,可空列为
//! Option;`doc_id` 为多态单据引用取 BIGINT/i64,`payload_params` 为 JSONB/Json 非 Option,
//! 时间列为 TIMESTAMPTZ/DateTimeWithTimeZone)。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 打印任务 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "print_tasks")]
pub struct Model {
    /// 任务主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 任务业务唯一编码
    pub job_code: String,

    /// 发起人用户 ID
    pub requested_by: i32,

    /// 发起人名称快照
    pub requested_by_name: String,

    /// 来源单据类型
    pub doc_type: String,

    /// 来源单据主键(多态引用,列为 BIGINT)
    pub doc_id: i64,

    /// 所选打印模板 ID(可空,外键引用 print_templates.id)
    pub template_id: Option<i32>,

    /// 打印目标种类(printer/pool)
    pub target_kind: String,

    /// 打印目标符号名(真实地址由运行期配置解析)
    pub target_name: String,

    /// 打印份数
    pub copies: i32,

    /// 纸张尺寸
    pub paper_size: String,

    /// 打印方向(portrait/landscape)
    pub orientation: String,

    /// 上边距(毫米)
    pub margin_top_mm: i16,

    /// 下边距(毫米)
    pub margin_bottom_mm: i16,

    /// 左边距(毫米)
    pub margin_left_mm: i16,

    /// 右边距(毫米)
    pub margin_right_mm: i16,

    /// 是否启用条码
    pub barcode_enabled: bool,

    /// 发起打印的来源模块
    pub source_module: String,

    /// 附加打印参数(JSON 对象)
    pub payload_params: Json,

    /// 任务状态(pending/queued/dispatched/succeeded/failed)
    pub status: String,

    /// 打印派发后端类型(cups/ipp/network/system_command)
    pub backend_kind: String,

    /// 后端返回的外部作业标识(可空)
    pub external_job_id: Option<String>,

    /// 失败原因(状态为 failed 时必非空)
    pub error_message: Option<String>,

    /// 派发时间(可空)
    pub dispatched_at: Option<DateTimeWithTimeZone>,

    /// 完成时间(可空)
    pub completed_at: Option<DateTimeWithTimeZone>,

    /// 创建时间
    pub created_at: DateTimeWithTimeZone,

    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

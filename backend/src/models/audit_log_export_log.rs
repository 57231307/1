//! V15 缺陷 10-4：审计日志导出二次审计表（防篡改）
//!
//! 独立于 `audit_logs` 表，记录每一次审计日志导出操作。
//! 数据库触发器禁止 UPDATE / DELETE（仅允许 INSERT），
//! 审计员无法篡改自身导出记录，满足 SOC2 / ISO27001 / 《数据安全法》第 32 条要求。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 审计日志导出二次审计记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_log_export_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 导出人用户 ID
    pub exporter_user_id: i32,
    /// 导出人用户名（冗余留存，用户改名后仍可追溯）
    pub exporter_username: String,
    /// 导出时的筛选条件 JSON
    #[sea_orm(column_type = "Text", nullable)]
    pub export_query_filter: Option<String>,
    /// 导出记录条数
    pub export_record_count: i32,
    /// 导出文件格式（xlsx / pdf）
    pub export_file_format: String,
    /// 导出文件 SHA256 指纹（事后比对验证文件未被替换）
    pub export_file_hash_sha256: Option<String>,
    /// 导出文件字节数
    pub export_file_size_bytes: Option<i64>,
    /// 导出请求来源 IP
    pub export_ip_address: Option<String>,
    /// 导出请求 User-Agent
    #[sea_orm(column_type = "Text", nullable)]
    pub export_user_agent: Option<String>,
    /// 请求追踪 ID（与 trace_context middleware 联动）
    pub export_request_id: Option<String>,
    /// 导出时间
    pub exported_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ExporterUserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

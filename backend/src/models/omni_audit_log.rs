#![allow(dead_code)]
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "omni_audit_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub description: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_method: Option<String>,
    pub request_path: Option<String>,
    pub request_body: Option<String>,
    pub response_status: Option<i32>,
    pub duration_ms: Option<i32>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub created_at: Option<DateTimeWithTimeZone>,
    /// HMAC-SHA256 防篡改签名（trace_id|event_type|action|payload）
    pub signature: Option<String>,
    /// 请求条件/查询条件（query string，V15 P0-S19：与 request_body 区分，用于快速筛选特定条件审计记录）
    #[sea_orm(column_type = "Text", nullable)]
    pub condition: Option<String>,
    /// V15 P2 B11-P2-3：操作分类
    pub operation_category: Option<String>,
    /// V15 P2 B11-P2-3：导出记录数
    pub export_record_count: Option<i32>,
    /// V15 P2 B11-P2-3：导出审批令牌
    pub export_approval_token: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#![allow(dead_code)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ==================== 资金操作详细日志 ====================
pub mod financial {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "financial_audit_logs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub tenant_id: Option<i32>,
        pub trace_id: Option<String>,
        pub user_id: Option<i32>,
        pub username: Option<String>,
        pub operation: String,
        pub financial_type: String,
        pub financial_id: i32,
        pub financial_no: String,
        pub amount: Option<Decimal>,
        pub currency: Option<String>,
        pub exchange_rate: Option<Decimal>,
        pub amount_cny: Option<Decimal>,
        pub operator_user_id: Option<i32>,
        pub operator_username: Option<String>,
        pub operator_ip: Option<String>,
        pub operator_department: Option<String>,
        pub related_type: Option<String>,
        pub related_id: Option<i32>,
        pub related_no: Option<String>,
        pub supplier_id: Option<i32>,
        pub supplier_name: Option<String>,
        pub customer_id: Option<i32>,
        pub customer_name: Option<String>,
        pub payment_method: Option<String>,
        pub bank_account: Option<String>,
        pub due_date: Option<Date>,
        pub invoice_ids: Option<Json>,
        pub before_status: Option<String>,
        pub after_status: Option<String>,
        pub approval_level: Option<i32>,
        pub approver_comments: Option<String>,
        pub context: Option<Json>,
        pub created_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ==================== 权限变更详细日志 ====================
pub mod permission {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "permission_audit_logs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub tenant_id: Option<i32>,
        pub trace_id: Option<String>,
        pub operator_user_id: i32,
        pub operator_username: String,
        pub operator_ip: Option<String>,
        pub operation: String,
        pub target_user_id: Option<i32>,
        pub target_username: Option<String>,
        pub target_roles: Option<Json>,
        pub before_roles: Option<Json>,
        pub after_roles: Option<Json>,
        pub before_permissions: Option<Json>,
        pub after_permissions: Option<Json>,
        pub roles_added: Option<Json>,
        pub roles_removed: Option<Json>,
        pub permissions_changed: Option<Json>,
        pub context: Option<Json>,
        pub created_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ==================== 安全事件详细日志 ====================
pub mod security {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "security_audit_logs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub tenant_id: Option<i32>,
        pub trace_id: Option<String>,
        pub event: String,
        pub username: Option<String>,
        pub user_id: Option<i32>,
        pub ip_address: Option<String>,
        pub user_agent: Option<String>,
        pub login_method: Option<String>,
        pub login_type: Option<String>,
        pub failure_reason: Option<String>,
        pub attempts_today: Option<i32>,
        pub attempts_total: Option<i32>,
        pub last_success: Option<DateTimeWithTimeZone>,
        pub last_failure: Option<DateTimeWithTimeZone>,
        pub risk_level: Option<String>,
        pub risk_factors: Option<Json>,
        pub blocked: Option<bool>,
        pub block_reason: Option<String>,
        pub require_captcha: Option<bool>,
        pub notify_user: Option<bool>,
        pub geo_country: Option<String>,
        pub geo_region: Option<String>,
        pub geo_city: Option<String>,
        pub geo_isp: Option<String>,
        pub device_os: Option<String>,
        pub device_browser: Option<String>,
        pub device_type: Option<String>,
        pub is_mobile: Option<bool>,
        pub created_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ==================== 性能监控日志 ====================
pub mod performance {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "performance_logs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub tenant_id: Option<i32>,
        pub trace_id: Option<String>,
        pub endpoint: String,
        pub method: String,
        pub user_id: Option<i32>,
        pub total_duration_ms: i32,
        pub db_duration_ms: Option<i32>,
        pub cache_duration_ms: Option<i32>,
        pub external_duration_ms: Option<i32>,
        pub serialization_duration_ms: Option<i32>,
        pub middleware_duration_ms: Option<i32>,
        pub db_queries_count: Option<i32>,
        pub db_slow_queries: Option<i32>,
        pub db_connection_pool_active: Option<i32>,
        pub db_connection_pool_idle: Option<i32>,
        pub db_connection_pool_waiting: Option<i32>,
        pub cache_hits: Option<Json>,
        pub cache_misses: Option<Json>,
        pub cache_hit_rate: Option<Decimal>,
        pub memory_allocated_mb: Option<Decimal>,
        pub memory_peak_mb: Option<Decimal>,
        pub gc_count: Option<i32>,
        pub response_status: Option<i32>,
        pub response_size_bytes: Option<i32>,
        pub created_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ==================== 系统健康日志 ====================
pub mod health {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "system_health_logs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub tenant_id: Option<i32>,
        pub cpu_usage_percent: Option<Decimal>,
        pub memory_usage_percent: Option<Decimal>,
        pub disk_usage_percent: Option<Decimal>,
        pub load_average_1m: Option<Decimal>,
        pub load_average_5m: Option<Decimal>,
        pub load_average_15m: Option<Decimal>,
        pub uptime_seconds: Option<i64>,
        pub db_status: Option<String>,
        pub db_connections_active: Option<i32>,
        pub db_connections_idle: Option<i32>,
        pub db_connections_max: Option<i32>,
        pub db_connections_waiting: Option<i32>,
        pub db_replication_lag_ms: Option<i32>,
        pub db_query_time_avg_ms: Option<Decimal>,
        pub cache_status: Option<String>,
        pub cache_memory_used_mb: Option<Decimal>,
        pub cache_memory_max_mb: Option<Decimal>,
        pub cache_hit_rate: Option<Decimal>,
        pub cache_evictions: Option<i64>,
        pub app_version: Option<String>,
        pub app_environment: Option<String>,
        pub app_active_users: Option<i32>,
        pub app_requests_per_minute: Option<i32>,
        pub app_error_rate_percent: Option<Decimal>,
        pub created_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ==================== 业务操作详细日志 ====================
pub mod business {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "business_audit_logs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub tenant_id: Option<i32>,
        pub trace_id: Option<String>,
        pub module: String,
        pub operation: String,
        pub resource_type: Option<String>,
        pub resource_id: Option<String>,
        pub resource_name: Option<String>,
        pub operator_user_id: Option<i32>,
        pub operator_username: Option<String>,
        pub operator_ip: Option<String>,
        pub action_details: Option<Json>,
        pub before_data: Option<Json>,
        pub after_data: Option<Json>,
        pub diff_data: Option<Json>,
        pub context: Option<Json>,
        pub success: Option<bool>,
        pub affected_rows: Option<i32>,
        pub generated_id: Option<i32>,
        pub error_message: Option<String>,
        pub created_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

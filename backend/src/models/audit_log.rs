use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

/// 审计日志差异快照 JSON 值（与 before/after_snapshot 字段对应）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct AuditValue(pub serde_json::Value);

/// 操作类型枚举（推荐用于新记录，兼容旧的 `action` 字段）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OperationType {
    /// 数据新建
    Create,
    /// 数据更新
    Update,
    /// 数据删除
    Delete,
    /// 登录成功
    Login,
    /// 登出
    Logout,
    /// 数据导出
    Export,
    /// 数据打印（V15 P0-S10 新增）
    Print,
    /// 文件下载（V15 P0-S10 新增）
    Download,
    /// 数据查询（详情 / 列表）
    Query,
    /// 其它类型
    Other,
}

impl OperationType {
    /// 序列化为大写字符串（持久化到数据库的稳定字符串）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "CREATE",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
            Self::Login => "LOGIN",
            Self::Logout => "LOGOUT",
            Self::Export => "EXPORT",
            Self::Print => "PRINT",
            Self::Download => "DOWNLOAD",
            Self::Query => "QUERY",
            Self::Other => "OTHER",
        }
    }
}

/// 严重级别枚举
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    /// 普通信息
    Info,
    /// 警告
    Warn,
    /// 错误
    Error,
    /// 严重（需立即响应）
    Critical,
}

impl Severity {
    /// 序列化为大写字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    /// 旧字段：通用操作类型（自由文本，与 operation_type 语义重叠，新代码优先用 operation_type）
    pub action: String,
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
    /// 旧字段：变更前快照 JSON（与 before_snapshot 同义）
    pub old_value: Option<AuditValue>,
    /// 旧字段：变更后快照 JSON（与 after_snapshot 同义）
    pub new_value: Option<AuditValue>,
    pub created_at: Option<DateTimeUtc>,
    // ============ P13 批 1 P3-2 增强字段（m0023 添加）============
    /// 操作类型（推荐字段，枚举字符串）
    pub operation_type: Option<String>,
    /// 严重级别（INFO / WARN / ERROR / CRITICAL）
    pub severity: Option<String>,
    /// 请求追踪 ID（与 trace_context middleware 联动）
    pub request_id: Option<String>,
    /// 变更前快照 JSON（推荐字段，语义清晰的命名）
    pub before_snapshot: Option<AuditValue>,
    /// 变更后快照 JSON（推荐字段，语义清晰的命名）
    pub after_snapshot: Option<AuditValue>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 操作类型枚举序列化为稳定字符串
    #[test]
    fn test_op_type_as_str() {
        assert_eq!(OperationType::Create.as_str(), "CREATE");
        assert_eq!(OperationType::Login.as_str(), "LOGIN");
        assert_eq!(OperationType::Export.as_str(), "EXPORT");
        assert_eq!(OperationType::Print.as_str(), "PRINT");
        assert_eq!(OperationType::Download.as_str(), "DOWNLOAD");
        assert_eq!(OperationType::Other.as_str(), "OTHER");
    }

    /// 严重级别枚举序列化为稳定字符串
    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Info.as_str(), "INFO");
        assert_eq!(Severity::Warn.as_str(), "WARN");
        assert_eq!(Severity::Error.as_str(), "ERROR");
        assert_eq!(Severity::Critical.as_str(), "CRITICAL");
    }
}

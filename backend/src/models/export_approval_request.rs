#![allow(dead_code)]
use sea_orm::FromJsonQueryResult;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 导出参数 JSON（过滤条件/字段选择等）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ExportParams(pub serde_json::Value);

/// 审批上下文 JSON
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ApprovalContext(pub serde_json::Value);

/// 审批状态枚举（与数据库 status 字段对应）：敏感数据导出审批机制，
/// 生命周期改造扩展 consumed 与 exhausted 两个终态，
/// 与数据库 chk_ear_status CHECK 约束取值集合逐字符一致
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// 待审批（一级待审批）
    Pending,
    /// V15 主线审计 Critical 修复：二级待审批（一级已通过，等待二级审批人介入）
    PendingL2,
    /// 已审批通过（BPM 终审完成后进入，等待申请人凭令牌导出）
    Approved,
    /// 已拒绝
    Rejected,
    /// token 已过期（有效期到但未成功导出）
    Expired,
    /// 申请人取消
    Cancelled,
    /// 令牌已消费（导出成功，一次性终态，禁复用）
    Consumed,
    /// 重试次数用尽（未成功前提下累计达 max_retries，不可再重试）
    Exhausted,
}

impl ApprovalStatus {
    /// 序列化为小写字符串（与数据库 status 字段一致）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::PendingL2 => "pending_l2",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Consumed => "consumed",
            Self::Exhausted => "exhausted",
        }
    }

    /// 从数据库字符串解析（V15 P0-S14 clippy 修复：from_str→parse_status 避免 std::str::FromStr 冲突）
    pub fn parse_status(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "pending_l2" => Some(Self::PendingL2),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "expired" => Some(Self::Expired),
            "cancelled" => Some(Self::Cancelled),
            "consumed" => Some(Self::Consumed),
            "exhausted" => Some(Self::Exhausted),
            _ => None,
        }
    }

    /// 是否属于终态（不可再迁移的状态集合）
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Expired | Self::Cancelled | Self::Consumed | Self::Exhausted
        )
    }

    /// 是否属于在途审批中态（pending/pending_l2，申请人可取消、审批人可办理）
    pub fn is_pending_any(&self) -> bool {
        matches!(self, Self::Pending | Self::PendingL2)
    }
}

/// 风险等级枚举
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// 低风险
    Low,
    /// 中风险
    Medium,
    /// 高风险
    High,
    /// 严重风险
    Critical,
}

impl RiskLevel {
    /// 序列化为小写字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// 根据预估导出行数评估风险等级
    pub fn from_row_count(rows: i64) -> Self {
        if rows >= 50_000 {
            Self::Critical
        } else if rows >= 10_000 {
            Self::High
        } else if rows >= 1_000 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// 敏感数据导出二级审批请求（V15 P0-S14：申请人→一级审批→二级审批，通过后生成 5 分钟临时 token 下载）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "export_approval_request")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 申请人用户 ID
    pub applicant_user_id: i32,
    /// 申请人用户名
    pub applicant_username: String,
    /// 审批人用户 ID（二级审批时填充）
    pub approver_user_id: Option<i32>,
    /// 审批人用户名
    pub approver_username: Option<String>,
    /// 导出资源类型：customer/supplier/dye_recipe/price_list/finance_report 等
    pub resource_type: String,
    /// 导出参数（过滤条件/字段选择等）
    pub export_params: Option<ExportParams>,
    /// 预估导出行数
    pub estimated_rows: Option<i64>,
    /// 文件格式：xlsx/pdf/csv
    pub file_format: String,
    /// 审批状态：pending/approved/rejected/expired/cancelled
    pub status: String,
    /// 当前审批层级：1=一级，2=二级
    pub approval_level: i32,
    /// 审批人备注
    pub approver_comments: Option<String>,
    /// 审批通过时间
    pub approved_at: Option<DateTimeWithTimeZone>,
    /// 审批拒绝时间
    pub rejected_at: Option<DateTimeWithTimeZone>,
    /// 临时下载令牌（审批通过后生成，5 分钟有效）
    pub download_token: Option<String>,
    /// token 过期时间（approved_at + 5min）
    pub token_expires_at: Option<DateTimeWithTimeZone>,
    /// 已下载次数
    pub download_count: i32,
    /// 最大下载次数（默认 1，防重放攻击）
    pub max_downloads: i32,
    /// 导出文件临时存储路径
    pub file_path: Option<String>,
    /// 文件大小（字节）
    pub file_size_bytes: Option<i64>,
    /// 文件 SHA256 校验值
    pub file_checksum: Option<String>,
    /// 申请人 IP
    pub applicant_ip: Option<String>,
    /// 审批人 IP
    pub approver_ip: Option<String>,
    /// 申请人 User-Agent
    pub applicant_user_agent: Option<String>,
    /// 风险等级：low/medium/high/critical
    pub risk_level: String,
    /// 审批上下文（JSON）
    pub context: Option<ApprovalContext>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    /// 流程终结时间（下载完成或 token 过期）
    pub completed_at: Option<DateTimeWithTimeZone>,
    /// 令牌 SHA256 哈希（m0105 起新记录只写哈希，明文不落库；老行为 NULL 走原 download_token 明文回退）
    pub token_hash: Option<String>,
    /// 令牌前缀指纹（供 UI 展示，形如 exp_a1b2c3d4，不含完整凭据）
    pub token_fingerprint: Option<String>,
    /// 一次性令牌是否已被申请人取走查看（true 后 token-once 端点拒第二次调用，只回指纹）
    pub token_viewed: bool,
    /// 令牌未成功导出前提下的累计重试次数
    pub retry_count: i32,
    /// 最近一次失败导出尝试的时刻（重试窗口判定源，与注入时钟比较）
    pub last_attempt_at: Option<DateTimeWithTimeZone>,
    /// 该令牌允许的最大重试次数（默认 3）
    pub max_retries: i32,
    /// 令牌是否已被成功导出消费（一次性消费标记）
    pub token_consumed: bool,
    /// 令牌消费时刻（导出成功那一刻落库）
    pub token_consumed_at: Option<DateTimeWithTimeZone>,
    /// 关联的 BPM 流程实例 ID（多态松散引用，走 bpm_process_instance.id）
    pub bpm_instance_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联申请人（users 表）
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::ApplicantUserId",
        to = "crate::models::user::Column::Id"
    )]
    Applicant,
    /// 关联审批人（users 表）
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::ApproverUserId",
        to = "crate::models::user::Column::Id"
    )]
    Approver,
}

impl ActiveModelBehavior for ActiveModel {}

/// 敏感资源类型注册表（V15 P0-S14：定义哪些资源类型的导出需要二级审批）
pub mod sensitive_resources {
    /// 客户清单
    pub const CUSTOMER: &str = "customer";
    /// 供应商清单
    pub const SUPPLIER: &str = "supplier";
    /// 染色配方
    pub const DYE_RECIPE: &str = "dye_recipe";
    /// 价格清单（销售/采购）
    pub const PRICE_LIST: &str = "price_list";
    /// 财务报表
    pub const FINANCE_REPORT: &str = "finance_report";
    /// 审计日志
    pub const AUDIT_LOG: &str = "audit_log";
    /// 产品清单
    pub const PRODUCT: &str = "product";

    /// 判断资源类型是否为敏感资源（需二级审批）
    pub fn is_sensitive(resource_type: &str) -> bool {
        matches!(
            resource_type,
            CUSTOMER | SUPPLIER | DYE_RECIPE | PRICE_LIST | FINANCE_REPORT | AUDIT_LOG | PRODUCT
        )
    }

    /// 获取所有敏感资源类型
    pub fn all_sensitive() -> &'static [&'static str] {
        &[
            CUSTOMER,
            SUPPLIER,
            DYE_RECIPE,
            PRICE_LIST,
            FINANCE_REPORT,
            AUDIT_LOG,
            PRODUCT,
        ]
    }
}

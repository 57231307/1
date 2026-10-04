//! 全局常量定义
//!
//! BE-C 修复（2026-06-26 第三优先级）：
//! 消除后端业务代码中的硬编码（"CNY" 币种码、默认账期天数、默认仓库/部门/采购员 ID）。
//! 所有硬编码常量集中于此，便于统一维护和审计。

/// 环保税法定常量（污染当量值＝法定不可调值，唯一来源）
pub mod environmental_tax;

/// customers.customer_type 唯一词表（列语义=渠道；允许值集合、缺省 `other`
/// 与各写入口校验的单一来源）
pub mod customer_type;

/// 默认币种码（ISO 4217）
///
/// 用于 currency 字段的默认值。当前业务主要面向国内市场，默认人民币。
/// 多币种模块接入后应由币种配置表驱动。
pub const DEFAULT_CURRENCY: &str = "CNY";

/// 默认付款账期天数
///
/// 用于 payment_terms 字段的默认值。30 天账期是面料行业常见付款条件。
/// 具体账期应由客户/供应商合同决定，此为兜底默认值。
pub const DEFAULT_PAYMENT_TERMS_DAYS: i32 = 30;

/// 默认仓库 ID
///
/// 用于 warehouse_id 字段的默认值。当前系统仅配置了一个仓库（ID=1）。
/// 多仓库模块接入后应由业务上下文决定。
pub const DEFAULT_WAREHOUSE_ID: i32 = 1;

/// 默认部门 ID
///
/// 用于 department_id 字段的默认值。当前系统仅配置了一个部门（ID=1）。
/// 组织架构模块接入后应由当前用户所属部门决定。
pub const DEFAULT_DEPARTMENT_ID: i32 = 1;

/// 默认采购员 ID
///
/// 用于 purchaser_id 字段的默认值。当前系统仅配置了一个采购员（ID=1）。
/// 应由当前登录用户的采购员身份决定。
pub const DEFAULT_PURCHASER_ID: i32 = 1;

/// 全局 HTTP 请求体大小上限：12 MB（安全漏洞 #8 既定值：CSV 导入 10MB + 2MB 编码/头部余量）。
///
/// 唯一事实源：`bootstrap/middleware_bootstrap.rs` 的全局 `DefaultBodyLimit` 层与任何
/// handler 内单点覆写提取上限（如供应商资质附件上传须让业务侧 5MB 显式校验先于
/// axum Multipart 的 2MB 默认截断生效）都引用本常量，禁止在业务代码里另写第二份数值。
pub const MAX_HTTP_BODY_BYTES: usize = 12 * 1024 * 1024;

pub mod admin_checker;
pub mod app_state;
pub mod cache;
pub mod data_permission;
pub mod date_utils;
pub mod failover;
pub mod path_utils;
pub mod request_ext;

pub mod config; // Wave 4 漏洞 #12：统一 is_production 配置来源（APP_ENV）
pub mod di_container;
pub mod dual_unit_converter;
pub mod error;
pub mod fabric_five_dimension;
pub mod incoterms;
pub mod password_validator;
pub mod response;

pub use response::ApiResponse;
pub use response::PaginatedResponse;
pub mod crud_macro;
pub mod field_mask;
pub mod hash;
pub mod import_export;
pub mod log_config;
pub mod number_generator;
pub mod pagination;
pub mod process_state_machine;
pub mod random;
pub mod sql_escape;
pub mod webhook_signature;
// P0-4 色卡仓储管理 - 色彩空间转换工具
pub mod color_space_converter;
// P0-5 面料多色号定价扩展 - 价格计算引擎
pub mod price_calculator;
// P4-1 性能优化 - N+1 查询修复工具集
pub mod n_plus_one;
// P4-2 安全加固 - 令牌桶限流算法
pub mod token_bucket;
// P9-1 关键路径 unwrap 清理 - 统一 expect/unwrap 集中化工具
pub mod unwrap_safe;
// P0-Wave1 安全加固 - 安全事件审计
pub mod audit;
// 低危 #2 SSRF 防护 - Webhook URL 内网白名单校验
pub mod ssrf_guard;
// 批次 98 P2-B 修复（v5 复审）：通用金额/数量范围 + 精度校验
pub mod validator;

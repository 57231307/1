//! 多业务模式 ops 子模块入口（business_mode_ops）
//!
//! 批次 489 D10-2b 拆分：从原 `business_mode_service.rs` 拆出 4 个 Service impl 块。
//! - `config`：BusinessModeConfigService impl（模式配置 CRUD + 默认管理 + 详情查询）
//! - `flow_step`：BusinessModeFlowStepService impl（流程节点 CRUD）
//! - `rule`：BusinessModeRuleService impl（规则 CRUD）
//! - `order_link`：BusinessModeOrderLinkService impl（单据关联 CRUD）
//! - `types`：10 个 DTO struct
//!
//! 4 个 Service struct 定义与 `new` 构造函数保留在 facade `business_mode_service` 中，
//! impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。

pub mod config;
pub mod flow_step;
pub mod order_link;
pub mod rule;
pub mod types;

// re-export DTOs，facade 通过 `pub use` 二次 re-export 保持外部引用路径不变
pub use types::{
    BusinessModeConfigQuery, BusinessModeOrderLinkQuery, CreateBusinessModeConfigRequest,
    CreateBusinessModeFlowStepRequest, CreateBusinessModeOrderLinkRequest,
    CreateBusinessModeRuleRequest, UpdateBusinessModeConfigRequest,
    UpdateBusinessModeFlowStepRequest, UpdateBusinessModeOrderLinkRequest,
    UpdateBusinessModeRuleRequest,
};

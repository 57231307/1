//! 能耗管理服务的业务实现子模块（energy_ops）
//!
//! 批次 488 D10-2a 拆分：从原 `energy_service.rs` 迁移 4 个独立 service。
//! 每个子模块自包含 struct + impl + DTOs。
//!
//! 模块层级关系：
//! - `energy_ops` 与 `energy_service` 同为 `crate::services` 下的兄弟模块
//! - `energy_service.rs` 作为 facade，re-export 4 个 service + 保留 9 个纯函数 + 测试模块
//! - 子模块通过 `use crate::services::energy_service::{...}` 复用 facade 的纯函数

pub mod allocation_record;
pub mod allocation_rule;
pub mod consumption;
pub mod meter;

pub use allocation_record::{
    AllocationRecordQuery, CreateAllocationRecordRequest, EnergyAllocationRecordService,
    MonthlyAllocationRequest, UpdateAllocationRecordRequest,
};
pub use allocation_rule::{
    CreateRuleRequest, EnergyAllocationRuleService, RuleQuery, UpdateRuleRequest,
};
pub use consumption::{
    ConsumptionQuery, CreateConsumptionRequest, EnergyConsumptionService, UpdateConsumptionRequest,
    WorkshopEnergySummary,
};
pub use meter::{CreateMeterRequest, EnergyMeterService, MeterQuery, UpdateMeterRequest};

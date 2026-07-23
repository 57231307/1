//! 销售发货服务的业务实现子模块（delivery_ops）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L126-1444 迁移 impl SalesService 方法。
//!
//! 模块层级关系：
//! - `delivery_ops` 与 `delivery` 同为 `crate::services::so` 下的兄弟模块
//! - `SalesService` struct 定义在 `crate::services::so::order`（facade）
//! - 各子模块通过 `use super::super::order::SalesService;` 直接导入，编写各自 `impl SalesService` 块
//! - impl 块分散在子模块中，Rust 允许同一 crate 多文件多 impl 块

pub mod cancel;
pub mod export;
pub mod inventory;
pub mod ship;
pub mod types;

pub use types::{ShipOrderContext, ShipPostCommitContext, ShipmentItemsResult};

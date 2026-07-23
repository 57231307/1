//! 生产订单服务的业务实现子模块（production_order_ops）
//!
//! 批次 488 D10-2 拆分：从原 `production_order_service.rs` L86-1502 迁移。
//!
//! 模块层级关系：
//! - `production_order_ops` 与 `production_order_service` 同为 `crate::services` 下的兄弟模块
//! - `ProductionOrderService` struct + `new` 定义在 `crate::services::production_order_service`（facade）
//! - 各子模块通过 `use crate::services::production_order_service::ProductionOrderService;` 直接导入，编写各自 `impl ProductionOrderService` 块
//! - `db` 字段在 facade 中声明为 `pub(crate)`，本模块 impl 块可直接访问 `self.db`
//! - impl 块分散在子模块中，Rust 允许同一 crate 多文件多 impl 块

pub mod approval;
pub mod completion;
pub mod crud;
pub mod types;

pub use types::{CreateProductionOrderRequest, ProductionOrderQuery, UpdateProductionOrderRequest};

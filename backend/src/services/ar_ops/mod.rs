//! 应收账款服务子模块（ar_ops）
//!
//! 批次 488 D10-1 拆分：从原 `services/ar_service.rs`（2489 行）拆出。
//! - `types`：内部聚合辅助 struct + 外部使用的 `CreateArPaymentParams`
//! - `json_helpers`：4 个 Model → JSON 序列化自由函数
//! - `collection`：收款管理（17 方法，原 ar_service.rs L112-751）
//! - `verification`：核销管理（23 方法，原 ar_service.rs L753-1778）
//! - `report`：报表管理（9 方法，原 ar_service.rs L1780-2177）
//!
//! 模块层级关系：
//! - `ar_ops` 与 `ar_service` 同为 `crate::services` 下的兄弟模块
//! - `ArService` struct + `new` 定义在 `crate::services::ar_service`（facade）
//! - 各子模块通过 `use crate::services::ar_service::ArService;` 直接导入，编写各自 `impl ArService` 块
//! - `db` 字段在 facade 中声明为 `pub(crate)`，本模块 impl 块可直接访问 `self.db`
//! - impl 块分散在子模块中，Rust 允许同一 crate 多文件多 impl 块

pub mod collection;
pub mod json_helpers;
pub mod report;
pub mod types;
pub mod verification;

pub use types::CreateArPaymentParams;

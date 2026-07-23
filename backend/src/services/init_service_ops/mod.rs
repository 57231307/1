//! 系统初始化服务的业务实现子模块（init_service_ops）
//!
//! 本模块与 `init_service` 同为 `crate::services` 下的兄弟模块。
//! `InitService` struct + `new` 定义在 `crate::services::init_service`（facade）。
//! 各子模块通过 `use crate::services::init_service::InitService;` 导入，编写各自 `impl InitService` 块。
//! `db` 字段在 facade 中声明为 `pub(crate)`，本模块 impl 块可直接访问 `self.db`。

pub mod dept_user;
pub mod permission;
pub mod role;
pub mod setup;

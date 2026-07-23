//! 系统更新服务的业务实现子模块（system_update_ops）
//!
//! 本模块与 `system_update_service` 同为 `crate::services` 下的兄弟模块。
//! `SystemUpdateService` struct + `new` 构造器定义在 `crate::services::system_update_service`（facade）。
//! 各子模块通过 `use crate::services::system_update_service::SystemUpdateService;` 导入，
//! 编写各自 `impl SystemUpdateService` 块。
//!
//! `app_dir` / `backup_dir` / `is_updating` 字段在 facade 中声明为 `pub(crate)`，
//! 本模块 impl 块可直接访问。
//!
//! 按职责拆分：
//! - `status`：版本号与状态查询 + 本地发布包列表（7 方法）
//! - `apply`：更新应用主流程 + 解压/校验/应用/日志（10 方法）
//! - `backup`：备份创建 + 回滚 + 旧备份清理（5 方法）
//! - `github`：GitHub 远程更新检查 + 下载（8 方法）

pub mod apply;
pub mod backup;
pub mod github;
pub mod status;

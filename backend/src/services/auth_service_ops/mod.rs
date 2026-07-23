//! 认证服务业务实现子模块（auth_service_ops）
//!
//! 本模块与 `auth_service` 同为 `crate::services` 下的兄弟模块。
//! `AuthService` struct + `new` 定义在 `crate::services::auth_service`（facade）。
//! `auth` 子模块通过 `use crate::services::auth_service::AuthService;` 导入，
//! 编写各自 `impl AuthService` 块。`db` / `encoding_key` 字段在 facade 中声明为
//! `pub(crate)`，本模块 impl 块可直接访问。
//!
//! `jti` 子模块承载 JTI 黑名单与用户级 Token 吊销的 free functions，
//! 由 facade 通过 `pub use` 重新导出，保持外部调用路径不变。

pub mod auth;
pub mod jti;

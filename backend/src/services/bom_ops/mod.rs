//! BOM Service 业务实现子模块入口（bom_ops）
//!
//! 批次 D10 拆分：从原 `bom_service.rs`（1046 行）拆出 `BomService` impl 块。
//! - `crud`：BOM 主表 CRUD + 版本/默认值管理（create/get_by_id/list/update/delete/get_versions/copy/get_next_version/set_default）
//! - `state`：状态机流转（submit/approve，lock_exclusive 串行化并发状态变更）
//! - `tree`：树形结构查询与多层级用量计算（get_bom_tree/calculate_bom_requirements + 私有 helper）
//!
//! `BomService` struct 定义与 `new` 构造函数保留在 facade `bom_service` 中，
//! impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。
//! `db` 字段在 facade 中声明为 `pub(crate)`，本模块可直接访问。
//!
//! 跨模块调用：
//! - `crud` 调用 facade 纯函数 `Self::cancel_existing_default_bom` / `Self::build_bom_item_models`（`pub(crate)`）
//! - `tree` 调用 facade 纯函数 `Self::build_leaf_bom_node`（`pub(crate)`）
//! - facade 测试模块 `bom_service::tests` 调用 `tree::collect_requirements`（故声明为 `pub(crate)`）
//! - ops 子模块间无跨模块调用，其余内部 helper 保持私有

pub mod crud;
pub mod state;
pub mod tree;

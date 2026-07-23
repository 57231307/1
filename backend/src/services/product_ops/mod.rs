//! 产品 Service 业务实现子模块入口（product_ops）
//!
//! 批次 D10 拆分：从原 `product_service.rs`（1075 行）拆出 `ProductService` impl 块。
//! - `sync`：ES 同步辅助（build_product_doc / sync_product_to_es）
//! - `crud`：产品 CRUD（list/get/create/update/delete + generate_product_code）
//! - `color`：色号管理（list/create/batch_create/update/delete product_color）
//! - `import_export`：CSV 导入导出（export/template/import + 字段校验/解析 helper）
//!
//! `ProductService` struct 定义与 `new` 构造函数保留在 facade `product_service` 中，
//! impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。
//! `db` / `search_syncer` 字段在 facade 中声明为 `pub(crate)`，本模块可直接访问。
//! 跨 ops 子模块调用的方法（如 `sync_product_to_es`）声明为 `pub(crate)`。

pub mod color;
pub mod crud;
pub mod import_export;
pub mod sync;

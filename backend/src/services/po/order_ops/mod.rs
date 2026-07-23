//! 采购订单服务的业务实现子模块（order_ops）
//!
//! 拆分自原 `po/order.rs` 的 `impl PurchaseOrderService` 块（facade 模式）。
//!
//! 模块层级关系：
//! - `order_ops` 与 `order` 同为 `crate::services::po` 下的兄弟模块
//! - `PurchaseOrderService` struct + `new` 定义在 `crate::services::po::order`（facade）
//! - 各子模块通过 `use crate::services::po::order::PurchaseOrderService;` 直接导入，编写各自 `impl PurchaseOrderService` 块
//! - `db` 字段在 facade 中声明为 `pub(crate)`，本模块 impl 块可直接访问 `self.db`
//! - impl 块分散在子模块中，Rust 允许同一 crate 多文件多 impl 块
//!
//! 子模块划分：
//! - `crud`      创建 / 更新 / 删除 / 列表 / 详情 + 其私有 helper
//! - `lifecycle` 生命周期（关闭）
//! - `query`     明细查询 / CSV 导出

pub mod crud;
pub mod lifecycle;
pub mod query;

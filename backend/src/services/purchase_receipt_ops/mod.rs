//! 采购入库业务实现子模块（purchase_receipt_ops）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs`（1074 行）拆出
//! `PurchaseReceiptService` impl 业务方法块，分散到本子模块。
//! Rust 允许同一 crate 内多个 `impl` 块，本子模块每个文件各持一个 impl 块。
//!
//! `PurchaseReceiptService` struct 定义 + `new` 构造器 + 单号生成宏
//! (`impl_generate_no!` generate_receipt_no) + 纯函数（build_receipt_active_model /
//! build_receipt_items_and_totals / build_confirmed_receipt_active_model）
//! + 单元测试保留在 facade `purchase_receipt_service` 中。
//! `db` 字段声明为 `pub(crate)` 供本子模块访问。
//!
//! 按职责拆分：
//! - `auth`：管理员身份校验（`is_admin_user`，`pub(crate)` 供 crud/items 跨模块调用）
//! - `crud`：入库单 CRUD（create_receipt / update_receipt / delete_receipt + update_receipt_totals）
//! - `state`：状态流转（confirm_receipt + lock_and_validate_receipt_txn + publish_events_and_generate_ap）
//! - `items`：入库明细 CRUD + 总金额重算（add/update/delete_receipt_item + calculate_receipt_total[_txn]）
//! - `query`：列表/详情/明细查询（list_receipts / get_receipt / list_receipt_items）
//!
//! DTO 定义在外部独立模块 `purchase_receipt_dto`，本子模块按需 `use` 引入，
//! 外部引用路径（`crate::services::purchase_receipt_service::PurchaseReceiptService`
//! 与 `crate::services::purchase_receipt_dto::*`）保持不变。

pub mod auth;
pub mod crud;
pub mod items;
pub mod query;
pub mod state;

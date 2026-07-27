//! 客户服务的业务实现子模块（customer_ops）
//!
//! 拆分：从原 `customer_service.rs` 迁移 CustomerService 的 impl 块。
//! struct 定义 + new 构造函数 + DTOs + 模块级 helper 保留在 facade `customer_service.rs`，
//! 通过 db / search_syncer 字段 pub(crate) 让子模块访问。
//!
//! 子模块划分：
//! - types：模块级 helper（select_customer_column / build_select_only_query）
//! - crud：create_customer / get_customer / list_customers / delete_customer
//! - query：list_customers_with_filter / get_customer_with_filter 等
//! - contact：客户联系人 4 方法（list/create/update/delete + clear_primary_contacts_txn）
//! - update：update_customer / sync_customer_to_es / generate_customer_code 等

pub mod contact;
pub mod crud;
pub mod query;
pub mod types;
pub mod update;

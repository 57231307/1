//! 多币种增强 Handler 兼容层
//!
//! 历史原因：原 currency_enhanced_handler.rs 中的 fn 已合并到
//! currency_handler.rs（多币种基础版）。本文件保留原模块名以便
//! `crate::handlers::currency_enhanced_handler::xxx` 的旧引用仍能工作。
//! 新代码请直接使用 `crate::handlers::currency_handler::xxx`。

pub use crate::handlers::currency_handler::*;

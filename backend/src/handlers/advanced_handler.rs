//! 高级 handler 兼容层
//!
//! 历史原因：原 advanced_handler.rs 已被拆分为 advanced/ 子模块下的 5 个子模块
//! （forecast / analytics / rec / reorder / decide）。
//! 本文件保留原模块名以便 `crate::handlers::advanced_handler::xxx` 的旧引用仍能工作。
//! 新代码请直接使用 `crate::handlers::advanced::xxx`。

pub use crate::handlers::advanced::*;

//! 销售价格服务（so/price）
//!
//! 占位模块。原 `sales_service.rs` 不包含独立的价格服务方法。
//! 销售价格相关逻辑位于 `models::product`（基础价格/着色加价/等级价差等）
//! 和 `so/order.rs::create_order`（按行计算 final_price）。
//! 本模块保留扩展空间，可用于后续实现：
//! - 客户专属价目表
//! - 批量调价
//! - 价格审批工作流

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

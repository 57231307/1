//! CRM 分配服务（crm/assign）
//!
//! 占位模块。原 `crm_service.rs` 不包含独立的"分配"服务方法。
//! 客户/线索/商机的所有权分配逻辑已分别由各业务子模块自行处理：
//! - 客户分配: cust.rs (update_customer_enhanced)
//! - 线索分配: lead.rs (create_lead / update_lead)
//! - 商机分配: opp.rs (create_opportunity / update_opportunity)
//! - 公海领取: pool.rs (claim_pool_customers)
//!
//! 本模块保留扩展空间，可用于后续实现：
//! - 批量分配（按区域/行业/规则）
//! - 自动分配（轮询/抢单）
//! - 转移分配（带审批）

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

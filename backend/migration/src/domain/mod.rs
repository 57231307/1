//! 迁移域分组
//!
//! 按业务域组织迁移：
//! - system: 核心表结构（角色/部门/用户/权限）
//! - business: 业务表（生产/采购/库存/销售/财务）
//! - sales_crm: 销售报价与CRM扩展
//! - production: 生产/质量/委外
//! - finance: 财务/合规/审计
//! - fixes: 修复与增强

pub mod system;
pub mod business;
pub mod sales_crm;
pub mod production;
pub mod v15_core;
pub mod v15_batch18;
pub mod v15_batch19;
pub mod v15_extensions;
pub mod v15_final;
pub mod finance;
pub mod fixes;

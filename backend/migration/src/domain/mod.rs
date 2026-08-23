//! 迁移域分组
//!
//! 按业务域组织，每个域 1 个聚合迁移文件：
//! - system: 核心表（用户/角色/部门/产品/供应商/客户/库存/财务基础）+ 修复
//! - business: 业务表（生产/采购/库存扩展/销售/财务扩展/BPM/通知/报表）
//! - sales_crm: 销售报价/CRM/色价
//! - production: 生产/质量/委外/事件/failover
//! - finance: 合规/权限/RLS/色卡/坏账/8D
//! - v15: V15 各批次（core/batch18/batch19/extensions/final 合并）

pub mod system;
pub mod business;
pub mod sales_crm;
pub mod production;
pub mod finance;
pub mod v15;

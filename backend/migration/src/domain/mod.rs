//! 迁移域分组
//!
//! 按业务域组织，每个域 1 个聚合迁移文件：
//! - system: 核心表（用户/角色/部门/产品/供应商/客户/库存/财务基础）+ 修复
//! - business: 业务表（生产/采购/库存扩展/销售/财务扩展/BPM/通知/报表）
//! - sales_crm: 销售报价/CRM/色价
//! - production: 生产/质量/委外/事件/failover
//! - finance: 合规/权限/RLS/色卡/坏账/8D
//! - v15: V15 各批次（core/batch18/batch19/extensions/final 合并）
//! - rls_dept: RLS dept 数据范围语义扩展（5 表冗余 department_id + 触发器 + 策略重写）
//! - crm_lead_claim_record: crm_lead 领取事件列（last_claimed_at/by，公海保护期与每日计数的唯一可靠判据）
//! - crm_vocab_check: CRM 状态词表 DB CHECK 约束（crm_lead/crm_opportunity，依赖 business 建表）
//! - price_vocab_check: 价格状态词表 DB CHECK 约束（sales_prices/purchase_prices，依赖 business
//!   建表与 v15 的默认值收敛/'ACTIVE' 回填）
//! - price_fk: 价格两表引用列外键（依赖 system 的 products/customers/suppliers 建表；有孤儿行
//!   时守卫 RAISE EXCEPTION 中止，不在迁移内洗数据）
//! - export_inspection_vocab_check: 出口商检结论词表 DB CHECK 约束（export_inspection.result，
//!   依赖 `migration/src/domain/v15/` 建表，排链尾满足"建表先于 CHECK"原则，全新库与存量库均不违反 CHECK）
//!   权重与出站开关三组，注册全链尾，角色/建表域均早于它）
//! - dye_recipe_reject: 染色配方拒绝通道（chk_dye_recipe_status 同约束名扩 rejected +
//!   rejected_reason 专列，必须排全链尾：v15 域尾重建四值 CHECK，早于它执行会被覆盖回窄集）

pub mod system;
pub mod business;
pub mod sales_crm;
pub mod production;
pub mod finance;
pub mod v15;
pub mod rls_dept;
pub mod rls_dept_user_sync;
pub mod crm_lead_claim_record;
pub mod crm_vocab_check;
pub mod price_vocab_check;
pub mod price_vocab_extend;
pub mod price_fk;
pub mod export_inspection_vocab_check;
pub mod dye_recipe_reject;

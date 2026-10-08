//! 数据库迁移模块
//!
//! 按业务域聚合，每个域 1 个注册项（域内可含多条子迁移，台账只记域名）：
//! system → business → sales_crm → production → finance → v15 → rls_dept
//!   → rls_dept_user_sync → crm_lead_claim_record → crm_vocab_check → price_vocab_check
//!   → price_vocab_extend → price_fk → export_inspection_vocab_check → dye_recipe_reject
//!
//! 迁移名: m_system_domain / m_business_domain / m_sales_crm_domain /
//!         m_production_domain / m_finance_domain / m_v15_domain / m_rls_dept_domain /
//!         m_rls_dept_user_sync / m_crm_lead_claim_record / m_crm_vocab_check
//!         / m_price_vocab_check / m0080_extend_price_status_check_rejected / m_price_fk
//!         / m_export_inspection_vocab_check / m0091_widen_dye_recipe_status_for_reject
//!（单文件域的台账名取文件名段，故 dye_recipe_reject 域登记为
//! `m0091_widen_dye_recipe_status_for_reject`；多文件域手写 `MigrationName`，台账记域名。）
//!
//! 顺序说明：
//! 1. system: 核心表（含 customers.owner_id/suppliers.created_by 补列）
//! 2. business: 业务表（依赖 system 的基础表；含 crm_lead/crm_opportunity 建表 m0013）
//! 3. sales_crm: 销售报价（依赖 business 的 product_color_prices）
//! 4. production: 生产/质量（依赖 business 的 custom_orders/process_nodes）
//! 5. finance: 合规/RLS（依赖 system 的 customers.owner_id/suppliers.created_by）
//! 6. v15: V15 各批次扩展
//! 7. rls_dept: RLS dept 语义扩展（依赖 finance 的 5 表 RLS 策略，重写为 dept 级）
//! 7.5 rls_dept_user_sync: users 换部门 → 同事务重算 5 表冗余 department_id
//!     （依赖 rls_dept 的补列/触发器，故紧随其后；含一次性存量回填与备份表精确回退）
//! 7.6 crm_lead_claim_record: crm_lead 领取事件列 last_claimed_at/last_claimed_by
//!     （依赖 business 建的 crm_lead；补列类迁移先于 CHECK 词表迁移收尾，故置于其前）
//! 8. crm_vocab_check: CRM 状态词表 CHECK（依赖 business 建的 crm_lead/crm_opportunity，
//!    排最后确保全部建表/回填类迁移（含 m0044 fix_fk_types）先行完成）
//! 9. price_vocab_check: 价格状态词表 CHECK（依赖 business 建表与 v15 的默认值收敛与
//!    'ACTIVE' 回填；排链尾满足"补列/回填先于 CHECK"原则，全新库与存量库均不违反 CHECK）
//! 9.5 price_vocab_extend: 价格状态词表扩 rejected 后按同约束名重建 CHECK
//!    （依赖 9 刚完成的默认值收敛与回填；施加前先点名词表外存量，非 0 即中止不造值）
//! 10. price_fk: 价格两表引用列外键（依赖 system 的 products/customers/suppliers；
//!    排在全部建表/回填之后，孤儿行存在时守卫中止而非静默删数据）
//! 11. dye_recipe_reject: 染色配方拒绝通道（chk_dye_recipe_status 同约束名扩 rejected +
//!    rejected_reason 专列；必须排全链尾——生效 CHECK 以本域为准，早于 v15 执行会被
//!    v15 域尾按同名约束重建的四值窄集覆盖回旧集）

pub use sea_orm_migration::prelude::*;

pub mod domain;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(domain::system::Migration),
            Box::new(domain::business::Migration),
            Box::new(domain::sales_crm::Migration),
            Box::new(domain::production::Migration),
            Box::new(domain::finance::Migration),
            Box::new(domain::v15::Migration),
            Box::new(domain::rls_dept::Migration),
            Box::new(domain::rls_dept_user_sync::Migration),
            Box::new(domain::crm_lead_claim_record::Migration),
            Box::new(domain::crm_vocab_check::Migration),
            Box::new(domain::price_vocab_check::Migration),
            Box::new(domain::price_vocab_extend::Migration),
            Box::new(domain::price_fk::Migration),
            Box::new(domain::export_inspection_vocab_check::Migration),
            Box::new(domain::dye_recipe_reject::Migration),
        ]
    }
}

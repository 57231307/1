//! 数据库迁移模块
//!
//! 按业务域聚合，每个域 1 个迁移文件，共 11 个：
//! system → business → sales_crm → production → finance → v15 → rls_dept
//!   → rls_dept_user_sync → crm_lead_claim_record → crm_vocab_check → price_vocab_check
//!
//! 迁移名: m_system_domain / m_business_domain / m_sales_crm_domain /
//!         m_production_domain / m_finance_domain / m_v15_domain / m_rls_dept_domain /
//!         m_rls_dept_user_sync / m_crm_lead_claim_record / m_crm_vocab_check
//!         / m_price_vocab_check
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
        ]
    }
}

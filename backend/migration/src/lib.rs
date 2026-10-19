//! 数据库迁移模块
//!
//! 按业务域聚合，每个域 1 个注册项（域内可含多条子迁移，台账只记域名）：
//! system → business → sales_crm → production → finance → v15 → rls_dept
//!   → rls_dept_user_sync → crm_lead_claim_record → crm_vocab_check → price_vocab_check
//!   → price_vocab_extend → price_fk → export_inspection_vocab_check → dye_recipe_reject
//!   → ddl_nullability_tighten → polymorphic_bill_id_widen → print_subsystem
//!   → company_profile → bulk_color_item_link → role_conflicts_drop_production_sod
//!   → finance_resource_rename → bpm_approver_grant → system_admin_bpm_config_grant
//!
//! 迁移名: m_system_domain / m_business_domain / m_sales_crm_domain /
//!         m_production_domain / m_finance_domain / m_v15_domain / m_rls_dept_domain /
//!         m_rls_dept_user_sync / m_crm_lead_claim_record / m_crm_vocab_check
//!         / m_price_vocab_check / m0080_extend_price_status_check_rejected / m_price_fk
//!         / m_export_inspection_vocab_check / m0091_widen_dye_recipe_status_for_reject
//!         / m_ddl_nullability_tighten / m_polymorphic_bill_id_widen_ar_invoice_txn / m_print_subsystem
//!         / m_company_profile
//!         / m_bulk_color_item_link
//!         / m_role_conflicts_drop_production_sod
//!         / m_finance_resource_rename / m_bpm_approver_grant
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
//! 12. print_subsystem: 打印子系统建表（print_templates 模板主数据 + print_tasks 任务台账
//!    + print_targets 打印目标登记；print_tasks.template_id 外键引用 print_templates，域内
//!    先建模板再建任务，print_targets 与两表无外键依赖、域内末位追加；置于链尾为新的
//!    顶层条目，域名在复用库上属 pending 必然执行，不并入既有域以免被整体跳过）
//! 13. company_profile: 公司档案单例建表（company_profile，id 恒 1 的抬头主数据，建表即
//!    播种空名行；置于链尾为新的顶层条目，复用库上也必然执行；打印 letterhead 与
//!    公司档案端点都依赖此表存在）
//! 14. bulk_color_item_link: 批色记录挂销售订单明细行（bulk_color_approval 补可空列
//!    sales_order_item_id 外键指向 sales_order_items + 检索索引；置于链尾为新的顶层条目，
//!    复用库上属 pending 必然执行，依赖 v15 建的 bulk_color_approval 与 sales_order_items）
//! 15. role_conflicts_drop_production_sod: 撤掉 finance 域种下的 2 对内置「生产↔质量」
//!    SoD 互斥默认（production_manager↔qc_manager、dyeing_master↔quality_inspector），
//!    财务钱账分管 6 对保持内置默认；必须排在 finance 域（建表并播种 8 对）之后才能删到
//!    那些行，且必须是链尾新顶层条目——并入已应用的域在复用库上因台账已记账会被整体跳过，
//!    删种动作永不执行。up 按精确三元组逐对删除并在删除前后各数一次财务 6 对在场行数
//!    （不等即 RAISE EXCEPTION，误伤不静默）；互斥机制（表 / GET /roles/conflicts 端点 /
//!    user_service 写口校验）与 role_relations 的 inherit+mutual_exclusive 全套均不触碰。
//! 16. finance_resource_rename: 财务域权限资源键改名波的存量库通道（role_permissions.
//!    resource_type 旧 URL 段名逐对改写到随域改名的新资源名；改名前按同 (role_id, action)
//!    做唯一索引冲突预检、一对多旧段设失败可见闸门待人工裁定；置于链尾为新的顶层条目）
//! 17. bpm_approver_grant: BPM 审批岗补授的存量库通道（按 roles.code 反查 id，为
//!    sales_manager/purchase_manager/production_manager 补授与 init 矩阵同源的六枚派生键；
//!    依赖 v15 建的 uq_role_permissions_role_resource_action 唯一索引，故必须排在其后的链尾，
//!    作为新的顶层条目，复用库上也必然执行）
//! 18. system_admin_bpm_config_grant: 系统管理岗补授 BPM 流程配置读面的存量库通道
//!    （按 roles.code 反查 id，为 system_admin 补授与 init 矩阵同源的 bpm-definitions 与
//!    bpm-templates 两枚 read 键；同样依赖 v15 的唯一索引，置于链尾为新的顶层条目）

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
            Box::new(domain::ddl_nullability_tighten::Migration),
            Box::new(domain::polymorphic_bill_id_widen::Migration),
            Box::new(domain::print_subsystem::Migration),
            Box::new(domain::company_profile::Migration),
            Box::new(domain::bulk_color_item_link::Migration),
            Box::new(domain::role_conflicts_drop_production_sod::Migration),
            Box::new(domain::finance_resource_rename::Migration),
            Box::new(domain::bpm_approver_grant::Migration),
            Box::new(domain::system_admin_bpm_config_grant::Migration),
        ]
    }
}

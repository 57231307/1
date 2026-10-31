//! 数据库迁移模块
//!
//! 按业务域聚合，每个域 1 个注册项（域内可含多条子迁移，台账只记域名）：
//! system → business → sales_crm → production → finance → v15 → rls_dept
//!   → rls_dept_user_sync → crm_lead_claim_record → crm_vocab_check → price_vocab_check
//!   → price_vocab_extend → price_fk → export_inspection_vocab_check → dye_recipe_reject
//!   → ddl_nullability_tighten → polymorphic_bill_id_widen → print_subsystem
//!   → company_profile → bulk_color_item_link → role_conflicts_drop_production_sod
//!   → fulfillment_source_and_po_origin → finance_resource_rename → bpm_approver_grant
//!   → system_admin_bpm_config_grant
//!   → ap_family_grant
//!   → purchase_greige_read_grant
//!   → customer_credit_name_backfill → customer_credit_remark_column
//!   → customer_credit_family_grant
//!   → system_admin_bpm_config_grant → ap_input_tax_subject
//!   → customers_nullable_columns_norm
//!   → status_vocab_check → wage_energy_vocab_check → contract_template_schema
//!   → m0104_add_document_no_partial_unique_four_tables
//!   → sales_contract_source_order
//!   → product_spec_extension_columns → product_vocab_check → product_roll_column_drop
//!   → contract_clause_snapshot_provenance → print_template_default_slot
//!   → textile_safety → purchase_inspection_result_check → export_approval_lifecycle
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
//!         / m_fulfillment_source_and_po_origin
//!         / m_finance_resource_rename / m_bpm_approver_grant / m_system_admin_bpm_config_grant
//!         / m_ap_family_grant
//!         / m_purchase_greige_read_grant
//!         / m_customer_credit_name_backfill / m_customer_credit_remark_column
//!         / m_customer_credit_family_grant
//!         其后四枚链尾登记为 m_status_vocab_check 与 m_wage_energy_vocab_check
//!         与 m_contract_template_schema 与 m_print_template_default_slot
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
//! 16. fulfillment_source_and_po_origin: 流转链地基两列（sales_order_items 补可空
//!    fulfillment_source TEXT 带三值 CHECK chk_sales_order_item_fulfillment_source，
//!    purchase_orders 补可空 source_sales_order_id INTEGER 外键指向 sales_orders + 检索索引；
//!    置于链尾为新的顶层条目，复用库上属 pending 必然执行，依赖 system 建的两表，
//!    两列均不回填存量行）
//! 17. finance_resource_rename: 财务域权限资源键改名波的存量库通道（role_permissions.
//!    resource_type 旧 URL 段名逐对改写到随域改名的新资源名；改名前按同 (role_id, action)
//!    做唯一索引冲突预检、一对多旧段设失败可见闸门待人工裁定；置于链尾为新的顶层条目）
//! 18. bpm_approver_grant: BPM 审批岗补授的存量库通道（按 roles.code 反查 id，为
//!    sales_manager/purchase_manager/production_manager 补授与 init 矩阵同源的六枚派生键；
//!    依赖 v15 建的 uq_role_permissions_role_resource_action 唯一索引，故必须排在其后的链尾，
//!    作为新的顶层条目，复用库上也必然执行）
//! 19. system_admin_bpm_config_grant: 系统管理岗补授 BPM 流程配置读面的存量库通道
//!    （按 roles.code 反查 id，为 system_admin 补授与 init 矩阵同源的 bpm-definitions 与
//!    bpm-templates 两枚 read 键；同样依赖 m_v15_domain 的唯一索引，置于链尾为新的顶层条目）
//! 20. ap_family_grant: 应付族授权面补授的存量库通道（按 roles.code 反查 id，为
//!    finance_manager 补授审核与报表面 11 枚、为 accountant 补授制单与记账面 13 枚，全部与
//!    init 矩阵逐字符同源；同样依赖 m_v15_domain 的唯一索引，置于链尾为新的顶层条目）
//! 21. purchase_greige_read_grant: 采购两岗补授 greige-fabrics:read 的存量库通道（与
//!    init 矩阵同源；原嵌在 business 域 up 链尾，复用库上随整域跳过而永不执行，故升为
//!    链尾顶层条目；roles/role_permissions 由 system 域先建，早于本域）
//!
//! 以下三域（客户信用族抢救域）追加在既有链尾之后，均满足三件事：已注册进本链、可重放、幂等。
//! 本支不主张自身为链尾末位——合并次序以先合者为尾，后合者须把各自域重挂在本三域之上；依赖
//! domain/v15 建的 uq_role_permissions_role_resource_action 唯一索引与 system、business 建的
//! customers、role_permissions、customer_credit_ratings 表，全部早于本三域：
//! - customer_credit_name_backfill: 客户信用评级冗余列 customer_name 的存量回填通道（按
//!   customer_id 关联 customers 取真名写入，取不到真名的行保持 NULL 并点名计数；备份将被改写的
//!   行的原值以支持精确回退）。
//! - customer_credit_remark_column: 客户信用评级备注兑现为真列（customer_credit_ratings 补可空
//!   列 remark VARCHAR(500)，与请求 DTO 的字段校验同宽；加列前先查 information_schema 走已存在
//!   则跳过的幂等路径）。
//! - customer_credit_family_grant: 客户信用族动作键补授财务两岗的存量库通道（按 roles.code 反查
//!   id，为 finance_manager 补授 read 与 create 与 update 与 delete 与 release 五枚、为
//!   accountant 补授 read 与 create 与 update 三枚，与 init 矩阵同源；ON CONFLICT 用真实唯一索引
//!   列集合保证幂等，逐键打印实写行数与跳过成因，同键显式拒绝行只点名不覆盖；down 只按三元组配对
//!   回收 allowed=true 的授权行，不抹人工显式拒绝行，也不碰 crm_manager 名下的通配授权行）。
//! 22. customers_nullable_columns_norm: customers 的 status/payment_terms 两列归一——
//!    system/finance 域当年以「可空、无默认」加列，而模型 `customer.rs` 声明非 Option，
//!    NULL 行整行解码失败；按既有同族收敛惯例（UPDATE 清 NULL → SET DEFAULT → SET NOT NULL，
//!    见 batch_no 与 owner_id/m0070）回填成文缺省（status 'active'，payment_terms 30，
//!    词表与默认值证据全量写在域文件头），置于链尾为新的顶层条目（并域会被台账跳过），
//!    依赖 system 建的两列，down 恢复可空无默认（数据无损论证见域文件头）
//! 23. status_vocab_check 与 wage_energy_vocab_check: 状态与取值域词表的 DB CHECK 收口两支
//! 24. contract_template_schema: 合同模板体系数据地基——contract_templates 模板主表（版本与
//!    同一类别同一立场只允许一版启用的部分唯一 ACTIVE 门）、clause_library 条款库（一枚条款码
//!    一枚可复用正文）、contract_template_clauses 模板条款项（条款库引用与模板内正文二选一，
//!    带顺序与必填标记与条件出条款标记与逐条法务复核位）、contract_clauses 签约条款快照
//!    （存固化当时的正文原文与模板版本号，模板改版不回溯改写已签合同），并给
//!    sales_contracts 与 purchase_contracts 补齐条款与签署与印花税与内外销与含税列，
//!    新增列一律可空且无默认，历史合同保持空值即「模板前签约，条款未固化」判据；
//!    域内全部形态为先探测后约束的失败可见写法，绝不 UPDATE 或 DELETE 业务数据。
//!    置于链尾为新的顶层条目（并入已应用的域会被台账整体跳过，四张表与补列全部缺席），
//!    依赖 system 与 business 与 v15 建好的两张合同主表，全部早于本域
//! 25. m0104_add_document_no_partial_unique_four_tables: 单据号四列活跃行部分唯一索引
//!    （production_recipe.recipe_no / production_recipe_addition.addition_no /
//!    process_wage_rate.rate_no / wage_record.record_no，形态照 m0068 部分唯一 +
//!    fail-visible 存量重复探测，谓词与读路径判重同源 is_deleted = false）——服务侧
//!    取号改「与插入同一事务」形态的 DB 先决约束，先补约束再收临界区；四表均由 v15 域
//!    建表（四表表定义见 v15 域），置于链尾为新的顶层条目：链尾晚于 v15 执行，
//!    表必然已存在、无跨域尾调问题，复用库上也必然执行（并入已应用域会被台账整体跳过）
//! 26. product_spec_extension_columns: 产品主档规格扩展列。products 加
//!    width_needle_included_cm 与 weight_tolerance 与 spandex_percent 与 usage 与
//!    safety_category，product_colors 加 color_fastness_grade，全部可空无默认。独立顶层域
//!    是因为并入 business 域会在复用库被台账整体跳过、补列全部缺席；两张表由 business 域
//!    先建，置于链尾必然晚于建表
//! 27. product_vocab_check: 上述扩展列的取值词表收口域，晚于加列域，形如先加列、再点名、
//!    再收约束；全部先探测后约束的失败可见写法，域内绝不 UPDATE 或 DELETE 业务数据
//! 28. product_roll_column_drop: 产品每卷米数列的探测式下线域，晚于代码面摘除读写
//!    （先改代码，再收列形态）；脏值存在时 RAISE EXCEPTION 点名中止不洗数据，
//!    down 只回空壳不回填历史值
//! 29. sales_contract_source_order: 销售合同与合同明细的两列溯源与四列行级金额与色号扩展
//!    （source_order_id 与 source_order_item_id 外键随源单删除置空，折扣率与折扣额与税率与
//!    税额四列和行级色号列，两表各一条检索索引）；新增列一律可空无默认，历史合同保持空值
//!    即「非订单转来」的判据，域内绝不 UPDATE 或 DELETE 业务数据；置于链尾为新的顶层条目，
//!    依赖 system 与 business 与 v15 建好的合同两表与销售订单两表，台账名取域名
//!    m_sales_contract_source_order
//! 30. contract_clause_snapshot_provenance: 合同条款快照来源指针独立域——给 contract_clauses
//!    追加一列可空无默认的 template_clause_id INTEGER，记录固化当时引用的模板条款项行主键，
//!    供详情腿单义回查必填标记；up 与 down 互逆（加一列撤一列），绝不 UPDATE、不回填、
//!    不建外键、不建索引，历史快照行留空即「无溯源指针」，读侧对空指针走 (模板, 条款码)
//!    唯一行回落，多义如实给空值不伪造。为什么另立独立域已在复用测试库上入账，
//!    域内新增子迁移在复用库上永不重放，只有让新域名处于 pending 才能被 migrate run 拾起；
//!    挂在链尾 contract_template_schema 之后，只依赖该域建的快照表
//! 31. print_template_default_slot: 打印模板默认位库侧唯一门——给 print_templates 按
//!    单据类型 doc_type 建"启用且默认至多一行"的部分唯一索引
//!    uq_print_template_active_default，索引谓词 WHERE is_default AND status='active'
//!    与出件解析 load_active_default_template 的三条件过滤逐字符同源（键 doc_type
//!    即解析查找键，写入侧建/改/设默认三处竞位清理同口径）；建索引前只读探测存量
//!    违例，命中即 RAISE EXCEPTION 点名中止，不 UPDATE 不 DELETE 洗数据，人工口径
//!    与解析同源保留每 doc_type id 最小一行；同支更正 is_default 列注释，如实描述
//!    互斥域是 doc_type 而非 module。置于链尾为新的顶层条目（并入已应用的
//!    print_subsystem 域会被台账跳过，索引与注释永不落地），依赖 print_subsystem 的
//!    m0098 建表，早于本域执行，down 撤索引并还原建表注释原文
//! 32. textile_safety: 纺织品安全合规域三支同域迁移（m0110 建八张表、m0111 取值列词表
//!    CHECK 收口、m0112 GB 18401 与 GB 31701 限值矩阵与出口市场要求播种，建表、CHECK、
//!    播种次序固定于域 mod.rs）。新建顶层域，与既有七张检验表零交集，送检单仅以
//!    product_id 外键只读关联 system 建的 products；置于链尾为新的顶层条目，复用库上
//!    属 pending 必然执行
//! 33. purchase_inspection_result_check: purchase_inspection.inspection_result 值域收口
//!    两支同域迁移（m0113a 严格只读逐值点名先于 m0113b 守卫施加 CHECK，口径照
//!    wage_energy_vocab_check 的 m0102 与 m0103 两段式），权威词表 models::status::
//!    purchase_inventory::purchase_inspection_result，NULL 为完成质检前合法初值，
//!    守卫拒绝不在迁移里 UPDATE 洗数据；置于链尾为新的顶层条目，晚于 business 建列
//! 34. export_approval_lifecycle: 导出审批令牌生命周期改造域（m0105）——给
//!    export_approval_request 幂等加 token_hash/token_fingerprint/token_viewed/retry_count/
//!    last_attempt_at/max_retries/token_consumed/token_consumed_at/bpm_instance_id 九列 +
//!    bpm_instance_id 外键指向 bpm_process_instance(ON DELETE SET NULL) + token_hash 部分唯一
//!    索引；status CHECK 以同约束名扩 consumed、exhausted 两终态。本域只改 schema，
//!    code=export_approval 的流程定义行不在迁移里登记，改由 services/bpm_definition_seed.rs 的
//!    出厂种子在初始化链路与启动链路写入 bpm_process_definition，因审批人锚点必须是已存在的
//!    admin 启用用户，而迁移执行时该用户尚未创建。迁移内绝不 UPDATE/DELETE 历史明文令牌，
//!    老行按 token_expires_at 自然过期、校验侧对 token_hash IS NULL 回退明文比对。依赖 system
//!    建的 bpm_process_instance 与 finance 建并 v15 扩展的
//!    export_approval_request 全部早于本域；置于全链尾为新的顶层条目，复用库上必然执行。
//! 35. ar_family_grant: 应收族八枚细键的存量库补授通道（m0125）。八枚资源名
//!    ar-invoices/ar-payments/ar-verifications/ar-collections/ar-aging/ar-daily/ar-monthly/
//!    ar-statistics 已在 PERMISSION_RESOURCES 与 path_utils 消歧表在册，但三通道（init 角色
//!    矩阵、存量库迁移、e2e SEED）此前全空，除超管外任何角色都调不到该族端点。本域先探测
//!    八枚资源名在 role_permissions 的既有行数并逐枚点名缺失，再按 roles.code 为
//!    finance_manager（审核、报表、催收出文共 11 枚）与 accountant（制单与记账共 10 枚）
//!    幂等补授，ON CONFLICT 用唯一索引真实列集合 DO NOTHING，只补缺失行，绝不 UPDATE 或
//!    DELETE 覆盖既有授权，同键显式拒绝行只点名不覆盖；down 按角色码与键的三元组配对收窄
//!    回收。依赖 system 建的 roles 与 role_permissions 及 m_v15_domain 建的
//!    uq_role_permissions_role_resource_action，全部早于本域；置于全链尾为新的顶层条目，
//!    复用库上必然执行。三通道同口径由 backend/tests/
//!    contract_ar_family_grant_channels_test.rs 双向钉住。

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
            Box::new(domain::fulfillment_source_and_po_origin::Migration),
            Box::new(domain::finance_resource_rename::Migration),
            Box::new(domain::bpm_approver_grant::Migration),
            Box::new(domain::system_admin_bpm_config_grant::Migration),
            Box::new(domain::ap_input_tax_subject::Migration),
            Box::new(domain::ap_family_grant::Migration),
            Box::new(domain::purchase_greige_read_grant::Migration),
            Box::new(domain::role_alias_withdrawal::Migration),
            Box::new(domain::customer_credit_name_backfill::Migration),
            Box::new(domain::customer_credit_remark_column::Migration),
            Box::new(domain::customer_credit_family_grant::Migration),
            Box::new(domain::customers_nullable_columns_norm::Migration),
            Box::new(domain::status_vocab_check::Migration),
            Box::new(domain::wage_energy_vocab_check::Migration),
            Box::new(domain::contract_template_schema::Migration),
            Box::new(domain::m0104_add_document_no_partial_unique_four_tables::Migration),
            Box::new(domain::sales_contract_source_order::Migration),
            // 产品主档规格扩展列，链尾独立域（products 与 product_colors 由 business 域先建）
            Box::new(domain::product_spec_extension_columns::Migration),
            // 产品主档规格扩展列取值域收口，晚于加列域（先加列，再点名，再收口）
            Box::new(domain::product_vocab_check::Migration),
            // 产品每卷米数列探测式下线，晚于代码面摘除读写（先改代码，再收列形态）
            Box::new(domain::product_roll_column_drop::Migration),
            // 合同条款快照来源指针，链尾独立域，只依赖 contract_template_schema 建的快照表
            Box::new(domain::contract_clause_snapshot_provenance::Migration),
            Box::new(domain::print_template_default_slot::Migration),
            Box::new(domain::textile_safety::Migration),
            Box::new(domain::purchase_inspection_result_check::Migration),
            Box::new(domain::export_approval_lifecycle::Migration),
            // 应收族八枚细键的存量库补授通道，链尾独立域（只依赖 system 建的 roles 与
            // role_permissions 与 m_v15_domain 建的同键唯一索引，全部早于本域）
            Box::new(domain::ar_family_grant::Migration),
        ]
    }
}

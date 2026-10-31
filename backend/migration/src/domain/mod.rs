//! 迁移域分组
//!
//! 按业务域组织，每个域 1 个聚合迁移文件：
//! - system: 核心表（用户/角色/部门/产品/供应商/客户/库存/财务基础）+ 修复
//! - business: 业务表（生产/采购/库存扩展/销售/财务扩展/BPM/通知/报表）
//! - sales_crm: 销售报价/CRM/色价
//! - production: 生产/质量/委外/事件/failover
//! - finance: 合规/权限/RLS/色卡/坏账/8D
//! - v15: V15 各期扩展（core 与 extensions 与 final 等合并）
//! - rls_dept: RLS dept 数据范围语义扩展（5 表冗余 department_id + 触发器 + 策略重写）
//! - crm_lead_claim_record: crm_lead 领取事件列（last_claimed_at/by，公海保护期与每日计数的唯一可靠判据）
//! - crm_vocab_check: CRM 状态词表 DB CHECK 约束（crm_lead/crm_opportunity，依赖 business 建表）
//! - price_vocab_check: 价格状态词表 DB CHECK 约束（sales_prices/purchase_prices，依赖 business
//!   建表与 v15 的默认值收敛/'ACTIVE' 回填）
//! - price_fk: 价格两表引用列外键（依赖 system 的 products/customers/suppliers 建表；有孤儿行
//!   时守卫 RAISE EXCEPTION 中止，不在迁移内洗数据）
//! - export_inspection_vocab_check: 出口商检结论词表 DB CHECK 约束（export_inspection.result，
//!   依赖 `migration/src/domain/v15/` 建表，排链尾满足"建表先于 CHECK"原则，全新库与存量库均不违反 CHECK）
//! - ddl_nullability_tighten: 模型非 Option 而列可空的落差收紧（订单金额/色号、标志与主数据
//!   状态、权重与出站开关三组，注册全链尾，角色/建表域均早于它）
//! - dye_recipe_reject: 染色配方拒绝通道（chk_dye_recipe_status 同约束名扩 rejected +
//!   rejected_reason 专列，必须排全链尾：v15 域尾重建四值 CHECK，早于它执行会被覆盖回窄集）
//! - polymorphic_bill_id_widen: 把八列多态单据 ID（各表 `source_bill_id` / `business_id`，
//!   模型均为 i64）从 INTEGER 统一放宽为 BIGINT，使按 i64 绑定时与列宽一致；置于链尾为新的
//!   顶层条目，复用库上也必然执行，幂等可重放
//! - print_subsystem: 打印子系统建表（print_templates 模板主数据 + print_tasks 任务台账，
//!   任务表外键引用模板表；置于链尾为新的顶层条目，复用库上也必然执行）
//! - company_profile: 公司档案单例表（company_profile，id 恒 1 的抬头主数据，建表即播种；
//!   置于链尾为新的顶层条目，复用库上也必然执行）
//! - bulk_color_item_link: 批色记录挂销售订单明细行（bulk_color_approval 补可空列
//!   sales_order_item_id 外键 + 检索索引；置于链尾为新的顶层条目，复用库上也必然执行）
//! - role_conflicts_drop_production_sod: 撤掉 role_conflicts 里 2 对内置「生产↔质量」SoD
//!   默认（一人多岗为纺织厂常态），财务钱账分管 6 对保持内置默认；按精确三元组删除并
//!   在删除前后各做一次财务 6 对在场自检；置于链尾为新的顶层条目，复用库上也必然执行
//!   （并入已应用的 finance 域会在复用库上被整体跳过，删种永不生效）
//! - fulfillment_source_and_po_origin: 流转链地基两列（sales_order_items 补可空链来源列
//!   fulfillment_source 带三值 CHECK + purchase_orders 补可空转采来源销售订单列
//!   source_sales_order_id 外键与检索索引；置于链尾为新的顶层条目，复用库上也必然执行）
//! - finance_resource_rename: 财务域权限资源键改名波（role_permissions.resource_type 旧 URL 段名
//!   逐对改写到随域改名新资源名，唯一索引冲突预检 + 一对多按裸值无法判域的失败可见闸门；
//!   置于链尾为新的顶层条目，复用库上也必然执行）
//! - bpm_approver_grant: BPM 审批岗补授（sales_manager / purchase_manager / production_manager
//!   各六枚派生键）的存量库通道——按 roles.code 反查 id、ON CONFLICT 用真实唯一索引的列集合
//!   保证幂等，逐键打印实写行数与跳过成因，同键显式拒绝行只点名不覆盖；置于链尾为新的顶层
//!   条目，复用库上也必然执行
//! - system_admin_bpm_config_grant: 系统管理岗补授 BPM 流程配置读面（bpm-definitions 与
//!   bpm-templates 各一枚 read 键）的存量库通道——同一套按码反查、幂等补授与逐键打印口径；
//!   置于链尾为新的顶层条目，复用库上也必然执行
//! - ap_family_grant: 应付族授权面补授（finance_manager 的审核与报表面 11 枚、accountant 的
//!   制单与记账面 13 枚）的存量库通道——同一套按码反查、幂等补授与逐键打印口径，回退按
//!   角色码与键的三元组配对收窄；置于链尾为新的顶层条目，复用库上也必然执行
//! - purchase_greige_read_grant: 采购两岗补授 greige-fabrics 只读键的存量库通道——原嵌在
//!   business 域 up 链尾，复用库上被整域跳过，升为顶层条目后必然执行；回退只按本域角色码
//!   回收该键，不动 m0097 授给生产与仓储岗的同名键
//! - customer_credit_name_backfill: 客户信用评级冗余列 customer_name 的存量回填通道——按
//!   customer_id 关联 customers 取真名写入，取不到真名的行保持 NULL 并点名计数（拒绝造名）；
//!   备份将被改写的行 的原值以支持精确回退；置于链尾为新的顶层条目，复用库上也必然执行
//! - customer_credit_remark_column: 客户信用评级备注兑现为真列（customer_credit_ratings 补
//!   可空列 remark VARCHAR(500)，宽度与请求 DTO 的字段校验同值）；加列前先查
//!   information_schema 走已存在则跳过的幂等路径并打印两条路径；置于链尾为新的顶层条目，
//!   复用库上也必然执行
//! - customer_credit_family_grant: 客户信用族动作键补授财务两岗（finance_manager 五枚
//!   read、create、update、delete、release，accountant 三枚 read、create、update）的存量库
//!   通道——同一套按 roles.code 反查 id、ON CONFLICT 用真实唯一索引列集合保证幂等、逐键打印
//!   实写行数与跳过成因、同键显式拒绝行只点名不覆盖；down 按三元组配对且只删 allowed=true 行；
//!   依赖 domain/v15 建的 uq_role_permissions_role_resource_action 唯一索引，置于链尾为新的
//!   顶层条目，复用库上也必然执行
//! - customers_nullable_columns_norm: customers 的 status/payment_terms 两列可空无默认 →
//!   回填成文缺省（'active' / 30）+ SET DEFAULT + SET NOT NULL，与模型非 Option 对齐；
//!   置于链尾为新的顶层条目，复用库上也必然执行（并入已应用的 system 域会被台账跳过）
//! - contract_template_schema: 合同模板体系数据地基域。四张新表加两侧合同主表补列，
//!   一支域文件一份词表常量，条款正文不在本域写入。contract_templates 承载模板版本与
//!   「同一类别同一立场只允许一版启用」的部分唯一 ACTIVE 门；clause_library 是一枚条款码
//!   一枚可复用正文的条款库；contract_template_clauses 是模板下的条款项，条款库引用与
//!   模板内正文二选一，带顺序、必填标记、条件出条款标记与逐条法务复核位；contract_clauses
//!   是签约时固化的条款快照，存固化当时的正文原文、标题、顺序、来源模板与模板版本号，
//!   模板改版不回溯改写已固化行（合同实例只存 template_id 不足以回答当时签的是哪一版哪一条）。
//!   两侧合同主表补出的条款、签署、印花税、内外销与含税列一律可空且不带默认，历史合同保持
//!   空值即「模板前签约，条款未固化」的判据，域内绝不 UPDATE 或 DELETE 业务数据。
//!   置于链尾为新的顶层条目，复用库上属 pending 必然执行；并入已应用的域会被台账整体跳过，
//!   四张表与补列全部缺席。台账名取域名 m_contract_template_schema
//! - m0104_add_document_no_partial_unique_four_tables: 单据号四列（production_recipe.recipe_no /
//!   production_recipe_addition.addition_no / process_wage_rate.rate_no / wage_record.record_no）
//!   补活跃行部分唯一索引（谓词与读路径判重同源 is_deleted = false），先于服务侧取号-插入
//!   同事务改造的 DB 先决约束；存量重复 fail-visible 中止不洗数据，down 显式 DROP INDEX；
//!   四表均由 v15 域建表，置于链尾为新的顶层条目（链尾执行时表必然已存在，无跨域尾调问题；
//!   并入已应用域会被台账整体跳过，复用库永不生效），复用库上也必然执行

//! - sales_contract_source_order: 销售合同与合同明细补 source_order_id 与
//!   source_order_item_id 两条溯源列（外键随源单删除置空），明细行另补折扣率与折扣额与税率
//!   与税额四列和行级色号列，两表各补一条检索索引；新增列一律可空且不带默认，历史合同保持
//!   空值即「非订单转来」的判据，域内绝不 UPDATE 或 DELETE 业务数据。置于链尾为新的顶层
//!   条目，台账名取域名 m_sales_contract_source_order
//! - contract_clause_snapshot_provenance: 合同条款快照来源指针独立域。给 contract_clauses 追加
//!   一列可空无默认的 template_clause_id INTEGER，记录固化当时引用的模板条款项行主键，
//!   供详情腿单义回查必填标记。为什么另立独立域 该列本可塞进 contract_template_schema 域尾，
//!   但后者已在复用测试库上入账，域内新增子迁移在复用库上永不重放，只有让本域名处于 pending
//!   才能被 migrate run 拾起。up 与 down 互逆（加一列撤一列），绝不 UPDATE、不回填、不建外键、
//!   不建索引，历史快照行留空即「无溯源指针」，读侧对空指针走 (模板, 条款码) 唯一行回落，
//!   多义如实给空值不伪造。台账名取域名 m_contract_clause_snapshot_provenance，
//!   挂在链尾 contract_template_schema 之后
//! - print_template_default_slot: 打印模板默认位库侧唯一门域。给 print_templates 按单据
//!   类型 doc_type 建"启用且默认至多一行"的部分唯一索引 uq_print_template_active_default，
//!   谓词三条件与出件解析过滤逐字符同源；存量违例只读探测 fail-visible 中止，不 UPDATE
//!   不 DELETE 洗数据；并更正 is_default 列注释，如实描述互斥域是 doc_type 而非 module。
//!   置于链尾为新的顶层条目，复用库上属 pending 必然执行；并入已应用的 print_subsystem 域
//!   会被台账整体跳过，索引与注释更正永不落地。台账名取域名 m_print_template_default_slot
//! - textile_safety: 纺织品安全合规域三支同域迁移（m0110 建八张表、m0111 词表 CHECK 收口、
//!   m0112 GB 18401 与 GB 31701 限值矩阵与出口要求播种）；新建顶层域，与既有七张检验表零
//!   交集，仅外键只读关联 products（依赖 system 建表，置于链尾为新的顶层条目）
//! - purchase_inspection_result_check: purchase_inspection.inspection_result 值域收口两支
//!   同域迁移（m0113a 严格只读逐值点名、m0113b 守卫后施加 CHECK），词表权威在
//!   models::status::purchase_inventory::purchase_inspection_result；置于链尾为新的顶层条目
//! - export_approval_lifecycle: 导出审批令牌生命周期改造域（export_approval_request 补
//!   BPM 关联/令牌哈希化/重试窗口列 + status CHECK 扩 consumed、exhausted；本域只改
//!   schema，export_approval 流程定义行由 services/bpm_definition_seed.rs 出厂种子登记；
//!   依赖 system 建的 bpm_process_instance、v15 建的 export_approval_request 与
//!   download_token 等既有列，故必须排全链尾为新的顶层条目，复用库上也必然执行）

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
pub mod ddl_nullability_tighten;
pub mod polymorphic_bill_id_widen;
pub mod print_subsystem;
pub mod company_profile;
pub mod bulk_color_item_link;
pub mod role_conflicts_drop_production_sod;
pub mod fulfillment_source_and_po_origin;
pub mod finance_resource_rename;
pub mod bpm_approver_grant;
pub mod system_admin_bpm_config_grant;
pub mod ap_input_tax_subject;
pub mod ap_family_grant;
pub mod purchase_greige_read_grant;
pub mod role_alias_withdrawal;
pub mod customer_credit_name_backfill;
pub mod customer_credit_remark_column;
pub mod customer_credit_family_grant;
pub mod customers_nullable_columns_norm;
pub mod status_vocab_check;
pub mod wage_energy_vocab_check;
pub mod contract_template_schema;
pub mod m0104_add_document_no_partial_unique_four_tables;
pub mod sales_contract_source_order;
// 产品主档规格扩展列。独立顶层域，理由见域头（并入 business 域会在复用库被整体跳过）。
pub mod product_spec_extension_columns;
// 产品主档规格扩展列的词表收口域，晚于上一域（先加列，再点名，再收口）。
pub mod product_vocab_check;
// 产品每卷米数列探测式下线域，晚于代码面下线（先改代码摘除读写，再收列形态）。
pub mod product_roll_column_drop;
// 合同条款快照来源指针域。独立顶层域，理由见域头（域内新增子迁移在复用库永不重放）。
pub mod contract_clause_snapshot_provenance;
pub mod print_template_default_slot;
pub mod textile_safety;
pub mod purchase_inspection_result_check;
pub mod export_approval_lifecycle;
// 产品等级列的词表收口域。独立顶层域，理由见域头（并入已应用域会被台账整体跳过）。
pub mod product_grade_vocab_check;
// 仓储两岗补授扫码入库细键的存量库通道。独立顶层域，理由与 purchase_greige_read_grant 一致
// （并入已应用域会在复用库被整体跳过，补授永不执行）。
pub mod scan_receipt_grant;

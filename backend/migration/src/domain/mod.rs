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
//! - finance_resource_rename: 财务域权限资源键改名波（role_permissions.resource_type 旧 URL 段名
//!   逐对改写到随域改名新资源名，唯一索引冲突预检 + 一对多按裸值无法判域的失败可见闸门；
//!   置于链尾为新的顶层条目，复用库上也必然执行）
//! - bpm_approver_grant: BPM 审批岗补授（sales_manager / purchase_manager / production_manager
//!   各六枚派生键）的存量库通道——按 roles.code 反查 id、ON CONFLICT 用真实唯一索引的列集合
//!   保证幂等，逐键打印实写行数与跳过成因，同键显式拒绝行只点名不覆盖；置于链尾为新的顶层
//!   条目，复用库上也必然执行

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
pub mod finance_resource_rename;
pub mod bpm_approver_grant;

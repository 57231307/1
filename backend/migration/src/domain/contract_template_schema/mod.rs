//! 合同模板体系建表域
//!
//! 本域把“合同约定内容”的数据地基一次落齐,登记为一条顶层迁移台账。域内五支按固定次序执行,
//! 先建被引用表再建引用表,最后给两侧合同主表补列。
//! - m0114_create_contract_templates 模板主表,承载模板编码、合同类别、我方立场、版本号、
//!   状态与 ACTIVE 互斥位、生效日、创建与法务复核留痕。
//! - m0115_create_clause_library 条款库主数据,一枚条款码对应一枚可复用正文。
//! - m0116_create_contract_template_clauses 模板下的条款项,引用条款库或自带正文二选一,
//!   带顺序、必填标记、条件出条款标记与逐条法务复核位。
//! - m0117_create_contract_clauses 签约时固化到合同实例的条款快照,存固化当时的正文原文、
//!   标题、条款码、来源模板与模板版本号,模板改版不回溯改写已固化行。
//! - m0118_add_contract_main_clause_columns 两侧合同主表条款与签署与印花税列补齐,
//!   新增列一律可空且无默认,历史行保持空值。
//!
//! 台账名取域名,注册在 lib.rs 迁移链尾。本域是新顶层条目而非并入既有域,并入已应用的域
//! 在复用库上会因台账已记账而被整体跳过,导致四张表与补列全部缺席。
//!
//! 词表口径,取值集合在本文件单点定义,域内各支只引用常量、不再内联第二套清单。
//! 类别与立场与模板状态是模板侧词表,条款码是条款库词表,合同侧词表区分销售与采购两族合同。
//! 本域只落结构与约束与词表 CHECK,条款正文由模板内容域写入,读写腿由后端服务层实现。
//!
//! 全部迁移形态为“先探测后约束”的失败可见写法。探测到不满足约束的既有数据即 RAISE EXCEPTION
//! 点名样例,绝不静默放宽约束,绝不 UPDATE 或 DELETE 业务数据把存量圆场。
//! 新增列一律可空且不带 DEFAULT,历史合同保持空值即“模板前签约,条款未固化”的判据。

use sea_orm_migration::prelude::*;

mod m0114_create_contract_templates;
mod m0115_create_clause_library;
mod m0116_create_contract_template_clauses;
mod m0117_create_contract_clauses;
mod m0118_add_contract_main_clause_columns;

/// 合同类别词表,十类,与模板清单逐一对应。
/// 内销销售 出口销售 面料与纱线采购 辅料与包装物采购 外协加工 染整加工
/// 劳务与用工 货物运输 仓储与堆存 保密与年度框架。
pub(crate) const CONTRACT_TYPE_ALLOWED_SQL: &str = "'domestic_sales','export_sales','fabric_yarn_purchase','accessory_packaging_purchase','outsourcing_processing','dyeing_finishing','labor_employment','freight_transport','warehousing_storage','confidentiality_framework'";

/// 我方立场词表。卖方与买方决定条款倾向,双方对等义务的合同族取 both。
pub(crate) const PARTY_SIDE_ALLOWED_SQL: &str = "'seller','buyer','both'";

/// 模板与条款库共用的生命周期词表。每类模板都必须 draft 起步,法务复核通过后才能置 active,
/// 被新版本取代后置 retired。
pub(crate) const TEMPLATE_STATUS_ALLOWED_SQL: &str = "'draft','active','retired'";

/// 条款码闭合集,十六枚必备条款加三枚专章条款。必备集缺任一项即该模板不完整,
/// 完整性判定需要跨行聚合,属服务层与活跃门的职责,不在建表迁移内代判。
/// 需要新增条款码时按同族先例另立扩集迁移,不在本处放宽成自由文本。
pub(crate) const CLAUSE_CODE_ALLOWED_SQL: &str = "'subject_and_spec','quantity_and_tolerance','price_and_adjustment','payment_and_invoice','delivery_and_risk','acceptance_and_objection','ip_and_sample_confidentiality','breach_liability_and_loss','force_majeure_and_change_of_circumstance','termination_and_continued_performance','dispute_resolution_and_governing_law','notice_and_service','taxes_and_documents','integrity_and_personal_info','effectiveness_and_signature','annex_and_priority','export_special_terms','mandatory_safety_standard','export_control_screening'";

/// 逐条法务复核状态词表。pending 之外必须有复核人留痕。
pub(crate) const LEGAL_REVIEW_STATUS_ALLOWED_SQL: &str = "'pending','approved','rejected'";

/// 条件出条款标记词表。空值表示无条件通用条款,非空值表示只在命中该业务形态时出条款。
pub(crate) const APPLIES_WHEN_ALLOWED_SQL: &str =
    "'export','domestic','tax_inclusive','tax_exclusive','custom_made','framework'";

/// 合同侧词表,区分条款快照挂在哪一张合同主表上。
pub(crate) const CONTRACT_KIND_ALLOWED_SQL: &str = "'sales','purchase'";

/// 内外销标记词表,落在两侧合同主表上。空值表示模板前签约未采集。
pub(crate) const CONTRACT_MARKET_ALLOWED_SQL: &str = "'domestic','export'";

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_contract_template_schema"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 建表次序由外键方向决定。模板主表与条款库无外键依赖,先建;
        // 模板条款项同时引用两者;合同条款快照引用模板主表;补列最后,只依赖既有两张合同主表。
        m0114_create_contract_templates::Migration
            .up(manager)
            .await?;
        m0115_create_clause_library::Migration.up(manager).await?;
        m0116_create_contract_template_clauses::Migration
            .up(manager)
            .await?;
        m0117_create_contract_clauses::Migration.up(manager).await?;
        m0118_add_contract_main_clause_columns::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序回滚。先撤补列,再按外键反向逐张删表,不留半张表在库里。
        m0118_add_contract_main_clause_columns::Migration
            .down(manager)
            .await?;
        m0117_create_contract_clauses::Migration
            .down(manager)
            .await?;
        m0116_create_contract_template_clauses::Migration
            .down(manager)
            .await?;
        m0115_create_clause_library::Migration.down(manager).await?;
        m0114_create_contract_templates::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}

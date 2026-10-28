#![allow(dead_code)]
//! 合同模板体系状态词表
//!
//! 本文件收口合同模板体系八组取值域的权威 token。词表唯一事实来源是写入方，模板与条款的
//! 建单、改版、启停、法务复核与签约固化都以本 mod 常量为唯一 token 来源，比较点与门控值
//! 必须逐字符相同。数据库侧 CHECK 约束与各 `ALL` 逐元素相等，由建表迁移
//! `migration/src/domain/contract_template_schema/mod.rs` 的八枚词表常量与契约锁用例
//! `tests/contract_template_schema_migration_lock_test.rs` 锁定，两侧任一侧单独增删值判红。
//!
//! 取值域清单
//! - `contract_type`，合同类别，闭合十值，对应模板清单十类
//! - `contract_party_side`，我方立场，闭合三值
//! - `contract_template_status`，模板与条款库共用的生命周期，闭合三值
//! - `contract_clause_code`，条款码闭合集，十六枚必备条款加三枚专章条款
//! - `contract_legal_review_status`，逐条法务复核状态，闭合三值
//! - `contract_applies_when`，条件出条款标记，闭合六值，列可空
//! - `contract_side_kind`，条款快照的合同侧别，闭合两值
//! - `contract_market`，内外销标记，闭合两值，列可空
//!
//! 口径
//! - 取值为全小写下划线纯 ASCII，数据库 CHECK 逐字符敏感，禁止中文值、大写值与大小写归一。
//! - NULL 不进 `ALL`。可空列的 NULL 由调用方在 `Option` 层单独放行，两侧都不必再写
//!   `OR ... IS NULL`，那会让取值清单在两处出现、破坏同源形态。
//! - 需要新增取值时按同族先例另立扩集迁移，在同一约束名上重建 CHECK，
//!   不在应用侧放宽而库里收紧。

/// 合同类别。十类,与模板清单逐一对应。
pub mod contract_type {
    /// 内销销售合同,我方卖方
    pub const DOMESTIC_SALES: &str = "domestic_sales";

    /// 出口销售合同,含贸易术语、结汇、报关主体与退税归属专章
    pub const EXPORT_SALES: &str = "export_sales";

    /// 面料与纱线采购合同,我方买方
    pub const FABRIC_YARN_PURCHASE: &str = "fabric_yarn_purchase";

    /// 辅料与包装物采购合同
    pub const ACCESSORY_PACKAGING_PURCHASE: &str = "accessory_packaging_purchase";

    /// 外协加工合同,裁剪与缝制等
    pub const OUTSOURCING_PROCESSING: &str = "outsourcing_processing";

    /// 染整加工合同,来料加工
    pub const DYEING_FINISHING: &str = "dyeing_finishing";

    /// 劳务与用工合同
    pub const LABOR_EMPLOYMENT: &str = "labor_employment";

    /// 货物运输合同
    pub const FREIGHT_TRANSPORT: &str = "freight_transport";

    /// 仓储与堆存合同
    pub const WAREHOUSING_STORAGE: &str = "warehousing_storage";

    /// 保密协议与年度框架合同
    pub const CONFIDENTIALITY_FRAMEWORK: &str = "confidentiality_framework";

    /// 本列全部非 NULL 合法取值,入参校验与 DB CHECK 的唯一取值来源。
    pub const ALL: &[&str] = &[
        DOMESTIC_SALES,
        EXPORT_SALES,
        FABRIC_YARN_PURCHASE,
        ACCESSORY_PACKAGING_PURCHASE,
        OUTSOURCING_PROCESSING,
        DYEING_FINISHING,
        LABOR_EMPLOYMENT,
        FREIGHT_TRANSPORT,
        WAREHOUSING_STORAGE,
        CONFIDENTIALITY_FRAMEWORK,
    ];

    /// 白名单判定,逐字符匹配,不做大小写或中英转换。
    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 我方立场,决定条款倾向。
pub mod contract_party_side {
    /// 我方收款供方一侧
    pub const SELLER: &str = "seller";

    /// 我方付款需方一侧
    pub const BUYER: &str = "buyer";

    /// 双方义务对等,保密协议与年度框架合同用本值,强行归到单侧会拆成两半并各自漂移
    pub const BOTH: &str = "both";

    pub const ALL: &[&str] = &[SELLER, BUYER, BOTH];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 模板与条款库共用的生命周期。
pub mod contract_template_status {
    /// 草拟中,法务复核未通过,不得被合同选用
    pub const DRAFT: &str = "draft";

    /// 启用中,同一类别同一立场同时只允许一版
    pub const ACTIVE: &str = "active";

    /// 已停用,被新版取代或主动下线
    pub const RETIRED: &str = "retired";

    pub const ALL: &[&str] = &[DRAFT, ACTIVE, RETIRED];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 条款码闭合集,十六枚必备条款加三枚专章条款。
/// 必备集缺任一项即该模板不完整,完整性判定属服务层的活跃门职责。
pub mod contract_clause_code {
    pub const SUBJECT_AND_SPEC: &str = "subject_and_spec";
    pub const QUANTITY_AND_TOLERANCE: &str = "quantity_and_tolerance";
    pub const PRICE_AND_ADJUSTMENT: &str = "price_and_adjustment";
    pub const PAYMENT_AND_INVOICE: &str = "payment_and_invoice";
    pub const DELIVERY_AND_RISK: &str = "delivery_and_risk";
    pub const ACCEPTANCE_AND_OBJECTION: &str = "acceptance_and_objection";
    pub const IP_AND_SAMPLE_CONFIDENTIALITY: &str = "ip_and_sample_confidentiality";
    pub const BREACH_LIABILITY_AND_LOSS: &str = "breach_liability_and_loss";
    pub const FORCE_MAJEURE_AND_CHANGE_OF_CIRCUMSTANCE: &str =
        "force_majeure_and_change_of_circumstance";
    pub const TERMINATION_AND_CONTINUED_PERFORMANCE: &str = "termination_and_continued_performance";
    pub const DISPUTE_RESOLUTION_AND_GOVERNING_LAW: &str = "dispute_resolution_and_governing_law";
    pub const NOTICE_AND_SERVICE: &str = "notice_and_service";
    pub const TAXES_AND_DOCUMENTS: &str = "taxes_and_documents";
    pub const INTEGRITY_AND_PERSONAL_INFO: &str = "integrity_and_personal_info";
    pub const EFFECTIVENESS_AND_SIGNATURE: &str = "effectiveness_and_signature";
    pub const ANNEX_AND_PRIORITY: &str = "annex_and_priority";

    /// 十六枚必备条款,缺任一项算模板不完整。
    pub const MANDATORY: &[&str] = &[
        SUBJECT_AND_SPEC,
        QUANTITY_AND_TOLERANCE,
        PRICE_AND_ADJUSTMENT,
        PAYMENT_AND_INVOICE,
        DELIVERY_AND_RISK,
        ACCEPTANCE_AND_OBJECTION,
        IP_AND_SAMPLE_CONFIDENTIALITY,
        BREACH_LIABILITY_AND_LOSS,
        FORCE_MAJEURE_AND_CHANGE_OF_CIRCUMSTANCE,
        TERMINATION_AND_CONTINUED_PERFORMANCE,
        DISPUTE_RESOLUTION_AND_GOVERNING_LAW,
        NOTICE_AND_SERVICE,
        TAXES_AND_DOCUMENTS,
        INTEGRITY_AND_PERSONAL_INFO,
        EFFECTIVENESS_AND_SIGNATURE,
        ANNEX_AND_PRIORITY,
    ];

    /// 出口专章,贸易术语、结汇币种与汇率基准日、报关主体与单证责任、退税收益归属
    pub const EXPORT_SPECIAL_TERMS: &str = "export_special_terms";

    /// 强制安全标准项,按国标 A 类 B 类 C 类与婴幼儿附加要求,不作为可协商下浮项
    pub const MANDATORY_SAFETY_STANDARD: &str = "mandatory_safety_standard";

    /// 出口管制与制裁主体筛查,付款前置条件,不作为商业让步条款
    pub const EXPORT_CONTROL_SCREENING: &str = "export_control_screening";

    /// 本列全部非 NULL 合法取值,入参校验与 DB CHECK 的唯一取值来源。
    pub const ALL: &[&str] = &[
        SUBJECT_AND_SPEC,
        QUANTITY_AND_TOLERANCE,
        PRICE_AND_ADJUSTMENT,
        PAYMENT_AND_INVOICE,
        DELIVERY_AND_RISK,
        ACCEPTANCE_AND_OBJECTION,
        IP_AND_SAMPLE_CONFIDENTIALITY,
        BREACH_LIABILITY_AND_LOSS,
        FORCE_MAJEURE_AND_CHANGE_OF_CIRCUMSTANCE,
        TERMINATION_AND_CONTINUED_PERFORMANCE,
        DISPUTE_RESOLUTION_AND_GOVERNING_LAW,
        NOTICE_AND_SERVICE,
        TAXES_AND_DOCUMENTS,
        INTEGRITY_AND_PERSONAL_INFO,
        EFFECTIVENESS_AND_SIGNATURE,
        ANNEX_AND_PRIORITY,
        EXPORT_SPECIAL_TERMS,
        MANDATORY_SAFETY_STANDARD,
        EXPORT_CONTROL_SCREENING,
    ];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 逐条法务复核状态。
pub mod contract_legal_review_status {
    /// 未复核,置 active 前必须离开本值
    pub const PENDING: &str = "pending";

    /// 复核通过
    pub const APPROVED: &str = "approved";

    /// 复核驳回
    pub const REJECTED: &str = "rejected";

    pub const ALL: &[&str] = &[PENDING, APPROVED, REJECTED];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 条件出条款标记,空值表示无条件通用条款。
pub mod contract_applies_when {
    pub const EXPORT: &str = "export";
    pub const DOMESTIC: &str = "domestic";
    pub const TAX_INCLUSIVE: &str = "tax_inclusive";
    pub const TAX_EXCLUSIVE: &str = "tax_exclusive";
    pub const CUSTOM_MADE: &str = "custom_made";
    pub const FRAMEWORK: &str = "framework";

    pub const ALL: &[&str] = &[
        EXPORT,
        DOMESTIC,
        TAX_INCLUSIVE,
        TAX_EXCLUSIVE,
        CUSTOM_MADE,
        FRAMEWORK,
    ];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 条款快照的合同侧别,区分快照挂在哪一张合同主表上。
pub mod contract_side_kind {
    /// 指向 sales_contracts
    pub const SALES: &str = "sales";

    /// 指向 purchase_contracts
    pub const PURCHASE: &str = "purchase";

    pub const ALL: &[&str] = &[SALES, PURCHASE];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

/// 内外销标记,落在两侧合同主表上,空值表示模板前签约未采集。
pub mod contract_market {
    pub const DOMESTIC: &str = "domestic";
    pub const EXPORT: &str = "export";

    pub const ALL: &[&str] = &[DOMESTIC, EXPORT];

    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }
}

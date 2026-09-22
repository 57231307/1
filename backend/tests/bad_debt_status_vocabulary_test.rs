//! 坏账 / 应收 / 委外 / 合同 / AI 模型「状态比较点 ↔ 写入点 ↔ DB CHECK」同源性回归
//! （确定性文本+常量比对，不依赖活库，CI 常规可跑）。
//!
//! 缺陷族：状态「比较点」与「写入点」大小写/词表不同源。本测试锁定修复后的不变量：
//! - ar_invoice.approval_status 写入点用大写（common::STATUS_APPROVED），
//!   坏账核销与 B01 计提扫描的比较点必须引用同一常量（历史缺陷：比较点写小写 "approved"，
//!   结构性永不匹配 → 坏账核销不可达）；
//! - bad_debt 自有表（provisions/writeoffs）比较点与写入点均收口到词表模块，
//!   且词表全集与迁移 CHECK 取值集逐项相等；
//! - 委外报表、合同状态门、AI 模型状态门均改引常量、去裸字面量。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::{bad_debt_provision_status, bad_debt_writeoff_status, common};

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 从迁移文本抽出一个 `CHECK ("col" IN ('a', 'b', ...))` 的取值集合。
fn check_in_values(sql: &str, constraint: &str) -> Vec<String> {
    let re = regex::Regex::new(&format!(
        r#"(?s)CONSTRAINT "{}"[\s\S]*?IN \(([^)]*)\)"#,
        regex::escape(constraint)
    ))
    .unwrap();
    let cap = re
        .captures(sql)
        .unwrap_or_else(|| panic!("迁移未找到 {} 的 CHECK ... IN (...)", constraint));
    regex::Regex::new(r"'([^']*)'")
        .unwrap()
        .captures_iter(&cap[1])
        .map(|c| c[1].to_string())
        .collect()
}

/// ar_invoice.approval_status 写入点词表为大写：确认常量值，锁定同源基准。
#[test]
fn ar_invoice_approval_word_list_is_uppercase() {
    assert_eq!(common::STATUS_APPROVED, "APPROVED");
    assert_eq!(common::STATUS_PENDING, "PENDING");
    assert_eq!(common::STATUS_DRAFT, "DRAFT");
}

/// 坏账核销服务对 ar_invoice.approval_status 的比较点必须引用 common::STATUS_APPROVED，
/// 不得残留小写 "approved" 裸字面量（历史缺陷：结构性永不匹配）。
#[test]
fn bad_debt_compares_ar_invoice_with_uppercase_constant() {
    let svc = read_rel("src/services/bad_debt_service.rs");
    assert!(
        svc.contains("invoice.approval_status != common::STATUS_APPROVED"),
        "create_writeoff 比较点必须以 common::STATUS_APPROVED 为源"
    );
    assert!(
        svc.contains("Column::ApprovalStatus.eq(common::STATUS_APPROVED)"),
        "run_monthly_provision 扫描比较点必须以 common::STATUS_APPROVED 为源"
    );
    assert!(
        !svc.contains(r#"ApprovalStatus.eq("approved")"#),
        "不得残留小写 \"approved\" 字面量比较 ar_invoice.approval_status"
    );
    assert!(
        !svc.contains(r#"approval_status != "approved""#),
        "不得残留小写 \"approved\" 字面量比较"
    );
}

/// 迁移 crate 位于 `backend/migration`（workspace members = [".", "migration"]），
/// 而 CARGO_MANIFEST_DIR 就是 `backend`，所以路径是 `migration/...` 而非 `../migration/...`。
const V15_MIGRATION: &str = "migration/src/domain/v15/mod.rs";

/// bad_debt_provisions 词表全集必须与迁移 chk_bdp_status 逐项相等。
#[test]
fn provision_word_list_equals_migration_check() {
    let sql = read_rel(V15_MIGRATION);
    let check = check_in_values(&sql, "chk_bdp_status");
    let mut want: Vec<String> = bad_debt_provision_status::ALL
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut got = check.clone();
    want.sort();
    got.sort();
    assert_eq!(
        got, want,
        "chk_bdp_status 取值集必须与 bad_debt_provision_status::ALL 完全一致"
    );
}

/// bad_debt_writeoffs 词表全集必须与迁移 chk_bdw_status 逐项相等。
#[test]
fn writeoff_word_list_equals_migration_check() {
    let sql = read_rel(V15_MIGRATION);
    let check = check_in_values(&sql, "chk_bdw_status");
    let mut want: Vec<String> = bad_debt_writeoff_status::ALL
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut got = check.clone();
    want.sort();
    got.sort();
    assert_eq!(
        got, want,
        "chk_bdw_status 取值集必须与 bad_debt_writeoff_status::ALL 完全一致"
    );
}

/// 坏账服务对自有表（provisions/writeoffs）的状态读写全部收口到词表常量，无裸字面量。
#[test]
fn bad_debt_own_tables_use_word_list_constants() {
    let svc = read_rel("src/services/bad_debt_service.rs");
    assert!(svc.contains("provision_status::DRAFT"), "计提状态应引常量");
    assert!(
        svc.contains("writeoff_status::FINANCE_APPROVED"),
        "核销状态应引常量"
    );
    // 全文件不得残留这些状态字面量（写入或比较）
    for lit in [
        r#""draft""#,
        r#""confirmed""#,
        r#""reversed""#,
        r#""pending""#,
        r#""finance_approved""#,
        r#""rejected""#,
        r#""cancelled""#,
    ] {
        assert!(!svc.contains(lit), "坏账服务不得残留状态裸字面量 {}", lit);
    }
}

/// 委外报表统计比较点改引 outsourcing_order_status 常量、去裸字面量。
#[test]
fn outsourcing_report_uses_word_list_constants() {
    let h = read_rel("src/handlers/outsourcing_handler.rs");
    assert!(
        h.contains("o.status == outsourcing_order_status::DRAFT"),
        "委外报表应以 outsourcing_order_status::DRAFT 为源"
    );
    for lit in [
        r#"== "draft""#,
        r#"== "issued""#,
        r#"== "processing""#,
        r#"== "received""#,
        r#"== "settled""#,
        r#"== "closed""#,
    ] {
        assert!(!h.contains(lit), "委外报表不得残留状态裸字面量 {}", lit);
    }
}

/// 合同状态门比较点改引 contract::DRAFT 常量（合同词表为小写，写入点同源）。
#[test]
fn contract_handlers_use_word_list_constant() {
    for h in [
        read_rel("src/handlers/purchase_contract_handler.rs"),
        read_rel("src/handlers/sales_contract_handler.rs"),
    ] {
        assert!(
            h.contains("crate::models::status::contract::DRAFT"),
            "合同状态门应以 contract::DRAFT 为源"
        );
        assert!(
            !h.contains(r#"status != "draft""#),
            "合同 handler 不得残留 \"draft\" 裸字面量比较"
        );
    }
}

/// AI 模型状态门改引 master_data 常量；取值域校验对大小写敏感（大写应被拒绝）。
#[test]
fn ai_model_status_validators_are_case_sensitive() {
    use bingxi_backend::services::ai_model_management_service::AiModelManagementService as S;
    // 小写合法值通过
    assert!(S::validate_approval_status("pending").is_ok());
    assert!(S::validate_approval_status("approved").is_ok());
    assert!(S::validate_model_status("active").is_ok());
    // 大写（ar_invoice 词表风格）应被拒绝——证明两套词表未被 to_uppercase 糊在一起
    assert!(S::validate_approval_status("APPROVED").is_err());
    assert!(S::validate_model_status("ACTIVE").is_err());

    let svc = read_rel("src/services/ai_model_management_service.rs");
    assert!(
        svc.contains("master_data::PENDING") && svc.contains("master_data::APPROVED"),
        "AI 模型状态读写应以 master_data 常量为源"
    );
    for lit in [r#""draft""#, r#""active""#, r#""pending""#, r#""approved""#] {
        assert!(
            !svc.contains(lit),
            "AI 模型服务不得残留状态裸字面量 {}",
            lit
        );
    }
}

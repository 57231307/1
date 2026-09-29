//! 报价单状态词表与「生效的」库层 CHECK 同源性回归（确定性文本比对，不依赖活库）。
//!
//! 背景：`sales_quotations` 在迁移里被定义了两次——
//! - `domain/sales_crm/mod.rs`（域执行顺序 system → business → sales_crm → production → …，
//!   它先跑，真正建表）：`status VARCHAR(20) NOT NULL DEFAULT 'draft'` +
//!   `CONSTRAINT "chk_status" CHECK (status IN ('draft','pending_approval','approved',
//!   'rejected','expired','converted','cancelled'))`（7 值，全小写）
//! - `domain/production/m0044_integrate_unreferenced_migrations.rs`（后跑）：同一张表的
//!   `CREATE TABLE IF NOT EXISTS` 带**大写** 7 值集 `chk_quotation_status`
//!   （DRAFT/SUBMITTED/APPROVED/…，且第二个集合用 SUBMITTED 取代 PENDING_APPROVAL）。
//! 因 `IF NOT EXISTS`，m0044 那份在正常迁移路径下是死语句；但它是一份"看起来生效"的
//! 平行词表：任何人按它写查询/校验都会得到永不命中的条件（大写 vs 小写），
//! 或把 SUBMITTED 当成合法态而写入被 `chk_status` 拒绝（23514）。
//! 本测试锁死：Rust 侧词表（`quotation` + `quotation_ext` 合并）必须与**先建表那份**
//! 的 CHECK 取值集逐项相等，并钉住后端写入点不再出现裸字面量。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::quotation as quotation_status;
use bingxi_backend::models::status::quotation_ext as quotation_ext_status;
use regex::Regex;

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 生效词表：Rust 两个模块合并 = 7 值、全小写、含 pending_approval（无 SUBMITTED）。
#[test]
fn rust_vocabulary_covers_all_seven_lowercase_states() {
    let mut all = [
        quotation_status::DRAFT,
        quotation_status::APPROVED,
        quotation_status::REJECTED,
        quotation_status::CANCELLED,
        quotation_ext_status::PENDING_APPROVAL,
        quotation_ext_status::EXPIRED,
        quotation_ext_status::CONVERTED,
    ];
    all.sort_unstable();
    let mut expected = [
        "approved",
        "cancelled",
        "converted",
        "draft",
        "pending_approval",
        "rejected",
        "expired",
    ];
    expected.sort_unstable();
    assert_eq!(
        all, expected,
        "报价单状态词表应恰为 7 个小写取值（缺值或多值都会与库层 chk_status 失配）"
    );
    for s in all {
        assert!(
            s.chars().all(|c| c.is_lowercase() || c == '_'),
            "报价单词表应全小写，发现 {s}"
        );
    }
}

/// 后端写入/校验点不得残留状态裸字面量（历史上审批与状态门各处独立写字面量，
/// 任一侧改值即静默失配）。
#[test]
fn approval_service_has_no_bare_status_literals() {
    let svc = read_rel("src/services/quotation_approval_service.rs");
    for lit in [
        r#""draft""#,
        r#""pending_approval""#,
        r#""approved""#,
        r#""rejected""#,
    ] {
        assert!(
            !svc.contains(lit),
            "quotation_approval_service 不得残留状态裸字面量 {lit}"
        );
    }
    assert!(
        svc.contains("quotation_status::APPROVED")
            && svc.contains("quotation_ext_status::PENDING_APPROVAL"),
        "写入点必须以 quotation / quotation_ext 常量为源"
    );
}

/// 查询点同样以常量为源：报价到期预警扫描按 quotation::APPROVED 过滤，
/// 而非独立的小写字面量（值相同也要同源，否则词表演进时静默漂移）。
#[test]
fn expiry_query_filters_with_constant() {
    let h = read_rel("src/handlers/quotation_handler.rs");
    assert!(
        h.contains("Column::Status.eq(quotation_status::APPROVED)"),
        "到期预警查询必须以 quotation::APPROVED 为源"
    );
    assert!(
        !h.contains(r#"Column::Status.eq("approved")"#),
        "不得残留 \"approved\" 裸字面量查询"
    );
}

/// 生效 CHECK 的取值集必须等于 Rust 词表合并集；同时钉住"另一份大写定义存在但不生效"
/// 这一事实，避免有人误按 m0044 的 SUBMITTED/大写集合写代码。
#[test]
fn effective_check_equals_rust_vocabulary() {
    let sales_crm = read_rel("migration/src/domain/sales_crm/mod.rs");
    let cap = Regex::new(
        r#"(?s)CREATE TABLE IF NOT EXISTS "sales_quotations"[\s\S]*?CONSTRAINT "chk_status" CHECK \("status" IN \(([^)]*)\)\)"#,
    )
    .unwrap()
    .captures(&sales_crm)
    .expect("sales_crm 的 sales_quotations 建表必须含 chk_status CHECK");
    let mut check: Vec<String> = Regex::new(r"'([^']*)'")
        .unwrap()
        .captures_iter(&cap[1])
        .map(|c| c[1].to_string())
        .collect();
    check.sort();

    let mut from_rust: Vec<String> = vec![
        quotation_status::DRAFT,
        quotation_status::APPROVED,
        quotation_status::REJECTED,
        quotation_status::CANCELLED,
        quotation_ext_status::PENDING_APPROVAL,
        quotation_ext_status::EXPIRED,
        quotation_ext_status::CONVERTED,
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    from_rust.sort();

    assert_eq!(
        check, from_rust,
        "生效 chk_status 取值集必须与 Rust 报价词表（quotation + quotation_ext）逐项相等"
    );

    let m0044 =
        read_rel("migration/src/domain/production/m0044_integrate_unreferenced_migrations.rs");
    assert!(
        m0044.contains(
            r#"CONSTRAINT "chk_quotation_status" CHECK ("status" IN ('DRAFT','SUBMITTED'"#
        ),
        "前置事实：m0044 仍带一份大写平行词表（因 IF NOT EXISTS 不生效）；\
         若哪天它变成生效定义，本断言与上面的相等断言会同时把差异暴露出来"
    );
}

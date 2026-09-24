//! CRM 状态词表与「生效的」库层 CHECK 同源性回归（确定性文本+常量比对，不依赖活库）。
//!
//! 背景（漂移长期没被 CI 抓到的根因）：`backend/tests/` 下 bad_debt / bpm / po / quotation /
//! dye_recipe / mrp 等都各有一份 `*status_word_list*` 守卫测试锁死「Rust 常量 == 迁移 CHECK」，
//! **唯独 CRM 没有**。CRM 三套词表（`crm_lead.lead_status`、`crm_opportunity.opportunity_stage`、
//! `crm_opportunity.opportunity_status`）的权威源是 `models/status/bpm_crm_contract.rs` 里的
//! `crm_lead::ALL` / `crm_opportunity::ALL_STAGES` / `ALL_STATUSES`，DB 侧兜底 CHECK 定义在迁移
//! `migration/src/domain/crm_vocab_check/mod.rs`（约束名 `chk_crm_lead_lead_status` /
//! `chk_crm_opportunity_stage` / `chk_crm_opportunity_status`）。两处独立维护极易漂移，
//! 本测试把「常量数组 == CHECK 取值集」逐项钉死，并额外断言词表字面量与大小写规则，
//! 使「后端加值忘补 CHECK」「CHECK 改值忘补常量」「前端漏值」这类漂移在 CI 直接红。
//!
//! 特别锁死 `assigned`：它是后端 `crm_lead::ALL` 与 CHECK 的合法值（`services/crm/assign.rs`
//! 自动分配/认领写入的归属态），历史上一度缺失于前端映射导致列表整页崩——此处从后端侧固定它存在，
//! 前端渲染侧回归见 `frontend/e2e/crm/02-leads.spec.ts` 的 02-06。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::crm_lead;
use bingxi_backend::models::status::crm_opportunity;
use regex::Regex;

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 从 CHECK 的 `IN (...)` 捕获段里抽出全部单引号取值并排序（迁移文本里取值不含括号，
/// 非贪婪 `[^)]*` 恰好停在 IN 列表右括号）。
fn quoted_values(captured: &str) -> Vec<String> {
    sorted_set(
        &Regex::new(r"'([^']*)'")
            .unwrap()
            .captures_iter(captured)
            .map(|c| c[1].to_string())
            .collect::<Vec<String>>(),
    )
}

/// 把一组 `&str` 转成排序后的 `Vec<String>`，供逐项集合比对。
fn sorted_set(vals: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = vals.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

/// `sorted_set` 的 String 版：用于把从迁移抽出的取值（Vec<String>）与字面量集合并列比较。
fn sorted_owned(vals: Vec<String>) -> Vec<String> {
    let mut v = vals;
    v.sort();
    v
}

const VOCAB_CHECKS: &str = "migration/src/domain/crm_vocab_check/mod.rs";

/// crm_lead.lead_status：Rust `crm_lead::ALL` 必须逐项等于生效 CHECK 的取值集，且全小写；
/// `assigned` 必须在其中（历史漏到前端即整页崩，后端侧此处钉权威存在）。
#[test]
fn crm_lead_check_equals_rust_word_list() {
    let src = read_rel(VOCAB_CHECKS);
    let cap = Regex::new(
        r##""chk_crm_lead_lead_status" CHECK \([\s\S]*?"lead_status" IN \(([^)]*)\)"##,
    )
    .unwrap()
    .captures(&src)
    .expect(
        "crm_vocab_check 迁移必须含 chk_crm_lead_lead_status 的 CHECK ... lead_status IN (...)",
    );
    let check = quoted_values(&cap[1]);

    let from_rust = sorted_set(crm_lead::ALL);
    assert_eq!(
        check, from_rust,
        "生效 chk_crm_lead_lead_status 取值集必须与 crm_lead::ALL 逐项相等（漂移即红）"
    );

    // 钉死词表字面量本身（防有人同时改常量和 CHECK 而改错方向）：7 值、全小写。
    let expected = sorted_set(&[
        "new",
        "contacted",
        "qualified",
        "assigned",
        "converted",
        "pool",
        "lost",
    ]);
    assert_eq!(
        from_rust, expected,
        "crm_lead::ALL 字面量应恰为 new/contacted/qualified/assigned/converted/pool/lost"
    );
    assert!(
        crm_lead::ALL.contains(&"assigned"),
        "assigned 属后端权威线索态（assign.rs 写入），不得从词表/CHECK 缺失"
    );
    for s in crm_lead::ALL {
        assert!(
            s.chars().all(|c| c.is_lowercase() || c == '_'),
            "线索词表应全小写，发现 {s}"
        );
    }
}

/// crm_opportunity.opportunity_stage：Rust `ALL_STAGES` 必须等于生效 CHECK 取值集，且全大写。
#[test]
fn crm_opportunity_stage_check_equals_rust_word_list() {
    let src = read_rel(VOCAB_CHECKS);
    let cap = Regex::new(
        r##""chk_crm_opportunity_stage" CHECK \([\s\S]*?"opportunity_stage" IN \(([^)]*)\)"##,
    )
    .unwrap()
    .captures(&src)
    .expect("crm_vocab_check 迁移必须含 chk_crm_opportunity_stage 的 CHECK ... opportunity_stage IN (...)");
    let check = quoted_values(&cap[1]);

    let from_rust = sorted_set(crm_opportunity::ALL_STAGES);
    assert_eq!(
        check, from_rust,
        "生效 chk_crm_opportunity_stage 取值集必须与 crm_opportunity::ALL_STAGES 逐项相等"
    );

    let expected = sorted_owned(vec![
        "QUALIFICATION".into(),
        "NEEDS_ANALYSIS".into(),
        "PROPOSAL".into(),
        "NEGOTIATION".into(),
        "CLOSED_WON".into(),
        "CLOSED_LOST".into(),
    ]);
    assert_eq!(
        from_rust, expected,
        "ALL_STAGES 字面量应恰为 QUALIFICATION/NEEDS_ANALYSIS/PROPOSAL/NEGOTIATION/CLOSED_WON/CLOSED_LOST"
    );
    for s in crm_opportunity::ALL_STAGES {
        assert!(
            s.chars().all(|c| c.is_uppercase() || c == '_'),
            "商机阶段词表应全大写，发现 {s}"
        );
    }
}

/// crm_opportunity.opportunity_status：Rust `ALL_STATUSES` 必须等于生效 CHECK 取值集，且全大写。
/// 词表仅 OPEN/CLOSED_WON/CLOSED_LOST（无 draft/cancelled/pool——公海机制仅存在于 crm_lead）。
#[test]
fn crm_opportunity_status_check_equals_rust_word_list() {
    let src = read_rel(VOCAB_CHECKS);
    let cap = Regex::new(
        r##""chk_crm_opportunity_status" CHECK \([\s\S]*?"opportunity_status" IN \(([^)]*)\)"##,
    )
    .unwrap()
    .captures(&src)
    .expect("crm_vocab_check 迁移必须含 chk_crm_opportunity_status 的 CHECK ... opportunity_status IN (...)");
    let check = quoted_values(&cap[1]);

    let from_rust = sorted_set(crm_opportunity::ALL_STATUSES);
    assert_eq!(
        check, from_rust,
        "生效 chk_crm_opportunity_status 取值集必须与 crm_opportunity::ALL_STATUSES 逐项相等"
    );

    let expected = sorted_owned(vec![
        "OPEN".into(),
        "CLOSED_WON".into(),
        "CLOSED_LOST".into(),
    ]);
    assert_eq!(
        from_rust, expected,
        "ALL_STATUSES 字面量应恰为 OPEN/CLOSED_WON/CLOSED_LOST（幽灵 draft/cancelled/pool 不得回流）"
    );
    for s in crm_opportunity::ALL_STATUSES {
        assert!(
            s.chars().all(|c| c.is_uppercase() || c == '_'),
            "商机状态词表应全大写，发现 {s}"
        );
    }
}

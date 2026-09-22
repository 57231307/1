//! color_cards.status 词表与 DB CHECK 约束一致性回归（确定性文本比对，不依赖活库）。
//!
//! 口径（用户拍板）：闭合词表是 `models/status/wage_energy_chemical_business.rs::color_card`
//! = {draft,issued,received,used,expired,archived,lost}，legacy `active` 等价 draft。
//! 迁移前 CHECK `chk_color_card_status` 仅允许 (active,archived,lost)——
//! 与词表双向越界：多 active（词表不认）、少 draft/issued/received/used/expired
//! （词表认且 service 会写，旧 CHECK 会直接拒绝这些流转）。
//! 本测试断言重建后的 CHECK 取值集与词表 `color_card::ALL` **逐项完全相等**（多一个
//! 少一个都失败），并锁定 service 不再写 legacy active、前端词表与后端一致。
//!
//! 取舍：用比对「迁移 SQL 文本里的 CHECK 取值集」与「Rust 词表常量」代替对活库查
//! pg_get_constraintdef——CI 常规集成测试连空库，直连查询需已迁移 PG 库、脆弱难跑；
//! 二者都是编译/文本期可见的静态事实，比对足以守住一致性。迁移真跑（setup 向导库）
//! 会在 ADD CONSTRAINT 阶段以真实数据二次兜底。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::color_card as card_status;
use regex::Regex;

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 词表全集：7 值、全小写、不含 legacy active。
#[test]
fn word_list_is_closed_lowercase_set() {
    let expected = [
        card_status::DRAFT,
        card_status::ISSUED,
        card_status::RECEIVED,
        card_status::USED,
        card_status::EXPIRED,
        card_status::ARCHIVED,
        card_status::LOST,
    ];
    assert_eq!(card_status::ALL, &expected[..]);
    assert_eq!(card_status::ALL.len(), 7);
    for s in card_status::ALL {
        assert!(
            s.chars().all(|c| c.is_lowercase() || c == '_'),
            "色卡词表应全小写，发现 {}",
            s
        );
        assert_ne!(*s, "active", "legacy active 不得作为词表取值");
    }
    assert_eq!(card_status::DRAFT, "draft");
}

/// 从 v15 迁移文本抽出重建后的 chk_color_card_status CHECK 取值集合。
fn check_values_from_migration() -> Vec<String> {
    let v15 = read_rel("../migration/src/domain/v15/mod.rs");
    assert!(
        v15.contains(r#"DROP CONSTRAINT IF EXISTS "chk_color_card_status""#),
        "v15 必须 DROP 旧 chk_color_card_status 后再 ADD，重建为词表全集"
    );
    let block =
        Regex::new(r#"(?s)ADD CONSTRAINT "chk_color_card_status"[\s\S]*?IN \(([^)]*)\)"#).unwrap();
    let cap = block
        .captures(&v15)
        .expect("未找到重建后的 chk_color_card_status ADD CONSTRAINT ... IN (...)");
    let inner = &cap[1];
    let quoted = Regex::new(r"'([^']*)'").unwrap();
    quoted
        .captures_iter(inner)
        .map(|c| c[1].to_string())
        .collect()
}

/// 迁移 CHECK 取值集与词表 ALL 必须逐项完全相等（多一少一皆失败）。
#[test]
fn check_constraint_equals_word_list() {
    let check = check_values_from_migration();
    let mut want: Vec<String> = card_status::ALL.iter().map(|s| s.to_string()).collect();
    let mut got = check.clone();
    want.sort();
    got.sort();
    assert_eq!(
        got, want,
        "chk_color_card_status 取值集必须与 color_card::ALL 完全一致；CHECK={:?}",
        check
    );
    assert!(
        !check.iter().any(|s| s == "active"),
        "重建后的 CHECK 仍含 legacy active：{:?}",
        check
    );
}

/// 迁移必须把 legacy active 回填为 draft（回填依据：列注释/词表模块说明 active 等价 draft）。
#[test]
fn migration_backfills_active_to_draft() {
    let v15 = read_rel("../migration/src/domain/v15/mod.rs");
    assert!(
        v15.contains(r#"UPDATE "color_cards" SET "status" = 'draft' WHERE "status" = 'active'"#),
        "v15 必须回填 active → draft（先于 CHECK 重建，避免残留行使 ADD CONSTRAINT 失败）"
    );
    assert!(
        v15.contains(r#"ALTER TABLE "color_cards" ALTER COLUMN "status" SET DEFAULT 'draft'"#),
        "v15 必须把 color_cards.status 默认值收敛到 draft"
    );
}

/// service 不得再写/判 legacy active，建单与更新门槛走词表常量。
#[test]
fn service_uses_word_list_not_legacy_active() {
    let svc = read_rel("src/services/color_card_crud_service.rs");
    assert!(
        !svc.contains("master_data::ACTIVE"),
        "色卡 service 不得再引用 master_data::ACTIVE（legacy）"
    );
    assert!(
        !svc.contains(r#""active""#),
        "色卡 service 不得残留裸字面量 \"active\""
    );
    assert!(
        svc.contains("card_status::DRAFT"),
        "建单/更新门槛必须以词表 card_status::DRAFT 为源"
    );
}

/// 前端色卡词表常量与后端一致（含全集、不含 legacy active 键）。
#[test]
fn frontend_status_matches_word_list() {
    let fe = read_rel("../frontend/src/api/color-card.ts");
    let start = fe
        .find("export const COLOR_CARD_STATUS =")
        .expect("前端缺 COLOR_CARD_STATUS 定义");
    let end = fe[start..]
        .find("} as const")
        .map(|i| start + i)
        .expect("COLOR_CARD_STATUS 未闭合");
    let block = &fe[start..end];
    for s in card_status::ALL {
        assert!(
            block.contains(&format!("{}:", s)) || block.contains(&format!("'{}'", s)),
            "前端色卡词表缺状态 {}",
            s
        );
    }
    assert!(
        !block.contains("active:"),
        "前端色卡词表不得含 legacy active 键"
    );
}

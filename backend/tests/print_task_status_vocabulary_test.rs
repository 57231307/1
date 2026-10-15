//! 打印子系统「状态词表 / 后端类型 ↔ 建表迁移 CHECK」同源性守卫（确定性文本比对，不依赖活库）。
//!
//! 锁定不变量：
//! - `print_tasks.status` 的词表全集 `print_task_status::ALL` 与迁移 `chk_print_tasks_status`
//!   的取值集逐项相等；
//! - `print_targets.kind` / `print_tasks.backend_kind` 的合法集合与传输工厂所识别的后端
//!   token 常量（`transport::KIND_*`）相等——三处任一侧单独增删值即判红，杜绝词表漂移；
//! - `print_tasks.target_kind` 为 printer/pool 闭合集合。
//!
//! 判责口径：DB CHECK 与 Rust 侧取值来源必须逐字符相同；本测试把「Rust 唯一事实来源」
//! 锚定到状态词表模块与传输常量，而非另立第二套手写清单。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::print::print_task_status;
use bingxi_backend::services::print_dispatch::transport::{
    KIND_CUPS, KIND_IPP, KIND_NETWORK, KIND_SYSTEM_COMMAND,
};

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 从迁移文本抽出一个具名 CHECK 约束 `... IN ('a', 'b', ...)` 的取值集合。
/// 打印域迁移里约束名不带引号（`CONSTRAINT name CHECK (col IN (...))`），故引号设为可选。
fn check_in_values(sql: &str, constraint: &str) -> Vec<String> {
    let re = regex::Regex::new(&format!(
        r#"(?s)CONSTRAINT\s+"?{}"?\s+CHECK[\s\S]*?IN \(([^)]*)\)"#,
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

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// `print_task_status::ALL` 必须与 `chk_print_tasks_status` 取值集完全一致。
#[test]
fn print_task_status_word_list_equals_migration_check() {
    let sql = read_rel("migration/src/domain/print_subsystem/m0099_create_print_tasks.rs");
    let check = check_in_values(&sql, "chk_print_tasks_status");
    let want: Vec<String> = print_task_status::ALL
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        sorted(check),
        sorted(want),
        "chk_print_tasks_status 取值集必须与 print_task_status::ALL 完全一致"
    );
}

/// 后端类型三处同源：迁移 backend_kind CHECK == 迁移 print_targets.kind CHECK == 传输工厂常量集合。
#[test]
fn backend_kind_sets_are_single_sourced() {
    let factory_tokens: Vec<String> = [KIND_CUPS, KIND_IPP, KIND_NETWORK, KIND_SYSTEM_COMMAND]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let tasks_sql = read_rel("migration/src/domain/print_subsystem/m0099_create_print_tasks.rs");
    let backend_check = check_in_values(&tasks_sql, "chk_print_tasks_backend_kind");

    let targets_sql =
        read_rel("migration/src/domain/print_subsystem/m0100_create_print_targets.rs");
    let kind_check = check_in_values(&targets_sql, "chk_print_targets_kind");

    assert_eq!(
        sorted(backend_check),
        sorted(factory_tokens.clone()),
        "chk_print_tasks_backend_kind 必须与传输工厂识别的后端常量集合一致"
    );
    assert_eq!(
        sorted(kind_check),
        sorted(factory_tokens),
        "chk_print_targets_kind 必须与传输工厂识别的后端常量集合一致"
    );
}

/// `print_tasks.target_kind` 为 printer/pool 闭合集合。
#[test]
fn target_kind_check_is_printer_pool() {
    let sql = read_rel("migration/src/domain/print_subsystem/m0099_create_print_tasks.rs");
    let check = check_in_values(&sql, "chk_print_tasks_target_kind");
    assert_eq!(
        sorted(check),
        sorted(vec!["printer".to_string(), "pool".to_string()]),
        "chk_print_tasks_target_kind 取值集必须恰为 printer/pool"
    );
}

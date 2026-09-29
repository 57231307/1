//! MRP 结果状态词表守卫测试（CI 常规可跑，不依赖活库）
//!
//! 背景：`mrp_results.status` 曾同时存在两套词表——`crate::models::status::mrp` 常量
//! （PLANNED/RELEASED/CONFIRMED/CANCELLED，实际写入用）与同文件里的 `MrpResultStatus`
//! ActiveEnum（PLANNED/CONFIRMED/RELEASED/COMPLETED，**无任何引用**）。两套各缺一个成员，
//! 且该列是 `String` + 索引、没有 CHECK 约束兜住，任何一侧被引用都会写入库里查不到的状态。
//! 现已删除死的 ActiveEnum，词表唯一来源是常量模块。
//!
//! 本测试锁死三条不变量，防止再次漂移：
//! 1. 常量模块里 `pub mod mrp` 的每个常量：名 == 值，且值为大写纯 ASCII。
//! 2. 前端 `api/mrp.ts` 的 `MrpStatus` 联合类型取值集合 == 后端常量取值集合（不丢不多）。
//! 3. MRP 相关的 Rust 写入/比较点不出现裸状态字面量（一律引常量）。

use std::fs;
use std::path::PathBuf;

fn backend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let path = backend_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败: {}", path.display(), e))
}

/// 从 `models/status/production.rs` 中取出 `pub mod mrp { ... }` 里的 (常量名, 值)。
fn backend_mrp_status() -> Vec<(String, String)> {
    let src = read("src/models/status/production.rs");
    let start = src
        .find("pub mod mrp {")
        .expect("models/status/production.rs 缺少 `pub mod mrp`");
    let tail = &src[start..];
    // 模块体到本文件里第一个顶格 `}` 为止（mrp 模块内不再嵌套 mod）
    let end = tail[1..]
        .find("\n}")
        .unwrap_or_else(|| panic!("`pub mod mrp` 大括号未闭合"));
    let body = &tail[..end + 2];
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.find("pub const ") {
        let after = &rest[pos + "pub const ".len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let vpos = after
            .find('=')
            .unwrap_or_else(|| panic!("常量 {name} 缺少 = 赋值"));
        let vs = &after[vpos + 1..];
        let q1 = vs
            .find('"')
            .unwrap_or_else(|| panic!("常量 {name} 的值不是字符串字面量"));
        let qs = &vs[q1 + 1..];
        let q2 = qs
            .find('"')
            .unwrap_or_else(|| panic!("常量 {name} 值引号未闭合"));
        out.push((name.clone(), qs[..q2].to_string()));
        rest = &qs[q2 + 1..];
    }
    out
}

/// 1. 常量名 == 值、值全大写纯 ASCII（本仓库口径：DB 存大写英文闭合词表，中文只出现在 i18n）。
#[test]
fn mrp_status_constants_are_uppercase_ascii_and_self_named() {
    let items = backend_mrp_status();
    assert!(
        !items.is_empty(),
        "未解析到 `pub mod mrp` 的任何常量，解析器已失效"
    );
    for (name, value) in &items {
        assert_eq!(name, value, "MRP 状态常量名与值不一致：{name} = {value}");
        assert!(
            value.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "MRP 状态值必须为大写纯 ASCII：{name} = {value}"
        );
    }
}

/// 2. 前端 `MrpStatus` 联合类型 == 后端常量取值集合（双向比对，防前端臆造或漏配）。
#[test]
fn frontend_mrp_status_union_matches_backend() {
    let backend: Vec<String> = backend_mrp_status().into_iter().map(|(_, v)| v).collect();
    let fe = read("../frontend/src/api/mrp.ts");
    let anchor = fe
        .find("export type MrpStatus =")
        .expect("frontend/src/api/mrp.ts 缺少 `export type MrpStatus`");
    let tail = &fe[anchor..];
    let stmt_end = tail.find(';').expect("MrpStatus 联合类型未以 ; 结束");
    let stmt = &tail[..stmt_end];
    let mut fe_values = Vec::new();
    let mut rest = stmt;
    while let Some(q1) = rest.find('\'') {
        let after = &rest[q1 + 1..];
        let q2 = after
            .find('\'')
            .unwrap_or_else(|| panic!("MrpStatus 取值引号未闭合：{stmt}"));
        fe_values.push(after[..q2].to_string());
        rest = &after[q2 + 1..];
    }
    let mut a = backend.clone();
    let mut b = fe_values.clone();
    a.sort();
    b.sort();
    assert_eq!(
        a, b,
        "MRP 状态词表前后端不同源：后端={:?} 前端={:?}",
        backend, fe_values
    );
}

/// 3. MRP 写入/比较点不得出现裸状态字面量（`Set("PLANNED")` / `== "CANCELLED"` 这类）。
/// 只允许 `mrp_status::XXX`（或 `status::mrp::XXX`）常量引用。
#[test]
fn mrp_status_writers_use_constants_not_literals() {
    let files = [
        "src/services/mrp_engine_ops/calculation.rs",
        "src/services/mrp_engine_ops/order.rs",
        "src/services/mrp_engine_ops/query.rs",
        "src/handlers/mrp_handler.rs",
    ];
    let known: Vec<String> = backend_mrp_status().into_iter().map(|(_, v)| v).collect();
    for f in files {
        let src = read(f);
        for (line_no, line) in src.lines().enumerate() {
            for status in &known {
                let quoted = format!("\"{status}\"");
                let hits_literal = line.contains(&quoted)
                    // 允许：常量模块自身的定义处（本测试第 1 项已覆盖），以及注释行
                    && !line.trim_start().starts_with("//")
                    && !line.contains("pub const");
                assert!(
                    !hits_literal,
                    "{}:{} 直接写状态字面量 {}，应改为引用 models::status::mrp 常量：{}",
                    f,
                    line_no + 1,
                    status,
                    line.trim()
                );
            }
        }
    }
}

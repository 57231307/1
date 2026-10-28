//! 反向静态锁：展示端取号端点全废后，generate-no 展示路由不得再现。
//!
//! 依据：20261028 裁定，五枚对外可见的单据号展示端 GET generate-no 端点全废、
//! 不设兼容期，单号一律从建单响应回读；逐单展示腿与回读腿在
//! contract_wave2_document_no_parity_test.rs 三层锁，本文件是全局路由面兜底：
//! 直接扫描路由注册面，任何豁免表之外文件出现 generate-no 字面即判红。
//!
//! 扫描面：backend/src/routes/ 递归全部 .rs（逐文件统计字面出现次数），
//! 不设文件白名单、不随当前路由增减而缩小；豁免只能显式列文件与精确次数。
//! 豁免表：B1 后批两枚死端点（零视图消费者，删除未开工），purchase.rs 与
//! sales.rs 现各含 1 处（orders/generate-no），次数偏离 1 即红，防止借豁免
//! 文件夹带新展示端；B1 删端点时须同步撤条目，豁免表空心失效由“两枚必须
//! 在扫描面内在场”断言兜住。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// B1 后批待删死端点豁免表：文件名与允许的 generate-no 字面出现次数
const PENDING_REMOVAL: &[(&str, usize)] = &[("purchase.rs", 1), ("sales.rs", 1)];

/// 递归收集目录（含子目录）下全部 .rs 文件
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|e| panic!("读取 {} 目录项失败: {e}", dir.display())).path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e.to_string_lossy() == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn generate_no_display_routes_must_not_reappear() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/routes");
    assert!(root.is_dir(), "路由扫描面根目录必须存在: {}", root.display());
    let mut files: Vec<PathBuf> = Vec::new();
    collect_rs(&root, &mut files);
    assert!(
        files.len() >= 40,
        "routes 扫描面异常收缩（仅 {} 个 .rs），判绿不可信",
        files.len()
    );

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in &files {
        let name = path
            .file_name()
            .unwrap_or_else(|| panic!("路径无文件名: {}", path.display()))
            .to_string_lossy()
            .to_string();
        let src = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()));
        // 同名文件（不同子目录）次数累加，豁免按文件名精确匹配，不给重名钻空子
        *counts.entry(name).or_insert(0) += src.matches("generate-no").count();
    }

    for (name, n) in &counts {
        if let Some((_, expected)) = PENDING_REMOVAL.iter().find(|(f, _)| *f == name.as_str()) {
            assert_eq!(
                n, expected,
                "豁免文件 {name} 的 generate-no 出现次数应为 {expected}，实际 {n}（豁免不等于放开）"
            );
        } else {
            assert_eq!(
                n, &0,
                "展示端 generate-no 路由再现：{name} 内出现 {n} 次（裁定口径：废除即永久，回退走 revert）"
            );
        }
    }

    // 豁免表两枚必须真在扫描面内在场：文件被改名或删除时豁免失效而非静默放行
    for (file, _) in PENDING_REMOVAL {
        assert!(
            counts.contains_key(&file.to_string()),
            "豁免文件 {file} 不在 routes 扫描面内，豁免表已失真"
        );
    }
}

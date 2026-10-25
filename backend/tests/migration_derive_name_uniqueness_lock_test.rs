//! 迁移名唯一性静态锁（不依赖活库，CI 常规 job 可跑）
//!
//! 缺陷：`DeriveMigrationName` 取的是**源文件名（不含扩展名）**，不是模块路径，也不是结构体名。
//! 域迁移写成 `domain/<域名>/mod.rs` 时，文件名恒为 `mod`，落到 `seaql_migrations.version`
//! 的字面值就是 "mod"；同一链上只要有两枚这样的迁移，第二枚 insert 就撞主键
//! `23505 duplicate key value violates unique constraint "seaql_migrations_pkey"`，
//! Key (version)=(mod) already exists，迁移链当场断在中间，下游集成测试与全部 e2e 分片连带判红。
//!
//! 调用方：CI 的 Rust 测试 job 与本仓本地门。入参：`migration/src` 目录（相对本 crate 清单目录）。
//! 传给谁：无下游，纯断言。存什么：不写库、不写文件。存哪里：不落地。
//!
//! 口径：只锁「同名」这一件事，不要求改名已经应用过的迁移——已应用迁移的 version 值
//! 已在存量库的 `seaql_migrations` 里在册，改它会让同一枚迁移在新库以另一名字重跑，
//! 属禁改已应用迁移的铁律范围。因此本锁判重不判文件名形态。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 递归收集目录下所有 .rs 文件路径，供逐个读文本判定是否带迁移名 derive。
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("读取迁移目录 {:?} 失败: {}", dir, e));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("读取 {:?} 目录项失败: {}", dir, e))
            .path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// 取 `derive(DeriveMigrationName)` 会算出的迁移名，即源文件名去掉扩展名。
fn derived_name(file: &Path) -> String {
    file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| panic!("取不到文件名主干: {:?}", file))
        .to_string()
}

/// 扫描 `migration/src`，返回「derive 出迁移名的文件」与其名字，按名字归组。
///
/// 返回项少于阈值即说明扫描本身没走到迁移源码（路径写错或目录改名），
/// 此时必须报错而不是静默通过，否则这条锁会退化成恒绿的假门。
fn scan_derived_migration_names() -> BTreeMap<String, Vec<PathBuf>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migration/src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let mut by_name: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let mut hit = 0usize;
    for file in files {
        let text =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", file, e));
        if text.contains("derive(DeriveMigrationName)") {
            hit += 1;
            by_name.entry(derived_name(&file)).or_default().push(file);
        }
    }
    assert!(
        hit >= 50,
        "带迁移名 derive 的源文件只有 {} 个，说明扫描没走到迁移源码（本仓迁移远超此数），\
         这条锁已失去检测力: {:?}",
        hit,
        root
    );
    by_name
}

/// 任何两枚迁移都不得算出同一个 version 值。
#[test]
fn migration_derived_names_are_unique_across_the_chain() {
    let by_name = scan_derived_migration_names();
    let collisions: Vec<(&String, &Vec<PathBuf>)> = by_name
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();
    assert!(
        collisions.is_empty(),
        "以下迁移算出的 version 同名，会在 seaql_migrations_pkey 上撞 23505 把迁移链断在中间：{}",
        collisions
            .iter()
            .map(|(name, files)| format!(
                "\n  名字 {} 由这些文件给出：{}",
                name,
                files
                    .iter()
                    .map(|f| f.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("，")
            ))
            .collect::<Vec<_>>()
            .join("")
    );
}

/// 判定器自证：同名塌缩的形态确实能被上一条测出来。
/// 用两条真实形态的文件名走取名字这段逻辑，取到同一个值才算这条锁有效。
#[test]
fn mod_rs_files_collapse_to_the_same_name() {
    let left = derived_name(Path::new("domain/ap_input_tax_subject/mod.rs"));
    let right = derived_name(Path::new("domain/customers_nullable_columns_norm/mod.rs"));
    assert_eq!(
        left, right,
        "两枚目录式 mod.rs 迁移应算出同一个 version 值，否则本锁的检测力前提不成立"
    );
    assert_eq!(left, "mod");
}

//! purchase_orders 双状态列防漂移回归（确定性源码文本扫描，不依赖数据库）。
//!
//! 收口口径：`order_status` 是唯一真相（全大写词表
//! `models/status/purchase_inventory.rs::purchase_order`），m0001 遗留的 `status`
//! 列降级为不映射、不作过滤的兼容列。SeaORM 实体 `models/purchase_order.rs` 只
//! 映射 `order_status`，因此 ORM 侧永远无法读写/过滤遗留列；唯一可能的漂移通道是
//! 有人把遗留列重新映射进实体、或在采购单代码里用 `Column::Status` 过滤。
//! 本测试锁死这两条通道，并核对迁移层已写入的回填与废弃注释。
//!
//! 取舍：这里用源码文本扫描而非对活库的 SQL 断言——CI 常规集成测试连的是空库
//! （`setup_test_db` 无表），SQL 层断言需要已迁移库且方言绑定 PG，脆弱且难跑；
//! 而「实体是否重新映射遗留列 / 是否出现 Column::Status 过滤」是编译期可见的静态事实，
//! 文本扫描确定、零依赖、每次 CI 必跑，足以守住漂移。回填后的两列一致性与 CHECK
//! 全集另由 color_card 侧的取值集断言与迁移 SQL 本身保证。

use std::fs;
use std::path::{Path, PathBuf};

/// 递归收集目录下全部 .rs 文件路径。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", p, e))
}

/// 遗留 `status` 列必须不映射进采购单实体，`order_status` 才是映射列。
#[test]
fn po_entity_maps_only_order_status() {
    let model = read(&src_root().join("models/purchase_order.rs"));
    assert!(
        model.contains("pub order_status: String"),
        "采购单实体必须映射 order_status 列"
    );
    assert!(
        !model.contains("pub status:"),
        "采购单实体不得重新映射遗留 status 列（否则 ORM 会读写失真列，两列将漂移）"
    );
}

/// 采购单实体与 handler 不得出现任何遗留列的裸 `Column::Status` 过滤。
/// （service 层跨实体——如 price.rs 过滤 supplier::Column::Status 属另一张表，由
///  下条完全限定名断言专门拦截 `purchase_order::Column::Status`，故此处只锁死
///  以采购单为唯一实体的实体文件与 handler。）
#[test]
fn po_owned_files_have_no_bare_status_filter() {
    for rel in [
        "models/purchase_order.rs",
        "handlers/purchase_order_handler.rs",
    ] {
        let f = src_root().join(rel);
        if !f.exists() {
            continue;
        }
        let text = read(&f);
        assert!(
            !text.contains("Column::Status"),
            "{:?} 出现 Column::Status——采购单只允许过滤 order_status（Column::OrderStatus），\
             不得按遗留 status 列过滤/读写",
            f
        );
    }
}

/// 全仓源码不得出现对遗留列的完全限定过滤 `purchase_order::Column::Status`。
#[test]
fn fully_qualified_legacy_status_filter_absent() {
    let mut files = Vec::new();
    collect_rs(&src_root(), &mut files);
    for f in &files {
        let text = read(f);
        assert!(
            !text.contains("purchase_order::Column::Status"),
            "{:?} 引用了 purchase_order::Column::Status（遗留失真列），禁止",
            f
        );
    }
}

/// 列表过滤必须走 order_status（真实列），且响应键重命名保持。
#[test]
fn po_list_filter_uses_order_status() {
    let crud = read(&src_root().join("services/po/order_ops/crud.rs"));
    assert!(
        crud.contains("Column::OrderStatus.eq("),
        "list_orders 必须按 Column::OrderStatus 过滤"
    );
    let order = read(&src_root().join("services/po/order.rs"));
    assert!(
        order.contains("#[serde(rename = \"status\")]")
            && order.contains("pub order_status: String"),
        "响应 DTO 必须把 order_status 重命名为 status 对外暴露（前端按 status 键消费真实列）"
    );
}

/// 迁移层收口必须存在：以 order_status 回填遗留 status、并标注 DEPRECATED。
#[test]
fn migration_backfills_and_deprecates_legacy_status() {
    let v15 =
        read(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../migration/src/domain/v15/mod.rs"));
    assert!(
        v15.contains(r#"SET "status" = "order_status""#),
        "v15 迁移必须含确定性回填：status := order_status"
    );
    assert!(
        v15.contains("[DEPRECATED"),
        "v15 迁移必须用列注释把遗留 status 列标记为 DEPRECATED"
    );
    assert!(
        v15.contains(r#"ALTER TABLE "purchase_orders" ALTER COLUMN "order_status" SET NOT NULL"#),
        "真相列 order_status 应收敛为 NOT NULL（先回填 NULL→DRAFT）"
    );
}

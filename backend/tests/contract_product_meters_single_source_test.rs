//! 产品米数单源与每卷米数列下线契约锁
//!
//! 裁定口径（产品档案规格与每匹米数收口工单）：一卷即一匹是唯一现网口径，
//! products.meters_per_roll 是从未携带独立信息的照抄假字段。代码面先整体摘除该键
//! （实体、handler 创建与更新 DTO、服务参数对象、批量链路、换算器卷制腿、报价转订单卷制腿），
//! 再由 m0109 探测式迁移下线列形态。每匹米数保留为换算默认值列，但退出产品表单采集面。
//!
//! 锁定的契约面分三层。
//! 其一为换算语义（纯函数，不连库）。卷制腿与匹制腿共用每匹米数一个真源：
//! 卷数乘每匹米数得米数，非正入参一律 Err 拒绝，不兜底不猜测。
//! 其二为源码普查（不连库）。每卷米数键与其同源补齐函数在全部写入链路与实体中零残留，
//! 出现即红；每匹米数保持创建单层两态与更新双层三态；批量创建显式落列、批量更新三态落列
//! 不得塌回两态；报价转订单的卷制腿必须读每匹米数。硬边界同样钉死：单位词表匹与卷仍是
//! 并列 token（只废换算列，不废计量单位），加列的历史迁移 m0059 保持原样不回改
//! （已应用迁移永不修改，下线由新迁移承担）。
//! 其三为落库回读（真 PostgreSQL，仅 CI 执行）。只带每匹米数建品时该列落请求值且产品出参
//! JSON 不再含每卷米数键；显式 null 清空真的落 NULL；请求不带键时历史值保持不动，
//! 证明列下线不影响每匹米数的三态语义。
//!
//! 检测力自证。若卷制腿改回读每卷米数，源码普查与纯函数用例同时红；若 DTO 残留该键，
//! 实体普查红；若 m0109 被改成静默 DROP（无探测无 RAISE EXCEPTION），迁移形态普查红；
//! 若迁移被塞进已应用域（m0059 被回改），历史迁移普查红；若清空塌回保持原值两态，
//! 落库清空用例读到旧值立即红。每种退化各有对应用例，不互相重叠。
//!
//! 夹具形态：真 PostgreSQL setup_test_db（已迁移库加 TRUNCATE 业务表），缺
//! TEST_DATABASE_URL 会在夹具内直接 panic，不做静默降级，因此本文件的落库用例只在
//! CI 侧执行，本地不跑。源码普查只比内容不比行尾，本仓工作区可能是 CRLF 而索引区是 LF。
//!
//! 引用既有实现一律以函数名为准，被他处挪位时按函数名重取，不记行位。

mod test_common;

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::product;
use bingxi_backend::models::status::master_data;
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::product_service::{
    CreateProductArgs, ProductService, UpdateProductArgs,
};
use bingxi_backend::utils::dual_unit_converter::DualUnitConverter;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait};
use test_common::setup_test_db;

fn dec(raw: &str) -> Decimal {
    Decimal::from_str(raw).expect("测试内十进制字面量应可解析")
}

/// 读仓库内后端源码并把行尾统一成 LF：工作区可能是 CRLF 而索引区是 LF，判据只看内容
fn read_backend_source(rel: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = Path::new(manifest_dir).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("无法读取 {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

// ===== 第一层：换算语义（纯函数）=====

#[test]
fn rolls_leg_uses_meters_per_piece_single_source() {
    // 一卷即一匹：卷数乘每匹米数，与匹制腿同一真源
    let meters =
        DualUnitConverter::rolls_to_meters(dec("3"), dec("60")).expect("正入参卷制换算应成功");
    assert_eq!(meters, dec("180"), "3 卷每匹 60 米必须得 180 米");
    let same_as_pieces =
        DualUnitConverter::pieces_to_meters(dec("3"), dec("60")).expect("同值匹制换算应成功");
    assert_eq!(
        meters, same_as_pieces,
        "卷制腿与匹制腿必须同值（共用每匹米数即两腿等价，出现不等说明卷制腿另有口径）"
    );
}

#[test]
fn rolls_leg_rejects_non_positive_piece_metadata() {
    let err = DualUnitConverter::rolls_to_meters(dec("2"), Decimal::ZERO)
        .expect_err("每匹米数为 0 必须拒绝，不得换算成 0 米冒充可算");
    assert!(
        err.contains("每匹米数"),
        "拒绝文案必须点名缺的是每匹米数，实际为：{err}"
    );
    let err_neg =
        DualUnitConverter::rolls_to_meters(dec("2"), dec("-5")).expect_err("负每匹米数必须拒绝");
    assert!(!err_neg.is_empty());
    let err_count =
        DualUnitConverter::rolls_to_meters(dec("-1"), dec("60")).expect_err("负卷数必须拒绝");
    assert!(
        err_count.contains("卷数"),
        "拒绝文案必须点名卷数，实际为：{err_count}"
    );
}

// ===== 第二层：源码普查与硬边界 =====

#[test]
fn roll_key_and_pairing_helper_fully_retired_from_write_paths() {
    for rel in [
        "src/models/product.rs",
        "src/handlers/product_handler.rs",
        "src/services/product_service.rs",
        "src/services/product_ops/crud.rs",
        "src/services/product_ops/import_export.rs",
        "src/services/batch_service.rs",
    ] {
        let src = read_backend_source(rel);
        assert!(
            !src.contains("meters_per_roll"),
            "{rel} 里不应再有每卷米数键的任何声明或读写（列由 m0109 探测式下线，代码必须先净）"
        );
        assert!(
            !src.contains("pair_piece_roll_meters"),
            "{rel} 里同源补齐函数必须整体退场，不得半退留壳"
        );
    }
    let converter = read_backend_source("src/utils/dual_unit_converter.rs");
    let rolls_fn = &converter[converter
        .find("pub fn rolls_to_meters")
        .expect("卷制腿必须存在")..];
    let rolls_body = &rolls_fn[..rolls_fn.find("\n    }").expect("卷制腿函数体应可截出")];
    assert!(
        rolls_body.contains("meters_per_piece"),
        "换算器卷制腿的入参真源必须是每匹米数"
    );
    assert!(
        !rolls_body.contains("meters_per_roll"),
        "换算器卷制腿不得再消费每卷米数"
    );
}

#[test]
fn quotation_rolls_arm_reads_meters_per_piece() {
    let src = read_backend_source("src/services/quotation_convert_service.rs");
    assert!(
        !src.contains("meters_per_roll"),
        "报价转订单全链路不应再出现每卷米数取数（一卷即一匹，卷制腿共用每匹米数）"
    );
    let rolls_arm = src[src
        .find("UnitStrategy::Rolls =>")
        .expect("卷制分支必须存在")
        ..src
            .find("UnitStrategy::Kilograms =>")
            .expect("公斤分支必须存在")]
        .to_string();
    assert!(
        rolls_arm.contains("product.meters_per_piece"),
        "报价转订单卷制腿必须读每匹米数列"
    );
    assert!(
        rolls_arm.contains("rolls_to_meters"),
        "卷制腿必须仍走 DualUnitConverter 单源，不得本地重写乘式"
    );
}

#[test]
fn piece_meters_keeps_create_twostate_and_update_tristate() {
    let src = read_backend_source("src/handlers/product_handler.rs");
    assert!(
        src.contains("pub meters_per_piece: Option<Decimal>"),
        "创建 DTO 的每匹米数保持单层两态（带值或缺值即落值或 NULL）"
    );
    assert!(
        src.contains("pub meters_per_piece: Option<Option<Decimal>>"),
        "更新 DTO 的每匹米数保持双层三态，塌回单层即清空通道复发"
    );

    let batch = read_backend_source("src/services/batch_service.rs");
    assert!(
        batch.contains("meters_per_piece: Set(req.meters_per_piece)"),
        "批量创建必须显式落每匹米数列，不能靠 Default 兜 NULL 静默丢值"
    );
    assert!(
        batch.contains("product.meters_per_piece = Set(piece)"),
        "批量更新必须三态落列（外层 Some 原样 Set），不得退回两态旧写法"
    );
    assert!(
        !batch.contains("product.meters_per_piece = Set(Some("),
        "批量更新不得把内层值再包一层 Some 塌回两态"
    );
}

#[test]
fn retirement_migration_registered_and_probe_shaped() {
    let migration = read_backend_source(
        "migration/src/domain/product_roll_column_drop/m0109_drop_product_meters_per_roll.rs",
    );
    assert!(
        migration.contains("information_schema") && migration.contains("meters_per_roll"),
        "下线迁移必须先探测列存在性（幂等，重复执行安全）"
    );
    assert!(
        migration.contains("IS DISTINCT FROM"),
        "下线迁移必须先探测每卷米数非空且与每匹米数不等的脏行，脏行存在即拒绝下线"
    );
    assert!(
        migration.contains("RAISE EXCEPTION"),
        "探测到脏行必须点名中止（失败可见），不得静默 DROP 有信息量的数据"
    );
    assert!(migration.contains("DROP COLUMN"), "探测通过才执行下线");
    assert!(
        !migration.contains("UPDATE products") && !migration.contains("UPDATE \"products\""),
        "下线迁移绝不 UPDATE 业务数据，脏值交人工核对后重跑"
    );
    assert!(
        !migration.contains("DELETE FROM"),
        "下线迁移绝不 DELETE 业务数据"
    );

    let mod_src = read_backend_source("migration/src/domain/mod.rs");
    assert!(
        mod_src.contains("pub mod product_roll_column_drop;"),
        "下线域必须登记为顶层域（并入已应用域会在复用库被整体跳过，DROP 永不执行）"
    );
    let lib_src = read_backend_source("migration/src/lib.rs");
    let mod_pos = lib_src.find("domain::product_vocab_check::Migration");
    let drop_pos = lib_src.find("domain::product_roll_column_drop::Migration");
    assert!(
        mod_pos.is_some() && drop_pos.is_some(),
        "下线域必须注册进迁移链"
    );
    assert!(
        drop_pos.expect("下线域注册位") > mod_pos.expect("收口域注册位"),
        "下线域必须排在取值域收口域之后（先加列收口，后废弃假字段）"
    );

    // 已应用迁移永不修改：加列的历史迁移保持原样，不回改
    let m0059 = read_backend_source(
        "migration/src/domain/production/m0059_add_product_piece_roll_conversion.rs",
    );
    assert!(
        m0059.contains("ADD COLUMN IF NOT EXISTS \"meters_per_roll\""),
        "历史迁移 m0059 不得被回改（已应用迁移永不修改，下线由 m0109 承担）"
    );
}

#[test]
fn unit_tokens_bolt_and_roll_are_not_removed() {
    let unit_src = read_backend_source("../frontend/src/constants/quotation-unit.ts");
    assert!(
        unit_src.contains("bolt: '匹'") && unit_src.contains("roll: '卷'"),
        "硬边界禁止改单位词表：匹与卷必须仍是并列单位 token（废的是换算列，不是计量单位）"
    );
}

// ===== 第三层：落库回读（真 PostgreSQL，仅 CI 执行）=====

fn product_service(db: &DatabaseConnection) -> ProductService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    ProductService::new(Arc::new(db.clone()), search_client)
}

/// 建非面料类产品避开必录门，本文件断言面是米数单源，不影响判据本身
async fn create_product_with_piece(
    db: &DatabaseConnection,
    piece: Option<Decimal>,
) -> product::Model {
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    product_service(db)
        .create_product(CreateProductArgs {
            name: format!("米数单源测试辅料 {stamp}"),
            code: format!("PMS-{stamp}"),
            barcode: None,
            category_id: None,
            specification: None,
            unit: "米".to_string(),
            standard_price: None,
            cost_price: None,
            description: None,
            status: master_data::ACTIVE.to_string(),
            product_type: Some("辅料".to_string()),
            fabric_composition: None,
            yarn_count: None,
            density: None,
            width: None,
            gram_weight: None,
            structure: None,
            finish: None,
            min_order_quantity: None,
            lead_time: None,
            execution_standard: None,
            factory_name: None,
            factory_address: None,
            product_grade: None,
            width_needle_included_cm: None,
            weight_tolerance: None,
            spandex_percent: None,
            usage: None,
            safety_category: None,
            meters_per_piece: piece,
        })
        .await
        .expect("建产品应成功")
}

/// 从库里直接回读每匹米数，绕开任何缓存，保证断言看到的是真实落库形态
async fn reload_piece(db: &DatabaseConnection, id: i32) -> Option<Decimal> {
    let row = product::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("回读 products 行应成功")
        .expect("回读 products 应命中该行");
    row.meters_per_piece.map(|v| v.normalize())
}

async fn update_product_piece(db: &DatabaseConnection, id: i32, piece: Option<Option<Decimal>>) {
    product_service(db)
        .update_product(UpdateProductArgs {
            id,
            name: None,
            barcode: None,
            specification: None,
            unit: None,
            standard_price: None,
            cost_price: None,
            description: None,
            status: None,
            product_type: None,
            fabric_composition: None,
            yarn_count: None,
            density: None,
            width: None,
            gram_weight: None,
            structure: None,
            finish: None,
            min_order_quantity: None,
            lead_time: None,
            execution_standard: None,
            factory_name: None,
            factory_address: None,
            product_grade: None,
            width_needle_included_cm: None,
            weight_tolerance: None,
            spandex_percent: None,
            usage: None,
            safety_category: None,
            meters_per_piece: piece,
            user_id: 1,
        })
        .await
        .expect("更新产品每匹米数应成功");
}

#[tokio::test]
async fn create_with_piece_lands_value_and_response_has_no_roll_key() {
    let db = setup_test_db().await;
    let created = create_product_with_piece(&db, Some(dec("50"))).await;

    assert_eq!(
        reload_piece(&db, created.id).await,
        Some(dec("50")),
        "新建时每匹米数应落请求值"
    );

    // 出参契约锁：产品实体序列化后不再含每卷米数键（活体面，序列化形态与 HTTP 出参同源）
    let json = serde_json::to_value(&created).expect("产品实体应可序列化");
    assert!(
        json.get("meters_per_roll").is_none(),
        "产品出参 JSON 不应再含每卷米数键，实际序列化：{json}"
    );
    let piece_val = json
        .get("meters_per_piece")
        .expect("每匹米数键应随出参下发");
    // Decimal 序列化形态按仓内配置可能是字符串或数字，取值判据按两形态归一后比较，不锁排版
    let piece_text = piece_val
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| piece_val.to_string());
    assert_eq!(
        Decimal::from_str(&piece_text).unwrap_or(Decimal::NEGATIVE_ONE),
        dec("50"),
        "每匹米数应随出参下发且值为 50，实际形态：{piece_val}"
    );
}

#[tokio::test]
async fn explicit_null_clears_piece_to_null() {
    let db = setup_test_db().await;
    let created = create_product_with_piece(&db, Some(dec("50"))).await;

    update_product_piece(&db, created.id, Some(None)).await;
    assert_eq!(
        reload_piece(&db, created.id).await,
        None,
        "显式 null 必须清空为 NULL，仍读到旧值即清空通道塌回了保持原值两态"
    );
}

#[tokio::test]
async fn absent_key_keeps_historical_piece() {
    let db = setup_test_db().await;
    let created = create_product_with_piece(&db, Some(dec("50"))).await;

    // 只改规格，请求不带米数键：每匹米数必须原样，证明表单摘除不影响存量值的保持语义
    product_service(&db)
        .update_product(UpdateProductArgs {
            id: created.id,
            name: None,
            barcode: None,
            specification: Some("仅改规格".to_string()),
            unit: None,
            standard_price: None,
            cost_price: None,
            description: None,
            status: None,
            product_type: None,
            fabric_composition: None,
            yarn_count: None,
            density: None,
            width: None,
            gram_weight: None,
            structure: None,
            finish: None,
            min_order_quantity: None,
            lead_time: None,
            execution_standard: None,
            factory_name: None,
            factory_address: None,
            product_grade: None,
            width_needle_included_cm: None,
            weight_tolerance: None,
            spandex_percent: None,
            usage: None,
            safety_category: None,
            meters_per_piece: None,
            user_id: 1,
        })
        .await
        .expect("只改规格的更新应成功");

    assert_eq!(
        reload_piece(&db, created.id).await,
        Some(dec("50")),
        "请求不带米数键时每匹米数必须保持历史值"
    );
}

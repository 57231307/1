//! 产品每匹米数与每卷米数同源双写契约锁
//!
//! 裁定口径（产品管理新建页只留每匹米数一个输入入口）：一卷即一匹，两列米数同源。
//! 前端把每卷米数改成由每匹米数带出的只读项，后端在请求只带其中一个值时按同值双写两列。
//! 硬边界同样锁死：不删列、不改库、不动历史数据，两列语义与单位词表一律保留。
//!
//! 锁定的契约面分三层：
//! 1. 补齐语义（纯函数，不连库）。只带每匹米数时把每卷米数补成同值，只带每卷米数时把
//!    每匹米数补成同值；两值都带时各自原样返回，含不等也不得静默取其一或互相覆写；
//!    两值都不带时不做补齐，不引入默认值掩盖缺键。
//! 2. 调用点（源码普查，不连库）。创建与更新两条写入链路都必须真的经补齐函数取值，
//!    否则补齐函数单独全绿而调用点被摘掉就是假绿；同时钉住两列仍存在于实体与建表迁移，
//!    防止顺手删列。
//! 3. 落库回读（真 PostgreSQL）。补齐后的两值确实落在 products 两列上；历史不等行在
//!    请求未带任何米数时两列都保持原值，证明编辑别处不会顺手改写米数元数据。
//!
//! 补齐为什么放 handler 层而不是 crud 层：crud 层对这两列用条件 Set，None 的语义是本次
//! 不更新该列。把补齐塞进 crud 会把不更新与改成同值两种意图混成一谈，历史行会被误覆写。
//!
//! 检测力自证：若实现改成两值不等时取其一，只带一个值的落库用例仍绿，但不等透传用例与
//! 不等行保持原值用例会同时变红；若补齐被摘掉，只带一个值的两个落库用例读到 NULL 立即
//! 变红；若调用点绕过补齐函数，源码普查用例立即变红。三种退化各有对应用例，不得简化。
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

use bingxi_backend::handlers::product_handler::pair_piece_roll_meters;
use bingxi_backend::models::product;
use bingxi_backend::models::status::master_data;
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::product_service::{
    CreateProductArgs, ProductService, UpdateProductArgs,
};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait};
use test_common::setup_test_db;

fn dec(raw: &str) -> Decimal {
    Decimal::from_str(raw).expect("测试内十进制字面量应可解析")
}

fn some_dec(raw: &str) -> Option<Decimal> {
    Some(dec(raw))
}

/// 读仓库内后端源码并把行尾统一成 LF：工作区可能是 CRLF 而索引区是 LF，判据只看内容
fn read_backend_source(rel: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = Path::new(manifest_dir).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("无法读取 {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

// ===== 第一层：补齐语义（纯函数）=====

#[test]
fn only_piece_value_fills_roll_with_same_value() {
    let (piece, roll) = pair_piece_roll_meters(some_dec("50"), None);
    assert_eq!(piece, some_dec("50"), "每匹米数应原样保留");
    assert_eq!(
        roll,
        some_dec("50"),
        "请求只带每匹米数时，每卷米数必须补成同值，否则按卷换算仍缺元数据"
    );
}

#[test]
fn only_roll_value_fills_piece_with_same_value() {
    let (piece, roll) = pair_piece_roll_meters(None, some_dec("30"));
    assert_eq!(
        piece,
        some_dec("30"),
        "请求只带每卷米数时，每匹米数必须补成同值（按卷入口是历史入口，不能反过来丢数据）"
    );
    assert_eq!(roll, some_dec("30"), "每卷米数应原样保留");
}

#[test]
fn unequal_pair_passes_through_without_picking_one() {
    let (piece, roll) = pair_piece_roll_meters(some_dec("50"), some_dec("30"));
    assert_eq!(
        piece,
        some_dec("50"),
        "两值都带且不等时每匹米数必须原样，禁止静默取其一"
    );
    assert_eq!(
        roll,
        some_dec("30"),
        "两值都带且不等时每卷米数必须原样，禁止用每匹米数覆写调用方显式取值"
    );
}

#[test]
fn equal_pair_passes_through_unchanged() {
    let (piece, roll) = pair_piece_roll_meters(some_dec("40"), some_dec("40"));
    assert_eq!(piece, some_dec("40"), "两值相等时不应被改写");
    assert_eq!(roll, some_dec("40"), "两值相等时不应被改写");
}

#[test]
fn absent_pair_stays_absent() {
    let (piece, roll) = pair_piece_roll_meters(None, None);
    assert_eq!(piece, None, "两键都不带时每匹米数不得补出值来");
    assert_eq!(
        roll, None,
        "两键都不带时每卷米数不得补出值来（补齐只由单键缺席触发，不能用默认值掩盖缺键）"
    );
}

// ===== 第二层：调用点与硬边界（源码普查）=====

#[test]
fn create_and_update_both_route_meters_through_pairing() {
    let src = read_backend_source("src/handlers/product_handler.rs");

    let call_sites = src
        .match_indices("pair_piece_roll_meters(req.meters_per_piece, req.meters_per_roll)")
        .count();
    assert_eq!(
        call_sites, 2,
        "创建与更新应各有一处补齐调用，实际命中 {call_sites} 处；若为 0 则补齐函数与调用点已脱钩"
    );

    for stale in [
        "meters_per_piece: req.meters_per_piece",
        "meters_per_roll: req.meters_per_roll",
    ] {
        assert!(
            !src.contains(stale),
            "两列取值不得绕过补齐直接透传请求字段，仍存在 {stale}"
        );
    }
}

#[test]
fn both_columns_and_unit_tokens_are_not_removed() {
    let model_src = read_backend_source("src/models/product.rs");
    for column in ["meters_per_piece", "meters_per_roll"] {
        assert!(
            model_src.contains(&format!("pub {column}: Option<Decimal>")),
            "硬边界禁止删列：实体里 {column} 必须仍在"
        );
    }

    let migration_src = read_backend_source(
        "migration/src/domain/production/m0059_add_product_piece_roll_conversion.rs",
    );
    for column in ["meters_per_piece", "meters_per_roll"] {
        assert!(
            migration_src.contains(&format!("ADD COLUMN IF NOT EXISTS \"{column}\"")),
            "硬边界禁止删列：建表迁移里 {column} 的 ADD COLUMN 必须仍在"
        );
    }

    let unit_src = read_backend_source("../frontend/src/constants/quotation-unit.ts");
    assert!(
        unit_src.contains("bolt: '匹'") && unit_src.contains("roll: '卷'"),
        "硬边界禁止改单位词表：匹与卷必须仍是并列单位 token"
    );
}

// ===== 第三层：落库回读（真 PostgreSQL，仅 CI 执行）=====

fn product_service(db: &DatabaseConnection) -> ProductService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    ProductService::new(Arc::new(db.clone()), search_client)
}

/// 走与 handler 相同的补齐再建产品，返回落库行
async fn create_product_with(
    db: &DatabaseConnection,
    piece: Option<Decimal>,
    roll: Option<Decimal>,
) -> product::Model {
    let (meters_per_piece, meters_per_roll) = pair_piece_roll_meters(piece, roll);
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();

    product_service(db)
        .create_product(CreateProductArgs {
            name: format!("米数同源测试坯布 {stamp}"),
            code: format!("PYD-{stamp}"),
            barcode: None,
            category_id: None,
            specification: None,
            unit: "米".to_string(),
            standard_price: None,
            cost_price: None,
            description: None,
            status: master_data::ACTIVE.to_string(),
            product_type: "坯布".to_string(),
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
            meters_per_piece,
            meters_per_roll,
        })
        .await
        .expect("建产品应成功")
}

/// 从库里直接回读两列，绕开任何缓存，保证断言看到的是真实落库形态
async fn reload_meters(db: &DatabaseConnection, id: i32) -> (Option<Decimal>, Option<Decimal>) {
    let row = product::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("回读 products 行应成功")
        .expect("回读 products 应命中该行");
    (
        row.meters_per_piece.map(|v| v.normalize()),
        row.meters_per_roll.map(|v| v.normalize()),
    )
}

async fn update_product_meters(
    db: &DatabaseConnection,
    id: i32,
    piece: Option<Decimal>,
    roll: Option<Decimal>,
) {
    let (meters_per_piece, meters_per_roll) = pair_piece_roll_meters(piece, roll);

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
            meters_per_piece,
            meters_per_roll,
            user_id: 1,
        })
        .await
        .expect("更新产品米数应成功");
}

#[tokio::test]
async fn create_with_only_piece_lands_same_value_on_both_columns() {
    let db = setup_test_db().await;
    let created = create_product_with(&db, some_dec("50"), None).await;

    let (piece, roll) = reload_meters(&db, created.id).await;
    assert_eq!(piece, Some(dec("50")), "新建时每匹米数应落请求值");
    assert_eq!(
        roll,
        Some(dec("50")),
        "新建时只带每匹米数，每卷米数必须同值落库，否则该产品的按卷报价仍会被拒"
    );
}

#[tokio::test]
async fn create_with_only_roll_lands_same_value_on_both_columns() {
    let db = setup_test_db().await;
    let created = create_product_with(&db, None, some_dec("30")).await;

    let (piece, roll) = reload_meters(&db, created.id).await;
    assert_eq!(
        piece,
        Some(dec("30")),
        "只带每卷米数时每匹米数应补成同值落库"
    );
    assert_eq!(roll, Some(dec("30")), "只带每卷米数时每卷米数应落请求值");
}

#[tokio::test]
async fn create_with_unequal_pair_lands_each_column_separately() {
    let db = setup_test_db().await;
    let created = create_product_with(&db, some_dec("50"), some_dec("30")).await;

    let (piece, roll) = reload_meters(&db, created.id).await;
    assert_eq!(piece, Some(dec("50")), "两值不等时新建的每匹米数应各自落库");
    assert_eq!(
        roll,
        Some(dec("30")),
        "两值不等时新建的每卷米数应各自落库，不得取其一或相互覆写"
    );
}

#[tokio::test]
async fn update_only_piece_on_unequal_history_row_sets_both_columns() {
    let db = setup_test_db().await;
    // 先造一行历史不等数据（改造前合法），再模拟用户在新表单里只改每匹米数
    let created = create_product_with(&db, some_dec("50"), some_dec("30")).await;

    update_product_meters(&db, created.id, some_dec("12"), None).await;

    let (piece, roll) = reload_meters(&db, created.id).await;
    assert_eq!(piece, Some(dec("12")), "改动每匹米数后该列应落新值");
    assert_eq!(
        roll,
        Some(dec("12")),
        "改动每匹米数后每卷米数必须随同源语义带成同值，这是用户主动改动造成的覆写"
    );
}

#[tokio::test]
async fn update_without_meters_keeps_historical_columns_untouched() {
    let db = setup_test_db().await;
    let created = create_product_with(&db, some_dec("50"), some_dec("30")).await;

    // 只改规格，请求不带任何米数键：两列必须原样，证明单入口改造不会顺手改写历史米数
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
            meters_per_piece: None,
            meters_per_roll: None,
            user_id: 1,
        })
        .await
        .expect("只改规格的更新应成功");

    let (piece, roll) = reload_meters(&db, created.id).await;
    assert_eq!(
        piece,
        Some(dec("50")),
        "请求不带米数时每匹米数必须保持历史值"
    );
    assert_eq!(
        roll,
        Some(dec("30")),
        "请求不带米数时每卷米数必须保持历史值，硬边界不动历史数据在此钉住"
    );
}

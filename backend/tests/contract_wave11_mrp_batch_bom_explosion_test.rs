//! Wave11 契约锁：MRP 批量计算链路必须做 BOM 多级展开，且展开结果必须进入 HTTP 响应
//! 的 `requirements` 数组（父行 bom_level=0 + 子行 bom_level≥1），与 `mrp_results` 落库行同源。
//!
//! 缺陷场景（CI run #4676 e2e `frontend/e2e/mrp/01-calculation.spec.ts:141`）：
//! `POST /production/mrp/calculate` → `batch_calculate` 的响应 `requirements` 只装顶层行，
//! BOM 展开的子物料需求从调用方视角不可见（`Received: 1`）。
//!
//! 判据来源（非推测）：
//! - 展开与用量口径：`services/mrp_engine_ops/bom.rs::process_bom_item`
//!   （base = parent_qty * item.qty round_dp(4)；scrap 存储口径 0–1 比率，×(1+scrap)）；
//! - 汇总需求判据：`tests/services_mrp_engine_service_test.rs::test_dqtj_sxydqx`
//!   （items_with_shortage 按含 bom_level=1 行计数）；
//! - e2e 等价场景数值：quantity 3 + scrap 0.1 ⇒ 有效用量 3.3（乘父需求 100 ⇒ 330）、
//!   quantity 5 ⇒ 500，与 `bom_items.scrap_rate DECIMAL(5,4)`（m0007:41）存储口径一致。
//!
//! 分层策略（对齐 `number_generator_residual_timestamp_test.rs` 既有先例）：
//! 1. 静态源码契约（无需 DB，必跑）：批量汇总必须复用 `run_mrp_calculation_for_line`
//!    的同一套展开产出，禁止在本文件出现第二套需求重算（calculate_requirement_with_stock /
//!    get_stock_info_batch 影子链路）；
//! 2. 活库行为层（`setup_test_db`，已迁移 PG、串行组 db-integration）：真实 boms/bom_items
//!    种数后走 `batch_calculate`，钉住「1 父 + 2 子」3 行、330/500 数值、子行落库号
//!    `{批次号}-{行序}-{子行序}`，以及多级（孙级 bom_level=2）数量沿展开链传递。

use bingxi_backend::models::bom::{ActiveModel as BomActiveModel, Entity as BomEntity};
use bingxi_backend::models::bom_item::{
    ActiveModel as BomItemActiveModel, Column as BomItemColumn, Entity as BomItemEntity,
};
use bingxi_backend::models::mrp_result::{Column as MrpResultColumn, Entity as MrpResultEntity};
use bingxi_backend::models::status::common;
use bingxi_backend::services::mrp_engine_ops::types::{
    MaterialRequirement, MrpCalculationItem, MrpCalculationRequest,
};
use bingxi_backend::services::mrp_engine_service::MrpEngineService;
use bingxi_backend::services::test_common::setup_test_db;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

const MRP_CALC: &str = "src/services/mrp_engine_ops/calculation.rs";

fn read(rel: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|e| panic!("夹具数量 {s} 必须可解析: {e}"))
}

fn fixture_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, 31).expect("夹具日期 2026-12-31 必须可解析")
}

// =========================================================
// 静态层：批量路径必须复用行计算（同一套展开与用量逻辑），
// 不得存在第二套需求重算影子链路
// =========================================================

#[test]
fn batch_summary_reuses_line_calculation_without_shadow_recompute() {
    let src = read(MRP_CALC);

    // 行计算必须把展开后的需求行与落库行成对返回（同一份数据源）
    assert!(
        src.contains("Ok((results, requirements))")
            && src.contains("let mut requirements = vec![main_req.clone()];")
            && src.contains("requirements.push(req.clone());"),
        "run_mrp_calculation_for_line 必须返回与落库行同源的需求行（父在前、BOM 子行随后）"
    );
    // 行计算内部必须经 explode_bom 展开（唯一展开入口）
    assert!(
        src.contains(".explode_bom(MrpExplodeQuery {"),
        "行计算必须调用 explode_bom 获取子物料需求，批量与单条共用同一条展开链"
    );
    // build_batch_rows 只许收集行计算产出，禁止另写需求公式
    let batch_body = src
        .split("async fn build_batch_rows(")
        .nth(1)
        .expect("缺少 build_batch_rows");
    assert!(
        batch_body.contains("run_mrp_calculation_for_line(")
            && batch_body.contains("all_results.extend(rows);")
            && batch_body.contains("all_requirements.extend(requirements);"),
        "build_batch_rows 必须复用 run_mrp_calculation_for_line 并把其需求行汇入汇总"
    );
    assert!(
        !batch_body.contains("calculate_requirement_with_stock("),
        "build_batch_rows 不得再走 calculate_requirement_with_stock 第二套重算（影子实现）"
    );
    assert!(
        !src.contains("get_stock_info_batch("),
        "calculation.rs 汇总需求已单一来源化，不得保留库存预取影子链路"
    );
}

// =========================================================
// 活库行为层（setup_test_db：已迁移 PG、串行组 db-integration）
// =========================================================

/// 种一张默认 ACTIVE BOM：items = [(material_id, 用量, 损耗率存储口径0-1比率或None)]，
/// 返回 bom.id。boms.product_id 无 FK（m0007 仅索引），夹具用高段合成 ID 不与真实产品纠缠。
async fn seed_default_bom(
    db: &DatabaseConnection,
    product_id: i32,
    items: &[(i32, &str, Option<&str>)],
) {
    let now = Utc::now();
    let bom = BomActiveModel {
        product_id: Set(product_id),
        version: Set(1),
        is_default: Set(true),
        status: Set(common::STATUS_ACTIVE.to_string()),
        remarks: Set(Some("MRP展开契约夹具BOM".to_string())),
        created_by: Set(1),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 boms 行失败");

    for (material_id, quantity, scrap_rate) in items {
        BomItemActiveModel {
            bom_id: Set(bom.id),
            material_id: Set(*material_id),
            quantity: Set(dec(quantity)),
            unit: Set(Some("千克".to_string())),
            scrap_rate: Set(scrap_rate.map(|s| dec(s))),
            sort_order: Set(Some(0)),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("夹具：种 bom_items 行失败");
    }
}

fn batch_request(items: Vec<(i32, Decimal)>) -> MrpCalculationRequest {
    MrpCalculationRequest {
        items: items
            .into_iter()
            .map(|(product_id, required_quantity)| MrpCalculationItem {
                product_id,
                required_quantity,
                required_date: fixture_date(),
            })
            .collect(),
        source_type: "SALES_ORDER".to_string(),
        source_id: None,
        consider_safety_stock: false,
        consider_in_transit: false,
    }
}

fn find_req(reqs: &[MaterialRequirement], product_id: i32) -> MaterialRequirement {
    reqs.iter()
        .find(|r| r.product_id == product_id)
        .cloned()
        .unwrap_or_else(|| panic!("requirements 必须包含 product_id={product_id} 的行"))
}

async fn exists_row(db: &DatabaseConnection, no: &str) -> bool {
    MrpResultEntity::find()
        .filter(MrpResultColumn::CalculationNo.eq(no))
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("夹具：查询 {no} 失败: {e}"))
        .is_some()
}

/// e2e（frontend/e2e/mrp/01-calculation.spec.ts）等价场景：父产品 100 单位，
/// BOM 两子件（3×1.1 损耗 ⇒ 3.3 有效用量 ⇒ 330；5 ⇒ 500）。
/// 钉住批量响应 requirements 必须为 3 行（1 父 + 2 子）且数值/层级正确、子行真实落库。
#[tokio::test]
async fn batch_calculate_requirements_include_bom_children_with_effective_quantity() {
    let db = setup_test_db().await;
    let (parent, child1, child2) = (910_001_i32, 910_002_i32, 910_003_i32);
    seed_default_bom(
        &db,
        parent,
        &[(child1, "3.0", Some("0.1")), (child2, "5.0", Some("0.0"))],
    )
    .await;

    let svc = MrpEngineService::new(Arc::new(db.clone()));
    let summary = svc
        .batch_calculate(batch_request(vec![(parent, dec("100"))]))
        .await
        .expect("批量 MRP 计算必须成功");

    assert_eq!(
        summary.requirements.len(),
        3,
        "requirements 必须是 1 父 + 2 子共 3 行（e2e 断言本体，曾经只返回 1 行父需求）"
    );
    let parent_req = find_req(&summary.requirements, parent);
    assert_eq!(parent_req.bom_level, 0, "父行 bom_level 必须为 0");
    let child1_req = find_req(&summary.requirements, child1);
    assert_eq!(child1_req.bom_level, 1, "child1 bom_level 必须为 1");
    assert_eq!(
        child1_req.required_quantity,
        dec("330"),
        "child1 有效用量必须按 quantity3×(1+scrap0.1)×父需求100=330（bom.rs::process_bom_item 口径）"
    );
    let child2_req = find_req(&summary.requirements, child2);
    assert_eq!(child2_req.bom_level, 1, "child2 bom_level 必须为 1");
    assert_eq!(
        child2_req.required_quantity,
        dec("500"),
        "child2 应为 100×5=500"
    );

    // 业务判据与 services_mrp_engine_service_test::test_dqtj_sxydqx 同源：
    // 空库（夹具已 TRUNCATE inventory_stock）下 3 行全部存在缺口
    assert_eq!(summary.items_with_shortage, 3);

    // 落库子行号 `{批次号}-0-{子行序}` 必须真实存在（与响应行同源，不是响应侧现造）
    assert!(
        exists_row(&db, &format!("{}-0", summary.calculation_no)).await,
        "主行必须落库"
    );
    for idx in 1..=2 {
        assert!(
            exists_row(&db, &format!("{}-0-{}", summary.calculation_no, idx)).await,
            "BOM 子行 {{批次号}}-0-{idx} 必须落库"
        );
    }
    assert_eq!(
        summary.total_items, 3,
        "total_items 必须计入展开后的全部落库行"
    );
}

/// 多级展开 + 多 item 批次：child1 自身还有默认 BOM（孙级用量 2、无损耗），
/// 需求必须沿展开链传递（330 ⇒ 孙级 660，bom_level=2）；同批第二 item 独立成行。
#[tokio::test]
async fn batch_calculate_multi_level_explosion_chains_quantity_and_isolates_items() {
    let db = setup_test_db().await;
    let (parent, child1, child2, grandchild, standalone) = (
        910_001_i32,
        910_002_i32,
        910_003_i32,
        910_004_i32,
        910_005_i32,
    );
    seed_default_bom(
        &db,
        parent,
        &[(child1, "3.0", Some("0.1")), (child2, "5.0", Some("0.0"))],
    )
    .await;
    seed_default_bom(&db, child1, &[(grandchild, "2.0", None)]).await;

    let svc = MrpEngineService::new(Arc::new(db.clone()));
    let summary = svc
        .batch_calculate(batch_request(vec![
            (parent, dec("100")),
            (standalone, dec("50")),
        ]))
        .await
        .expect("多 item 批量 MRP 计算必须成功");

    assert_eq!(
        summary.requirements.len(),
        5,
        "两 item 汇总必须为 父+子+孙+子 + 独立父 = 5 行"
    );
    let gc = find_req(&summary.requirements, grandchild);
    assert_eq!(gc.bom_level, 2, "孙级 bom_level 必须为 2");
    assert_eq!(
        gc.required_quantity,
        dec("660"),
        "孙级数量必须用 child1 展开后的 330 作父量（330×2=660），不得回退用原始 100"
    );
    let st = find_req(&summary.requirements, standalone);
    assert_eq!(st.bom_level, 0, "无 BOM 的独立 item 只出父行");
    assert_eq!(
        st.required_quantity,
        dec("50"),
        "独立 item 需求不受同批其他行污染"
    );
    assert_eq!(summary.total_items, 5);

    // 两条批行的行号前缀互不相同：`{批次号}-0`（含子行 -0-1/-0-2/-0-3）与 `{批次号}-1`
    for no in [
        format!("{}-0", summary.calculation_no),
        format!("{}-0-1", summary.calculation_no),
        format!("{}-0-2", summary.calculation_no),
        format!("{}-0-3", summary.calculation_no),
        format!("{}-1", summary.calculation_no),
    ] {
        assert!(exists_row(&db, &no).await, "行 {no} 必须真实落库");
    }

    // 夹具种下的 BOM 行不应把孙级分支再展开成环：child2/standalone 无默认 BOM，
    // explode_bom 对无 BOM 产品返回空（bom.rs::explode_bom_recursive 的 None 分支）
    let bom_count = BomEntity::find()
        .all(&db)
        .await
        .expect("查 boms 失败")
        .len();
    let item_count = BomItemEntity::find()
        .all(&db)
        .await
        .expect("查 bom_items 失败")
        .len();
    assert_eq!(bom_count, 2, "夹具只应存在两张 BOM（行为锁的对照前提）");
    assert_eq!(item_count, 3, "夹具只应存在 3 条明细");
}

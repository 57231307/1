//! 任务 #246 ②「`inventory_piece` 三列实测值缺值域 CHECK」—— 真库契约锁（后端+DB 线）
//!
//! 钉死两件事（同一取值域的两道门，缺一道都算没修）：
//! 1. **DB 兜底（migration m0076）**：`weight` / `width` / `gram_weight` 三条
//!    `chk_inventory_piece_*_positive`（`IS NULL OR > 0`）必须真实存在于 pg_constraint、
//!    `convalidated`，且真库旁路直写 0 / 负数必须被数据库拒绝。
//!    ⇒ 本例同时是**注册链的活体证明**：m0076 未在 `domain/production/mod.rs` 注册前，
//!    本文件第 1/2 例必红（这正是"迁移文本写了 ≠ 库里生效"要防的假绿）。
//! 2. **服务层同口径**：匹行的三条落库路径（生产报工建匹 / 拆匹剪裁 / 委外收回产匹）
//!    必须与 DB 同口径 `> 0`（拒绝归校验族，与打卷门 `fabric_inspection_service.rs:601-617`、
//!    收回门 `outsourcing_ops/receipt.rs:65-87` 同族同码），禁止"只加 DB 约束、放任服务层写 0"。
//!
//! 为什么这三列值得单独一条门：它们是成品布入库标签的**直读源**
//! （`print_service.rs:4991-5011` 逐列判空后原样印出），标签只判 NULL 不判取值 ⇒
//! 一条 `weight=0` 的行会被当成"已实测"直接印上实物标签，0 就是伪造实测值。
//!
//! 夹具约定：表结构唯一来源 = `backend/migration`（不自建 DDL）；`setup_test_db()` 缺
//! `TEST_DATABASE_URL` 直接 panic（禁 sqlite 回退）；FK 父行（仓库/产品）自种子。

mod test_common;

use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use bingxi_backend::models::{inventory_piece, product, warehouse};
use bingxi_backend::services::piece_domain_service::{
    PIECE_TYPE_DYED, PIECE_TYPE_GREIGE, ReportPieceInput, create_greige_pieces_from_report,
};
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DbBackend, EntityTrait, QueryFilter, Statement, Value,
};
use std::str::FromStr;
use std::sync::Arc;

use test_common::setup_test_db;

const WH_ID: i32 = 9511;
const PRODUCT_ID: i32 = 9512;

/// Decimal 解析（夹具一律以字符串写值，避免浮点精度失真）
fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|e| panic!("夹具小数 {s} 解析失败: {e}"))
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_ref_rows(db: &Arc<DatabaseConnection>) {
    warehouse::ActiveModel {
        id: Set(WH_ID),
        warehouse_code: Set("WH-W8-PIECE".to_string()),
        name: Set("实测值门仓".to_string()),
        // 未指定类型：报工建匹（greige）与拆匹均不被仓库类型门拦住，本测只验取值域门
        warehouse_type: Set(None),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("种子仓库插入失败");

    product::ActiveModel {
        id: Set(PRODUCT_ID),
        code: Set("FAB-W8-CHK".to_string()),
        name: Set("实测值门面料".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("种子产品插入失败");
}

/// 裸插一行匹记录（绕开所有服务口 = 模拟旁路/并发写入，正是 CHECK 要拦的那一类）
fn bare_piece(
    piece_no: &str,
    weight: Option<Decimal>,
    width: Option<Decimal>,
    gram_weight: Option<Decimal>,
) -> inventory_piece::ActiveModel {
    inventory_piece::ActiveModel {
        piece_no: Set(piece_no.to_string()),
        piece_type: Set(PIECE_TYPE_DYED.to_string()),
        status: Set(piece_status::AVAILABLE.to_string()),
        inventory_status: Set(Some(piece_status::AVAILABLE.to_string())),
        warehouse_id: Set(WH_ID),
        product_id: Set(PRODUCT_ID),
        batch_no: Set("W8BATCH".to_string()),
        color_no: Set("W8C".to_string()),
        dye_lot_no: Set("W8LOT".to_string()),
        length: Set(dec("50.00")),
        weight: Set(weight),
        width: Set(width),
        gram_weight: Set(gram_weight),
        barcode: Set(Some(piece_no.to_string())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
}

async fn query_one_string(db: &Arc<DatabaseConnection>, sql: &str, arg: &str) -> Option<String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql.to_string(),
            vec![Value::String(Some(arg.to_string()))],
        ))
        .await
        .unwrap_or_else(|e| panic!("查询失败: {e}"));
    rows.first().map(|r| {
        r.try_get_by_index::<String>(0)
            .unwrap_or_else(|e| panic!("解码失败: {e}"))
    })
}

// ---------------------------------------------------------------------------
// 1. 结构锁：三条正值 CHECK 必须真实存在、validated；三列仍可空（NULL=未补录）
//    改坏什么必红：m0076 未注册 / 注册链位置错（表还没建） / 约束名改动 /
//    用 `ADD CONSTRAINT ... NOT VALID` 蒙过存量 ⇒ 本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn m0076_positive_checks_are_live_in_inventory_piece() {
    let db = Arc::new(setup_test_db().await);

    for column in ["weight", "width", "gram_weight"] {
        let nn = query_one_string(
            &db,
            "SELECT is_nullable FROM information_schema.columns \
              WHERE table_schema = 'public' AND table_name = 'inventory_piece' \
                AND column_name = $1",
            column,
        )
        .await;
        assert_eq!(
            nn.as_deref(),
            Some("YES"),
            "{column} 必须仍可空（NULL 表达「未补录实测值」，m0076 不得改列形态）"
        );

        let def = query_one_string(
            &db,
            "SELECT COALESCE(pg_get_constraintdef(c.oid), '<缺失>') \
               FROM pg_constraint c \
              WHERE c.conrelid = 'public.inventory_piece'::regclass \
                AND c.contype = 'c' AND c.conname = $1",
            &format!("chk_inventory_piece_{column}_positive"),
        )
        .await
        .expect("查询 CHECK 定义不应失败");
        assert_ne!(
            def, "<缺失>",
            "缺少 CHECK chk_inventory_piece_{column}_positive：三列是标签直读源，\
             没有值域约束时旁路/并发写入的 0 与负数可落库并被印上标签（m0076）"
        );
        // 归一化后必须是"列 IS NULL OR 列 > 0"（>= 会放过 0；漏 IS NULL 会把未补录当违规）
        let norm: String = def
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '(' && *c != ')' && *c != '"')
            .collect::<String>()
            .replace("::numeric", "");
        assert_eq!(
            norm,
            format!("CHECK{column}ISNULLOR{column}>0"),
            "{column} 的 CHECK 定义与 m0075/m0076 同口径不符，实得: {def}"
        );

        let validated = query_one_string(
            &db,
            "SELECT c.convalidated::text FROM pg_constraint c \
              WHERE c.conrelid = 'public.inventory_piece'::regclass \
                AND c.contype = 'c' AND c.conname = $1",
            &format!("chk_inventory_piece_{column}_positive"),
        )
        .await;
        assert_eq!(
            validated.as_deref(),
            Some("true"),
            "{column} 的 CHECK 未 convalidated（对存量不生效）＝门只对新写入半关"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. 活性锁：真库旁路直写 0 / 负数必须被拒且零落库；NULL 与正值是合法形态
//    改坏什么必红：约束被 DROP / 被改成 >= 0 / 只加了注释没加约束 ⇒ 本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn bypass_write_of_zero_or_negative_measured_value_is_refused_by_db() {
    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;

    for (case, column, value) in [
        ("zero-weight", "weight", dec("0")),
        ("neg-width", "width", dec("-1.5")),
        ("zero-gram", "gram_weight", dec("0.0000")),
    ] {
        let (w, wd, g) = match column {
            "weight" => (Some(value), None, None),
            "width" => (None, Some(value), None),
            _ => (None, None, Some(value)),
        };
        let err = bare_piece(&format!("W8-{case}"), w, wd, g)
            .insert(db.as_ref())
            .await
            .expect_err("{column}=0 或负数不得落库（伪造实测值会被直接印上标签）");
        let msg = err.to_string();
        assert!(
            msg.contains(&format!("chk_inventory_piece_{column}_positive"))
                || msg.contains("23514"),
            "拒绝原因必须是该列的 CHECK 违例（可归因到列），实际: {msg}"
        );

        let left = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::PieceNo.eq(format!("W8-{case}")))
            .all(db.as_ref())
            .await
            .unwrap();
        assert!(
            left.is_empty(),
            "被 CHECK 拒掉的写入必须零落库（不得留半行）"
        );
    }

    // 对照一：NULL = 未补录，合法（标签侧继续 fail-closed 点名，不在 DB 造值）
    let null_row = bare_piece("W8-legal-null", None, None, None)
        .insert(db.as_ref())
        .await
        .expect("三列 NULL 必须是合法形态（m0076 只加值域，不改成 NOT NULL）");
    assert_eq!(
        (null_row.weight, null_row.width, null_row.gram_weight),
        (None, None, None)
    );

    // 对照二：正值合法，且逐列落库不被改写
    let ok = bare_piece(
        "W8-legal-positive",
        Some(dec("21.5")),
        Some(dec("185")),
        Some(dec("200")),
    )
    .insert(db.as_ref())
    .await
    .expect("正的实测值必须可落库");
    assert_eq!(ok.weight, Some(dec("21.5")));
    assert_eq!(ok.width, Some(dec("185")));
    assert_eq!(ok.gram_weight, Some(dec("200")));
}

// ---------------------------------------------------------------------------
// 3. 服务层同口径（生产报工建匹）：非空必须正值，越界 400 校验族 + 整批零落库
//    改坏什么必红：删掉 `create_greige_pieces_from_report` 里的预检 → 该写入会
//    先落前几匹再被 DB CHECK 打断（部分写入 + 裸 23514）→ 本例的"零落库 + 点名"红；
//    把拒绝码改成 BUSINESS_ERROR（族错配）→ 红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn report_registration_gate_rejects_non_positive_measured_values() {
    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;

    let mk = |no: &str, weight: Option<Decimal>, width: Option<Decimal>, gram: Option<Decimal>| {
        ReportPieceInput {
            piece_no: no.to_string(),
            machine_no: Some("M-W8".to_string()),
            machine_operator: Some("开机人".to_string()),
            length: dec("100.00"),
            weight,
            width,
            gram_weight: gram,
            warehouse_id: WH_ID,
            production_date: None,
        }
    };

    // 三条列各自越界；批内混一条合法匹（锁"整批预检"而非"逐匹边写边判"）
    let cases: Vec<(&str, Vec<ReportPieceInput>)> = vec![
        (
            "零重量",
            vec![
                mk("W8-RPT-OK", Some(dec("1")), None, None),
                mk("W8-RPT-ZERO", Some(Decimal::ZERO), None, None),
            ],
        ),
        (
            "负幅宽",
            vec![mk("W8-RPT-NEGW", None, Some(dec("-2")), None)],
        ),
        (
            "零克重",
            vec![mk("W8-RPT-ZEROG", None, None, Some(Decimal::ZERO))],
        ),
    ];
    for (title, batch) in cases {
        let err =
            create_greige_pieces_from_report(db.as_ref(), "MO-W8-CHK", PRODUCT_ID, Some(1), &batch)
                .await
                .unwrap_err();
        assert_eq!(
            err.error_code(),
            "VALIDATION_ERROR",
            "{title}：值域越界属校验族（与打卷门/收回门同码），实得 {err:?}"
        );
        assert!(
            matches!(err, AppError::ValidationErrorDisplayable(_)),
            "{title}：必须外显真实原因（用户自己提交的字段），实得 {err:?}"
        );
        let msg = err.to_response().message;
        assert!(
            msg.contains("重量(weight)")
                || msg.contains("幅宽(width)")
                || msg.contains("克重(gram_weight)"),
            "{title}：拒绝必须点名是哪一列，实际: {msg}"
        );
        assert!(
            msg.contains("必须大于 0"),
            "{title}：文案要给出口径，实际: {msg}"
        );

        // 整批零落库（预检在任何 DB 访问之前，不给"前几匹已写入"留窗口）
        let left = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::BatchNo.eq("MO-W8-CHK"))
            .all(db.as_ref())
            .await
            .unwrap();
        assert!(
            left.is_empty(),
            "{title}：整批预检必须拦住，实得 {} 行已落库",
            left.len()
        );
    }

    // 对照：留空（未补录）与正值都合法，且如实落 NULL（不塞 0、不回落 products 标称值）
    let created = create_greige_pieces_from_report(
        db.as_ref(),
        "MO-W8-OK",
        PRODUCT_ID,
        Some(1),
        &[
            mk("W8-RPT-NULL", None, None, None),
            mk(
                "W8-RPT-POS",
                Some(dec("50")),
                Some(dec("160")),
                Some(dec("210")),
            ),
        ],
    )
    .await
    .expect("合法报工建匹不得被误伤");
    assert_eq!(created.len(), 2);
    assert_eq!(
        (created[0].weight, created[0].width, created[0].gram_weight),
        (None, None, None),
        "未补录必须保持 NULL"
    );
    assert_eq!(created[1].weight, Some(dec("50")));
    assert_eq!(created[1].piece_type, PIECE_TYPE_GREIGE);
}

// ---------------------------------------------------------------------------
// 4. 服务层同口径（拆匹）：cut_weight 越界与"整卷拆尽"都在写库前被拒，母卷零变化
//    改坏什么必红：把 `update_parent_piece` 的差值写回而不判额度 ⇒ 母卷剩余重量落 0
//    → 被 m0076 CHECK 打断成裸 23514（本例断言的是 400 业务文案 + 零变化）→ 红；
//    删掉 cut_weight 的取值域门 ⇒ 0 公斤子卷落库 → 红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn split_rejects_zero_cut_weight_and_whole_roll_split() {
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode, header},
        routing::post,
    };
    use bingxi_backend::container::AppState;
    use bingxi_backend::handlers::piece_split_handler;
    use serde_json::{Value as Json, json};
    use tower::ServiceExt;

    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;
    inventory_piece::ActiveModel {
        id: Set(9520),
        piece_no: Set("W8-PARENT".to_string()),
        piece_type: Set(PIECE_TYPE_DYED.to_string()),
        status: Set(piece_status::AVAILABLE.to_string()),
        inventory_status: Set(Some(piece_status::AVAILABLE.to_string())),
        warehouse_id: Set(WH_ID),
        product_id: Set(PRODUCT_ID),
        batch_no: Set("W8BATCH".to_string()),
        color_no: Set("W8C".to_string()),
        dye_lot_no: Set("W8LOT".to_string()),
        length: Set(dec("100.00")),
        weight: Set(Some(dec("50.00"))),
        width: Set(Some(dec("185.00"))),
        gram_weight: Set(Some(dec("200.00"))),
        barcode: Set(Some("W8-PARENT".to_string())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("种子母卷失败");

    // 被测端点的真实形状：`split_fabric_piece(State, Json)` 不取 AuthContext
    // （鉴权由 nest 层中间件负责），本测只验取值域门，不额外挂中间件伪造前提。
    let app = Router::new()
        .route(
            "/inventory/piece-split",
            post(piece_split_handler::split_fabric_piece),
        )
        .with_state(AppState {
            db: db.clone(),
            ..Default::default()
        });

    let call = |body: Json| {
        let app = app.clone();
        async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/inventory/piece-split")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = resp.status();
            let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
                .await
                .unwrap()
                .to_vec();
            let v: Json =
                serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("失败信封应是 JSON: {e}"));
            (status, v)
        }
    };

    // (a) cut_weight = 0：伪造实测值 ⇒ 400 校验族并点名重量列
    let (status, v) = call(json!({
        "parent_piece_id": 9520, "cut_length": "10.00", "cut_weight": "0",
    }))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "信封: {v}");
    assert_eq!(v["code"], "VALIDATION_ERROR", "信封: {v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("重量(weight)"),
        "应点名重量列，实际: {}",
        v["message"]
    );

    // (b) cut_weight 等于母卷实测重量：母卷剩余会被写成 0（m0076 禁止的伪实测值形态）
    //     ⇒ 400 业务族给出出口（整卷请直接出库），且母卷重量分毫未动
    let (status, v) = call(json!({
        "parent_piece_id": 9520, "cut_length": "10.00", "cut_weight": "50.0000",
    }))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "信封: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR", "信封: {v}");
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("必须小于") && msg.contains("整卷"),
        "文案要给出口径（整卷无需拆分），实际: {msg}"
    );
    let parent = inventory_piece::Entity::find_by_id(9520)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parent.weight, Some(dec("50.00")), "被拒的拆分不得扣母卷");
    assert_eq!(parent.length, dec("100.00"), "被拒的拆分不得扣母卷长度");
    let children = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::ParentPieceId.eq(9520))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(children.is_empty(), "被拒的拆分不得留下子卷行");

    // (c) 正常拆分：母卷剩余实测重量仍为正值（这正是 m0076 值域要求的形态）
    let (status, v) = call(json!({
        "parent_piece_id": 9520, "cut_length": "10.00", "cut_weight": "5.0000",
        "new_barcode": "W8-CHILD-1",
    }))
    .await;
    assert_eq!(status, StatusCode::OK, "正常拆匹应成功，信封: {v}");
    let parent = inventory_piece::Entity::find_by_id(9520)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parent.weight, Some(dec("45.00")));
    let child = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq("W8-CHILD-1"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("子卷应落库");
    assert_eq!(child.weight, Some(dec("5.00")));
    assert_eq!(child.width, Some(dec("185.00")), "子卷幅宽继承母卷实测值");
    assert!(child.weight.unwrap() > Decimal::ZERO);
}

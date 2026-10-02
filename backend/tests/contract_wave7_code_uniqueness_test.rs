//! Wave-I（CI #4669 I 族）染化料单据族编码唯一性契约锁
//!
//! 锁定的契约面（文件:行号以本波修复后为准）：
//! - `backend/src/services/chemical_ops/category.rs::create`：`category_code` 对
//!   **未删除行**查重命中 → 400 `BUSINESS_ERROR` 且 message 为公开规则真实文案
//!   （非脱敏常量「业务处理失败」、不含表名/SQL/并发约束细节/其他记录字段）；
//!   INSERT 竞态兜底：撞 DB 唯一约束（`SqlErr::UniqueConstraintViolation`，23505）
//!   时同口径降级为业务拒绝（单语句 INSERT 原子失败、整行不落库不留半行）。
//! - `backend/src/services/chemical_ops/master.rs`：`chemical_code` 同上口径。
//! - `backend/src/services/chemical_ops/lot.rs`：`lot_no` 同上口径。
//! - 本表族既定语义（三端同源，非本波臆造）：`category.rs:50-60` /
//!   `master.rs::check_chemical_code_uniqueness` / `lot.rs::create` 的查重均
//!   `filter(IsDeleted.eq(false))`，即**软删后同码可复用**（e2e 70-01/70-02 同锁此语义）。
//!
//! ④ 说明（活库并发「撞 UNIQUE 不留半行」`#[ignore]` 用例）：chemical_category /
//! chemical_master / chemical_lot 三表在迁移中**均无**编码列 UNIQUE 索引
//! （`grep -rniE "unique" backend/migration/src | grep -i chemical` 为空），
//! 该用例不成立条件、不预写假断言；待数据库专家按 wave-i 报告补
//! `WHERE is_deleted = false` 部分唯一索引后，另行追加活库并发用例。
//! 本波先以源码扫描锁钉住三文件 INSERT 路径的 23505→业务拒绝归类不得回潮
//! （回潮形态 = `From<DbErr>` 拍平 500 DATABASE_ERROR）。
//!
//! 夹具纪律：`mod test_common;` + `test_common::setup_test_db()` —— 缺
//! `TEST_DATABASE_URL` 即 panic（禁 sqlite 回退、禁自建 DDL）；批次用例的 FK
//! 父行（chemical_master）经本进程内真实建单 API 自种子。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::chemical_handler;
use chrono::Utc;
use serde_json::{Value, json};
use tower::ServiceExt;

/// 本用例专属唯一码（纳秒后缀，防撞既有残留行）
fn uniq_code(prefix: &str) -> String {
    format!(
        "{}-W7-{}",
        prefix,
        Utc::now().timestamp_nanos_opt().unwrap()
    )
}

async fn build_app() -> Router {
    let db = test_common::setup_test_db().await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/chemical-categories",
            post(chemical_handler::create_chemical_category),
        )
        .route(
            "/chemical-categories/{id}",
            get(chemical_handler::get_chemical_category)
                .delete(chemical_handler::delete_chemical_category),
        )
        .route("/chemicals", post(chemical_handler::create_chemical))
        .route(
            "/chemicals/{id}",
            get(chemical_handler::get_chemical).delete(chemical_handler::delete_chemical),
        )
        .route(
            "/chemical-lots",
            post(chemical_handler::create_chemical_lot),
        )
        .with_state(state)
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            builder.body(Body::from(v.to_string())).unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// 拒绝出参的「非脱敏常量 + 不含第三方/内部信息」双向钉（先例：piece 四维出库契约测）
fn assert_reject_envelope(v: &Value, status: StatusCode, code_in_msg: &str) {
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "重复编码必须 400 拒绝：{v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "机器码应为 BUSINESS_ERROR：{v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert_ne!(
        msg, "业务处理失败",
        "拒绝原因必须外显真实文案（不得是脱敏常量）：{msg}"
    );
    assert!(msg.contains("已存在"), "文案须说明重复规则：{msg}");
    assert!(
        msg.contains(code_in_msg),
        "文案须回显用户自己提交的编码：{msg}"
    );
    // 禁止泄漏内部信息与他人记录字段
    for needle in [
        "chemical_category",
        "chemical_master",
        "chemical_lot",
        "SQL",
        "INSERT",
        "23505",
        "unique",
        "UNIQUE",
        "constraint",
    ] {
        assert!(
            !msg.contains(needle),
            "message 不得含表名/SQL/约束细节「{needle}」：{msg}"
        );
    }
}

// =========================================================
// ① + ② 分类：建单回读落库 → 重复编码 4xx（真实文案、机器码正确）
// =========================================================

#[tokio::test]
async fn w7_category_duplicate_code_rejected_with_real_message() {
    let app = build_app().await;
    let code = uniq_code("W7CRT");
    let first = send(
        &app,
        "POST",
        "/chemical-categories",
        Some(json!({
            "category_code": code,
            "category_name": format!("W7根分类{code}"),
            "category_type": "dye",
        })),
    )
    .await;
    assert_eq!(first.0, StatusCode::OK, "首次建分类应 200：{}", first.1);
    let id = first.1["data"]["id"].as_i64().unwrap();
    assert!(id > 0);

    // ② 回读确证编码列已逐字落库（非仅在响应里回声）
    let read = send(&app, "GET", &format!("/chemical-categories/{id}"), None).await;
    assert_eq!(read.0, StatusCode::OK);
    assert_eq!(
        read.1["data"]["category_code"], code,
        "category_code 应真实落库"
    );

    // ① 未删行重复编码 → 400 真实拒绝；提交第二行名称「重复行B」，
    //    断言首行名称不回显（不含其他记录字段）
    let dup = send(
        &app,
        "POST",
        "/chemical-categories",
        Some(json!({
            "category_code": code,
            "category_name": "重复行B",
            "category_type": "dye",
        })),
    )
    .await;
    assert_reject_envelope(&dup.1, dup.0, &code);
    assert!(
        !dup.1["message"].as_str().unwrap().contains("根分类"),
        "message 不得泄漏既有记录名称：{}",
        dup.1["message"]
    );
}

// =========================================================
// ③ 分类：软删后同码可再建（本表既定语义，非新口径）
// =========================================================

#[tokio::test]
async fn w7_category_same_code_reusable_after_soft_delete() {
    let app = build_app().await;
    let code = uniq_code("W7CRT");
    let first = send(
        &app,
        "POST",
        "/chemical-categories",
        Some(json!({
            "category_code": code,
            "category_name": format!("W7一号{code}"),
            "category_type": "dye",
        })),
    )
    .await;
    assert_eq!(first.0, StatusCode::OK, "首次建分类应 200：{}", first.1);
    let id1 = first.1["data"]["id"].as_i64().unwrap();

    // 软删（DELETE = is_deleted=true，category.rs::delete）
    let del = send(&app, "DELETE", &format!("/chemical-categories/{id1}"), None).await;
    assert_eq!(del.0, StatusCode::OK, "软删应 200：{}", del.1);

    // 查重只针对未删行 → 同码可复用（与 e2e 70-01 对 childCode、70-02 对 chemical
    // 的复用断言三端同源）；新行 id 必须不同（新记录而非复活）
    let again = send(
        &app,
        "POST",
        "/chemical-categories",
        Some(json!({
            "category_code": code,
            "category_name": format!("W7二号{code}"),
            "category_type": "dye",
        })),
    )
    .await;
    assert_eq!(again.0, StatusCode::OK, "软删后同码应可再建：{}", again.1);
    let id2 = again.1["data"]["id"].as_i64().unwrap();
    assert_ne!(id1, id2, "复用同码应落新行");
    let read = send(&app, "GET", &format!("/chemical-categories/{id2}"), None).await;
    assert_eq!(read.1["data"]["category_code"], code);
}

// =========================================================
// ① + ② 主数据 chemical_code 同族收口
// =========================================================

#[tokio::test]
async fn w7_chemical_master_duplicate_code_rejected_with_real_message() {
    let app = build_app().await;
    let code = uniq_code("W7CH");
    let first = send(
        &app,
        "POST",
        "/chemicals",
        Some(json!({
            "chemical_code": code,
            "chemical_name": format!("W7化料{code}"),
            "chemical_type": "chemical",
            "unit": "kg",
        })),
    )
    .await;
    assert_eq!(first.0, StatusCode::OK, "首次建主数据应 200：{}", first.1);
    let id = first.1["data"]["id"].as_i64().unwrap();

    let read = send(&app, "GET", &format!("/chemicals/{id}"), None).await;
    assert_eq!(
        read.1["data"]["chemical_code"], code,
        "chemical_code 应真实落库"
    );

    let dup = send(
        &app,
        "POST",
        "/chemicals",
        Some(json!({
            "chemical_code": code,
            "chemical_name": "重复化料B",
            "chemical_type": "chemical",
            "unit": "kg",
        })),
    )
    .await;
    assert_reject_envelope(&dup.1, dup.0, &code);
    assert!(
        !dup.1["message"].as_str().unwrap().contains("化料"),
        "既有行名称不得回显（仅用户自己提交的编码可外显）：{}",
        dup.1["message"]
    );
}

// =========================================================
// ① + ② 批次 lot_no 同族收口（FK/引用父行经真实建单 API 自种子）
// =========================================================

#[tokio::test]
async fn w7_lot_duplicate_lot_no_rejected_with_real_message() {
    let app = build_app().await;
    // 父行自种子：批次必须挂在真实存在的染化料主数据上（lot.rs::create 校验）
    let chem = send(
        &app,
        "POST",
        "/chemicals",
        Some(json!({
            "chemical_code": uniq_code("W7CH"),
            "chemical_name": "W7批次父化料",
            "chemical_type": "chemical",
            "unit": "kg",
        })),
    )
    .await;
    assert_eq!(chem.0, StatusCode::OK, "父化料建单应 200：{}", chem.1);
    let chemical_id = chem.1["data"]["id"].as_i64().unwrap();

    let lot_no = uniq_code("W7LOT");
    let body = json!({
        "lot_no": lot_no,
        "chemical_id": chemical_id,
        "quantity_received": "10",
        "unit_cost": "1.00",
    });
    let first = send(&app, "POST", "/chemical-lots", Some(body.clone())).await;
    assert_eq!(first.0, StatusCode::OK, "首次建批次应 200：{}", first.1);
    let lot_id = first.1["data"]["id"].as_i64().unwrap();
    assert_eq!(
        first.1["data"]["lot_no"].as_str().unwrap(),
        lot_no,
        "lot_no 应逐字落库回显"
    );
    assert!(lot_id > 0);

    let dup = send(&app, "POST", "/chemical-lots", Some(body)).await;
    assert_reject_envelope(&dup.1, dup.0, &lot_no);
}

// =========================================================
// 源码扫描锁：三文件「查重 + INSERT 竞态兜底」范式不得回潮
// =========================================================

#[test]
fn w7_chemical_ops_unique_race_fallback_locked() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services");
    // 每个文件：查重走可外显拒绝；INSERT 路径必须分类 23505（禁止 From<DbErr>/database()
    // 原样拍平成 500 DATABASE_ERROR）；判重文案不得回潮成脱敏的 AppError::business。
    // 源码按去空白归一后匹配，避免跨行 call 的缩进脆断（先例：迁移形态扫描锁）。
    let cases = [
        (
            "chemical_ops/category.rs",
            "分类编码 {} 已存在，请更换编码后重试",
            "分类编码{}已存在",
        ),
        (
            "chemical_ops/master.rs",
            "染化料编码 {} 已存在，请更换编码后重试",
            "染化料编码{}已存在",
        ),
        (
            "chemical_ops/lot.rs",
            "批号 {} 已存在，请更换批号后重试",
            "批号{}已存在",
        ),
    ];
    for (rel, public_msg, key) in cases {
        let path = manifest.join(rel);
        let src =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
        let flat: String = src.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            flat.contains("SqlErr::UniqueConstraintViolation"),
            "{rel} 缺少 INSERT 撞唯一约束(23505)的竞态兜底分类：并发同码将以 500 DATABASE_ERROR 裸抛"
        );
        assert!(
            src.contains(public_msg),
            "{rel} 缺少公开规则文案「{public_msg}」（重复编码拒绝必须可外显，非脱敏常量）"
        );
        let masked = format!("AppError::business(format!(\"{key}");
        assert!(
            !flat.contains(&masked),
            "{rel} 判重文案「{key}」回潮为脱敏的 AppError::business 变体：真因将被出参吞掉"
        );
        let displayable = format!("AppError::business_displayable(format!(\"{key}");
        assert!(
            flat.contains(&displayable),
            "{rel} 判重拒绝「{key}」必须走 business_displayable（可外显业务族，先例 #165 流程编码）"
        );
    }
}

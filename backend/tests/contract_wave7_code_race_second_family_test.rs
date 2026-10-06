//! Wave「编码唯一性/裸 500 同族」第二收口族契约锁（warehouse / supplier / outsourcing）
//!
//! 锁定的契约面（文件:行号以本波修复后为准，判责来源 wave-i §③ 待收口表）：
//! - `backend/src/services/warehouse_service.rs::create` 人工指定码分支：
//!   ①同事务预校验命中 `warehouses.warehouse_code` 已有值 → 400 `BUSINESS_ERROR`
//!   + 可外显真实文案（回显用户自己提交的编码，禁表名/约束名/内部信息）；
//!   ②INSERT 撞 warehouse_code UNIQUE(23505) 的竞态兜底同口径降级业务拒绝，
//!   单语句原子失败、整事务回滚、不留半行（本表有真实 DB UNIQUE，
//!   m0001_initial_schema.rs:281 —— 本文件以"未提交旁路重复行挂起 INSERT"的方式
//!   真实命中该兜底分支一次，禁止只以源码扫描冒充）。
//! - `backend/src/services/supplier_service.rs`：
//!   ①取号+判重+INSERT 收敛到 `DocumentNumberGenerator::insert_with_no_retry`
//!   唯一实现（suppliers.supplier_code 有 UNIQUE，m0001:303，禁止再走
//!   「generate → 直插 `?`」的裸 500 旁路）；
//!   ②改名判重收敛到 `check_supplier_name_unique` 唯一实现（与 create 同口径），
//!   改到他人已占用名称 → 400 可外显；与自身当前名相同的幂等 PUT 放行。
//! - `backend/src/services/outsourcing_ops/{order,receipt,voucher}.rs`：
//!   "单号重复/缺前置"拒绝从脱敏 `AppError::business` 升为可外显
//!   `business_displayable`（400 `BUSINESS_ERROR` 机器码不变，不破坏既有响应结构）；
//!   INSERT 路径 23505 竞态兜底就位（三表单号列现仅 NOT NULL 无 DB UNIQUE，
//!   v15/mod.rs:3229/:691/:1179 —— 兜底分支在 DB 专家补唯一索引前不可达，
//!   以源码扫描锁钉住归类形态不回潮；权限拒绝 401/403 不在本族、保持脱敏未动）。
//!
//! 夹具纪律：`mod test_common;` + `test_common::setup_test_db()` —— 缺
//! `TEST_DATABASE_URL` 即 panic（禁 sqlite 回退、禁自建 DDL）；FK/前置父行
//! （供应商、委外订单、成品）经真实建单 API 或既有表的单行种子自举。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::{Next, from_fn},
    response::Response,
    routing::{get, post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{outsourcing_handler, supplier_handler, warehouse_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

/// 本用例注入的假操作人（warehouses/suppliers 的 created_by 无 FK 约束，
/// audit_logs.user_id 亦无 FK——sales_orders 才约束 created_by，本族不触碰）
const AUTH_USER_ID: i32 = 90771;

/// 测试侧注入认证上下文（warehouse/supplier 的 handler 从请求扩展取
/// AuthContext；data_scope=all 避开行级数据范围干扰，被测对象是判重族本身。
/// axum 0.8 的 `middleware::Next` 不再泛型化 body，直接取具体 `Request`）
async fn inject_auth(mut req: Request, next: Next) -> Response {
    req.extensions_mut().insert(AuthContext {
        user_id: AUTH_USER_ID,
        username: "w7_second_family_test".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    });
    next.run(req).await
}

async fn build_app() -> (Router, Arc<DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route("/warehouses", post(warehouse_handler::create))
        .route("/warehouses/{id}", get(warehouse_handler::get))
        .route("/suppliers", post(supplier_handler::create_supplier))
        .route(
            "/suppliers/{id}",
            put(supplier_handler::update_supplier).get(supplier_handler::get_supplier),
        )
        .route(
            "/outsourcing-orders",
            post(outsourcing_handler::create_outsourcing_order),
        )
        .route(
            "/outsourcing-receipts",
            post(outsourcing_handler::create_outsourcing_receipt),
        )
        .route(
            "/outsourcing-vouchers",
            post(outsourcing_handler::create_outsourcing_voucher),
        )
        .layer(from_fn(inject_auth))
        .with_state(state);
    (app, db)
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

/// 本用例专属唯一码（纳秒后缀，防撞既有残留行；仅 [A-Z0-9-]，可安全拼入
/// 本文件自种的 raw SQL 探针语句——探针只 INSERT 既有表既有列，不建任何 DDL）
fn uniq_code(prefix: &str) -> String {
    format!(
        "{}-W7S-{}",
        prefix,
        Utc::now().timestamp_nanos_opt().unwrap()
    )
}

async fn scalar_i64(db: &DatabaseConnection, sql: String) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .expect("探针计数 SQL 执行失败")
        .expect("探针计数应返回一行");
    row.try_get_by_index::<i64>(0).expect("count 应解码为 i64")
}

async fn scalar_text(db: &DatabaseConnection, sql: String) -> Option<String> {
    let row = db
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .expect("探针读回 SQL 执行失败");
    match row {
        Some(r) => r
            .try_get_by_index::<Option<String>>(0)
            .expect("文本列应解码为 Option<String>"),
        None => None,
    }
}

/// 拒绝出参双向钉：400 + BUSINESS_ERROR + 真实文案回显用户自己提交值，
/// 且不得是脱敏常量、不得含表名/SQL/约束细节（先例：wave-i 同族判据）
fn assert_reject_envelope(v: &Value, status: StatusCode, code_in_msg: &str) {
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "重复编码/单号必须 400 拒绝：{v}"
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
        "文案须回显用户自己提交的编码/单号：{msg}"
    );
    for needle in [
        "warehouses",
        "suppliers",
        "outsourcing",
        "SQL",
        "INSERT",
        "23505",
        "unique",
        "UNIQUE",
        "constraint",
        "DATABASE_ERROR",
    ] {
        assert!(
            !msg.contains(needle),
            "message 不得含表名/SQL/约束细节「{needle}」：{msg}"
        );
    }
}

// =========================================================
// ① warehouse 人工编码：重复 400 外显 + 真实落库回读 + 零残留
// =========================================================

#[tokio::test]
async fn w7s_warehouse_manual_duplicate_code_400_displayable_zero_residue() {
    let (app, db) = build_app().await;
    let code = uniq_code("WH");
    let first = send(
        &app,
        "POST",
        "/warehouses",
        Some(json!({ "warehouse_code": code, "warehouse_name": format!("W7S一号仓{code}") })),
    )
    .await;
    assert_eq!(first.0, StatusCode::OK, "首次建仓库应 200：{}", first.1);
    let id = first.1["data"]["id"].as_i64().unwrap();

    // 编码逐字落库回读（非响应回声）
    let read = send(&app, "GET", &format!("/warehouses/{id}"), None).await;
    assert_eq!(read.0, StatusCode::OK);
    assert_eq!(
        read.1["data"]["warehouse_code"], code,
        "warehouse_code 应真实落库"
    );

    // 同码重复提交 → 400 可外显；第二行名称「重复仓B」不得回显
    let dup = send(
        &app,
        "POST",
        "/warehouses",
        Some(json!({ "warehouse_code": code, "warehouse_name": "重复仓B" })),
    )
    .await;
    assert_reject_envelope(&dup.1, dup.0, &code);
    assert!(
        !dup.1["message"].as_str().unwrap().contains("一号仓"),
        "既有行名称不得回显：{}",
        dup.1["message"]
    );

    // 零残留：被拒的第二次写入没有落任何行
    let cnt = scalar_i64(
        &db,
        format!("SELECT count(*) FROM warehouses WHERE warehouse_code = '{code}'"),
    )
    .await;
    assert_eq!(cnt, 1, "重复编码被拒后该码只允许存在首行，零残留");
}

// =========================================================
// ② warehouse 竞态兜底：真实命中一次（未提交旁路重复行挂起 INSERT →
//    预校验放行 → INSERT 撞 23505 → 降级业务拒绝、整事务回滚不留半行）
// =========================================================

#[tokio::test]
async fn w7s_warehouse_race_fallback_hits_with_displayable_reject() {
    let (app, db) = build_app().await;
    let code = uniq_code("WH");

    // 探针事务：在未提交状态下先占住同码（模拟"预校验通过后并发插入同码"，
    // 形态与 utils/number_generator.rs 文档注释描述的竞态窗口完全一致）
    let dup_txn = db.begin().await.unwrap();
    dup_txn
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            format!(
                "INSERT INTO warehouses (name, warehouse_code) VALUES ('W7S竞态探针', '{code}')"
            ),
        ))
        .await
        .expect("探针同码行写入失败");

    // 并发发起人工同码创建：其事务内预校验看不到未提交探针行 → 放行 →
    // INSERT 在 warehouse_code UNIQUE 索引上挂起等待探针事务裁决
    let app2 = app.clone();
    let body = json!({ "warehouse_code": code, "warehouse_name": "W7S竞态触发仓" });
    let task = tokio::spawn(async move { send(&app2, "POST", "/warehouses", Some(body)).await });

    // 观测挂起点：pg_stat_activity 中出现来自其他后端的 warehouses INSERT（判定
    // "预校验已放行、INSERT 已被旁路重复号挂起"），确认后才裁决探针事务
    let observed = async {
        for _ in 0..100 {
            let n = scalar_i64(
                &db,
                "SELECT count(*) FROM pg_stat_activity \
                 WHERE state = 'active' AND pid <> pg_backend_pid() \
                 AND query LIKE 'INSERT INTO \"warehouses\"%'"
                    .to_string(),
            )
            .await;
            if n > 0 {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        false
    }
    .await;
    assert!(
        observed,
        "前置失效：未观测到被旁路重复号挂起的 INSERT（竞态窗口未形成，不允许跳过本用例）"
    );
    dup_txn.commit().await.expect("探针事务提交失败");

    let (status, v) = task.await.unwrap();
    // 兜底分支被真实命中：与预校验同口径的 400 可外显拒绝（修复前此处是
    // From<DbErr> 拍平的 500 DATABASE_ERROR 裸抛）
    assert_reject_envelope(&v, status, &code);

    // 零残留：服务侧整事务回滚，该码只剩探针行一行（服务行未落半行）
    let cnt = scalar_i64(
        &db,
        format!("SELECT count(*) FROM warehouses WHERE warehouse_code = '{code}'"),
    )
    .await;
    assert_eq!(cnt, 1, "竞态撞 23505 后服务行必须零残留（单语句原子失败）");
}

// =========================================================
// ③ supplier：改名判重收敛唯一实现（他人占用名 400 外显 + 幂等自名放行 + 零残留）
// =========================================================

#[tokio::test]
async fn w7s_supplier_rename_duplicate_400_and_self_name_idempotent_200() {
    let (app, db) = build_app().await;
    let name_a = format!("W7S供应商甲{}", Utc::now().timestamp_nanos_opt().unwrap());
    let name_b = format!("W7S供应商乙{}", Utc::now().timestamp_nanos_opt().unwrap());

    let sup_a = send(
        &app,
        "POST",
        "/suppliers",
        Some(json!({ "supplier_name": name_a })),
    )
    .await;
    assert_eq!(sup_a.0, StatusCode::OK, "建供应商甲应 200：{}", sup_a.1);
    // 取号路径仍走 SUP 前缀（insert_with_no_retry 收敛后编码格式契约不变）
    assert!(
        sup_a.1["data"]["supplier_code"]
            .as_str()
            .unwrap_or_default()
            .starts_with("SUP"),
        "supplier_code 应以 SUP 前缀取号落库：{}",
        sup_a.1["data"]
    );

    let sup_b = send(
        &app,
        "POST",
        "/suppliers",
        Some(json!({ "supplier_name": name_b })),
    )
    .await;
    assert_eq!(sup_b.0, StatusCode::OK, "建供应商乙应 200：{}", sup_b.1);
    let id_b = sup_b.1["data"]["id"].as_i64().unwrap();

    // 改名撞甲的名称 → 400 可外显（修复前：判重缺失，静默落出重复名）
    let dup = send(
        &app,
        "PUT",
        &format!("/suppliers/{id_b}"),
        Some(json!({ "supplier_name": name_a })),
    )
    .await;
    assert_eq!(
        dup.0,
        StatusCode::BAD_REQUEST,
        "改名到他人已占用名称必须 400：{}",
        dup.1
    );
    assert_eq!(dup.1["code"], "BUSINESS_ERROR");
    let msg = dup.1["message"].as_str().unwrap_or_default();
    assert_ne!(msg, "业务处理失败", "拒绝原因必须外显：{msg}");
    assert!(
        msg.contains("已存在") && msg.contains(&name_a),
        "文案：{msg}"
    );

    // 零残留：被拒改名没有落库（直读 DB，绕开缓存层）
    let stored = scalar_text(
        &db,
        format!("SELECT supplier_name FROM suppliers WHERE id = {id_b}"),
    )
    .await;
    assert_eq!(
        stored.as_deref(),
        Some(name_b.as_str()),
        "被拒后名称必须原样"
    );

    // 与自身当前名相同的幂等 PUT 放行（防过度收紧打错既有行为）
    let same = send(
        &app,
        "PUT",
        &format!("/suppliers/{id_b}"),
        Some(json!({ "supplier_name": name_b })),
    )
    .await;
    assert_eq!(same.0, StatusCode::OK, "同值改名应幂等放行：{}", same.1);
}

// =========================================================
// ④ outsourcing：单号重复 + 缺前置 拒绝可外显（三表族）+ 零残留
// =========================================================

async fn seed_supplier_and_order(app: &Router) -> (i64, i64, String) {
    let sup = send(
        app,
        "POST",
        "/suppliers",
        Some(json!({ "supplier_name": format!("W7S加工厂{}", Utc::now().timestamp_nanos_opt().unwrap()) })),
    )
    .await;
    assert_eq!(sup.0, StatusCode::OK, "种子供应商应 200：{}", sup.1);
    let supplier_id = sup.1["data"]["id"].as_i64().unwrap();

    let order_no = uniq_code("OW");
    let order = send(
        app,
        "POST",
        "/outsourcing-orders",
        Some(json!({
            "order_no": order_no,
            "order_type": "dyeing",
            "supplier_id": supplier_id,
            "issue_date": "2026-01-15",
            "issue_quantity": "10",
            "material_cost": "100",
        })),
    )
    .await;
    assert_eq!(order.0, StatusCode::OK, "种子委外订单应 200：{}", order.1);
    let order_id = order.1["data"]["id"].as_i64().unwrap();
    (supplier_id, order_id, order_no)
}

#[tokio::test]
async fn w7s_outsourcing_order_duplicate_no_400_displayable_zero_residue() {
    let (app, db) = build_app().await;
    let (_sup, _order_id, order_no) = seed_supplier_and_order(&app).await;

    let dup_body = json!({
        "order_no": order_no,
        "order_type": "dyeing",
        "supplier_id": 1,
        "issue_date": "2026-01-16",
        "issue_quantity": "5",
        "material_cost": "50",
    });
    let dup = send(&app, "POST", "/outsourcing-orders", Some(dup_body)).await;
    assert_reject_envelope(&dup.1, dup.0, &order_no);

    let cnt = scalar_i64(
        &db,
        format!("SELECT count(*) FROM outsourcing_order WHERE order_no = '{order_no}'"),
    )
    .await;
    assert_eq!(cnt, 1, "重复单号被拒后零残留");
}

#[tokio::test]
async fn w7s_outsourcing_receipt_voucher_duplicate_and_missing_prereq_400_displayable() {
    let (app, db) = build_app().await;
    let (_sup, order_id, _order_no) = seed_supplier_and_order(&app).await;

    // 前置父行自种子：成品布一行（products 既有表，只种行不建 DDL）。
    // 必须补齐 model 非 Option 的三列 unit/status/product_type：收回单 create 路径
    // `validate_create_request`（services/outsourcing_ops/receipt.rs:193）以
    // `product::Entity::find_by_id(..).one(db)` 整行解码 product::Model，任一非 Option 列
    // 在库里为 NULL 即抛 DbErr::Type("Missing value for column 'unit'") → DATABASE_ERROR，
    // 首次建单应 200 的断言随即红。DDL 里这三列可空（unit 见 m0001:227，product_type、status
    // 为后续迁移后补列，均无 NOT NULL/DEFAULT），但整行解码需求 ≠ DDL NOT NULL：取值按写入侧
    // 词表对齐（status=master_data::ACTIVE 字面 'active'，product_type 取模型注释词表成品布，
    // unit 取面料域真实单位米），与 production_order_workflow_test.rs 同族夹具修法一致。
    let prod_code = uniq_code("PROD");
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!(
                "INSERT INTO products (code, name, unit, status, product_type) \
                 VALUES ('{prod_code}', 'W7S成品布探针', '米', 'active', '成品布') RETURNING id"
            ),
        ))
        .await
        .expect("成品种子行写入失败")
        .expect("RETURNING id 应返回一行");
    let product_id = row.try_get_by_index::<i32>(0).expect("id 应解码为 i32");

    // —— 收回单：单号重复 400 外显 + 零残留（修复前被吞成「业务处理失败」）
    let receipt_no = uniq_code("RC");
    let r_body = json!({
        "receipt_no": receipt_no,
        "outsourcing_order_id": order_id,
        "receipt_date": "2026-01-20",
        "product_id": product_id,
        "return_quantity": "5",
    });
    let r_first = send(&app, "POST", "/outsourcing-receipts", Some(r_body.clone())).await;
    assert_eq!(
        r_first.0,
        StatusCode::OK,
        "首次建收回单应 200：{}",
        r_first.1
    );
    let r_dup = send(&app, "POST", "/outsourcing-receipts", Some(r_body)).await;
    assert_reject_envelope(&r_dup.1, r_dup.0, &receipt_no);
    let r_cnt = scalar_i64(
        &db,
        format!("SELECT count(*) FROM outsourcing_receipt WHERE receipt_no = '{receipt_no}'"),
    )
    .await;
    assert_eq!(r_cnt, 1, "收回单重复单号被拒后零残留");

    // —— 凭证：单号重复 400 外显 + 零残留
    let voucher_no = uniq_code("VC");
    let v_body = json!({
        "voucher_no": voucher_no,
        "outsourcing_order_id": order_id,
        "voucher_type": "issue",
        "debit_account": "原材料",
        "credit_account": "应付账款",
        "amount": "100",
        "voucher_date": "2026-01-20",
    });
    let v_first = send(&app, "POST", "/outsourcing-vouchers", Some(v_body.clone())).await;
    assert_eq!(
        v_first.0,
        StatusCode::OK,
        "首次建委外凭证应 200：{}",
        v_first.1
    );
    let v_dup = send(&app, "POST", "/outsourcing-vouchers", Some(v_body)).await;
    assert_reject_envelope(&v_dup.1, v_dup.0, &voucher_no);
    let v_cnt = scalar_i64(
        &db,
        format!("SELECT count(*) FROM outsourcing_voucher WHERE voucher_no = '{voucher_no}'"),
    )
    .await;
    assert_eq!(v_cnt, 1, "凭证号重复被拒后零残留");

    // —— 缺前置：凭证挂到不存在的委外订单 → 拒绝原因可外显（非脱敏常量）
    let missing = send(
        &app,
        "POST",
        "/outsourcing-vouchers",
        Some(json!({
            "voucher_no": uniq_code("VC"),
            "outsourcing_order_id": 999_999_999i64,
            "voucher_type": "issue",
            "debit_account": "原材料",
            "credit_account": "应付账款",
            "amount": "1",
            "voucher_date": "2026-01-20",
        })),
    )
    .await;
    assert_eq!(
        missing.0,
        StatusCode::BAD_REQUEST,
        "缺前置必须 400 拒绝：{}",
        missing.1
    );
    assert_eq!(missing.1["code"], "BUSINESS_ERROR");
    let msg = missing.1["message"].as_str().unwrap_or_default();
    assert_ne!(msg, "业务处理失败", "缺前置拒绝原因必须外显：{msg}");
    assert!(msg.contains("不存在"), "文案：{msg}");
}

// =========================================================
// 源码扫描锁：本族「预校验/外显 + 23505 归类」范式不得回潮（纯源码、无 DB）
// =========================================================

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// 本族的禁词/必备文案在源码里**同时存在于说明注释**（"旧口径是脱敏 business，
/// 现升为 business_displayable"这类），按原文判会把注释当执行体（#4671 判责 B1①）。
/// 按行而非按字符扫描：被锁文件里的跨行 raw string/多行 format 参数使单行引号配平
/// 不可靠，宁少剥（行尾尾注释、块注释不动）不可错吃代码文本。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn w7s_source_scan_displayable_and_race_fallback_locked() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services");
    // (文件, 公开文案关键片段[去空白], 该片段是否要求 INSERT 兜底)
    let cases = [
        (
            "warehouse_service.rs",
            "仓库编码{}已存在，请更换编码后重试",
            true,
        ),
        (
            "outsourcing_ops/order.rs",
            "委外订单号{}已存在，请更换单号后重试",
            true,
        ),
        (
            "outsourcing_ops/receipt.rs",
            "收回单号{}已存在，请更换单号后重试",
            true,
        ),
        (
            "outsourcing_ops/voucher.rs",
            "凭证号{}已存在，请更换凭证号后重试",
            true,
        ),
    ];
    for (rel, key, need_fallback) in cases {
        let path = manifest.join(rel);
        let src =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
        // 判据文本 = 剥掉整行注释后再去空白：
        // ① needle 里的 `{}` 与中文标点两侧本就没有空白，而源码里的文案字面量是
        //    `"仓库编码 {} 已存在…"`（`{}` 两侧有空格，rustfmt/可读性所需），
        //    原写法把去空白后的 needle 拿去 `src.contains` 比对**原文**，对 4 条 case
        //    全不成立（#4671 判责 B1②：单行字面量 needle 假失败，改一处会连报三处）；
        // ② 说明性注释（"旧口径是脱敏 business，现升为 business_displayable"）不是
        //    执行体，禁项/必备项都只看代码文本。
        let flat: String = code_only(&src)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let key_flat: String = key.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            flat.contains(&key_flat),
            "{rel} 缺少公开规则文案「{key}」（重复单号/编码拒绝必须可外显）"
        );
        let displayable = format!("AppError::business_displayable(format!(\"{key_flat}");
        assert!(
            flat.contains(&displayable),
            "{rel} 判重拒绝「{key}」必须走 business_displayable（可外显业务族，先例 #165）"
        );
        let masked = format!("AppError::business(format!(\"{key_flat}");
        assert!(
            !flat.contains(&masked),
            "{rel} 判重文案「{key}」回潮为脱敏的 AppError::business：真因将被出参吞掉"
        );
        if need_fallback {
            assert!(
                flat.contains("SqlErr::UniqueConstraintViolation"),
                "{rel} 缺少 INSERT 撞唯一约束(23505)的竞态兜底分类：并发同码将以 500 DATABASE_ERROR 裸抛"
            );
        }
    }

    // outsourcing 收回单/凭证：旧的 map_err 自造 database(format!({e})) 吞 23505 形态不得回潮
    for (rel, dead_shape) in [
        (
            "outsourcing_ops/receipt.rs",
            "map_err(|e|AppError::database(format!(\"委外收回单创建失败",
        ),
        (
            "outsourcing_ops/voucher.rs",
            "map_err(|e|AppError::database(format!(\"委外凭证创建失败",
        ),
    ] {
        let src = std::fs::read_to_string(manifest.join(rel)).unwrap();
        let flat: String = code_only(&src)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        assert!(
            !flat.contains(dead_shape),
            "{rel} 回潮 map_err 自造 DatabaseError 形态：会把 23505 与其他 DbErr 一并拍平成裸 500"
        );
    }

    // supplier：取号+判重+INSERT 唯一实现收口；旧的「generate → 直插 ?」旁路不得回潮
    let sup = std::fs::read_to_string(manifest.join("supplier_service.rs")).unwrap();
    let sup_flat: String = code_only(&sup)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(
        sup_flat.contains("DocumentNumberGenerator::insert_with_no_retry("),
        "supplier_service create 必须走 insert_with_no_retry 唯一取号+撞号重试实现"
    );
    assert!(
        !sup_flat.contains("generate_supplier_code"),
        "generate_supplier_code（generate→直插两段式）已废弃，不得回潮：竞态撞号将以裸 500 上抛"
    );
    assert!(
        sup_flat.contains("self.check_supplier_name_unique(new_name).await?"),
        "update_supplier 改名判重必须复用 check_supplier_name_unique 唯一实现（禁止第二套规则）"
    );
    assert!(
        sup_flat.contains("供应商名称'{}'已存在"),
        "供应商名称重复拒绝必须保留可外显公开规则文案（business_displayable 族）"
    );

    // warehouse：人工码分支的旧裸 `?` 直插形态不得回潮
    let wh = std::fs::read_to_string(manifest.join("warehouse_service.rs")).unwrap();
    let wh_flat: String = code_only(&wh)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(
        !wh_flat.contains("build_warehouse_active_model(code,&req,manager_id).insert(&txn).await?"),
        "warehouse 人工码分支回潮裸 `?` 直插：撞 UNIQUE(23505) 将再次成为 500 DATABASE_ERROR"
    );
}

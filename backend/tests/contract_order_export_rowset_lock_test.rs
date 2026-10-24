//! 契约锁：订单导出全量行集完整性（销售腿与采购腿双向钉死）
//!
//! 本文件锁三件事，缺一不算导出取数腿契约成立。
//! 一，取数腿不经分页入口。导出取数不得复用列表分页函数，必须走导出专用的
//! list_all_orders_for_export 全量函数，且该函数体与列表共用同一套筛选条件
//! 构造器 build_orders_query，保证导出与列表的口径同源。把取数退回以
//! page_size 当 LIMIT 的分页形态时，页外行会静默丢失且不留任何痕，
//! 本锁必须判红，这就是预设的证伪对照。
//! 二，完整性等式。交付行数、审计留痕里的服务端全量总数、独立数据库计数
//! 三者必须相等。行为用例在两侧各播多行订单，走真实 handler 导出后回读
//! 审计快照，断 exported_rows、total 与数据库计数三值全等，且 export_complete
//! 为真、审计描述与水印携带同一组真实数字。
//! 三，防回潮。行数阈值常量与超限分支不得再现（它恒不可达，属假防护），
//! 审计快照必须携带真实键，留痕路径必须与真实挂载逐字一致（采购侧挂载
//! 前缀是单数），销售侧水印与采购同型（条款页已公示导出带条数水印）。
//!
//! 通道：静态源码锁不连库，用 include_str 读真实源文件逐锚点断。行为锁经
//! test_common 夹具 setup_test_db 连已迁移 PostgreSQL 真跑（缺库由夹具直接
//! 失败，不降级为假断言），走两个真实 handler export_orders，审计落库走
//! 生产同源的 AuditLogService 异步通道。两侧导出与两段留痕回读刻意合并在
//! 同一个用例行内：审计全局消费者绑在首次初始化它的运行时上，拆成并列用例
//! 会把消费者悬在已退出的运行时里，回读不可靠。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{purchase_order_handler, sales_order_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::{purchase_order as po_status, sales_order as so_status};
use bingxi_backend::models::{audit_log, customer, purchase_order, sales_order, supplier, user};
use bingxi_backend::search::{ElasticClient, SearchClient};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use warehouse_model::ActiveModel as WarehouseActiveModel;

use bingxi_backend::models::warehouse as warehouse_model;

const OPERATOR_ID: i32 = 9631;
const SALES_EXPORT_PATH: &str = "/api/v1/erp/sales/orders/export";
const PURCHASE_EXPORT_PATH: &str = "/api/v1/erp/purchase/orders/export";
const PURCHASE_EXPORT_PATH_WRONG_PLURAL: &str = "/api/v1/erp/purchases/orders/export";
const SEED_SALES_ROWS: u64 = 3;
const SEED_PURCHASE_ROWS: u64 = 2;

fn now() -> DateTime<Utc> {
    Utc::now()
}

// ---------------------------------------------------------------------------
// 静态锁一：导出取数腿不经分页入口且返回形状带服务端全量总数
// ---------------------------------------------------------------------------

/// 取全函数定义段：从全量取数函数签名到完整性判据函数签名之间。
/// 锚点缺失直接判红，因为那意味着导出专用腿整体不在位。
fn full_fetch_segment<'a>(source: &'a str, file_name: &str) -> &'a str {
    let start = source
        .find("pub async fn list_all_orders_for_export")
        .unwrap_or_else(|| panic!("导出专用全量取数函数缺失，取数腿退回分页形态：{file_name}"));
    let end = source[start..]
        .find("pub fn export_rows_complete")
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("导出行集完整性判据函数缺失：{file_name}"));
    &source[start..end]
}

#[test]
fn order_export_leg_never_enters_paging_entry() {
    let so_export = include_str!("../src/services/so/delivery_ops/export.rs").replace('\r', "");
    let so_query = include_str!("../src/services/so/order_query.rs").replace('\r', "");
    let po_query = include_str!("../src/services/po/order_ops/query.rs").replace('\r', "");

    // 1. 导出取数两个文件内不得出现对分页入口的调用（分页复用等于以页宽裁行）。
    //    列表函数自身的定义与出参键不在本文件，此处出现的 list_orders 调用一律是取数腿走偏。
    for (file_name, source) in [
        ("so/delivery_ops/export.rs", &so_export),
        ("po/order_ops/query.rs", &po_query),
    ] {
        assert_eq!(
            source.matches("list_orders(").count(),
            0,
            "导出取数腿禁止经分页入口取行，回潮即静默丢行：{file_name}"
        );
    }

    // 2. 全量取数与完整性判据两函数必须在位，且导出表格函数返回三元形状
    //    （表头、行集、服务端全量总数），形状退回二元等于把总数丢弃。
    for (file_name, source) in [
        ("so/order_query.rs", &so_query),
        ("po/order_ops/query.rs", &po_query),
    ] {
        assert!(
            source.contains("pub async fn list_all_orders_for_export"),
            "导出腿缺全量取数函数定义：{file_name}"
        );
        assert!(
            source.contains("pub fn export_rows_complete"),
            "导出腿缺完整性判据单点定义：{file_name}"
        );
    }
    for (file_name, source) in [
        ("so/delivery_ops/export.rs", &so_export),
        ("po/order_ops/query.rs", &po_query),
    ] {
        assert!(
            source.contains("Vec<String>, Vec<Vec<String>>, u64), AppError>"),
            "导出表格函数返回形状必须携带服务端全量总数：{file_name}"
        );
    }
    assert!(
        so_export.contains("list_all_orders_for_export("),
        "销售导出腿必须调用全量取数函数"
    );

    // 3. 口径同源：全量函数体内必须用列表共用的筛选条件构造器；
    //    销售全量腿只消费单头字段，不得顺带装配订单明细（白付批量查询）。
    let so_segment = full_fetch_segment(&so_query, "so/order_query.rs");
    assert!(
        so_segment.contains("build_orders_query("),
        "销售全量腿必须与列表共用筛选条件构造器"
    );
    assert_eq!(
        so_segment.matches("assemble_order_details(").count(),
        0,
        "销售全量腿不得装配明细"
    );
    let po_segment = full_fetch_segment(&po_query, "po/order_ops/query.rs");
    assert!(
        po_segment.contains("build_orders_query("),
        "采购全量腿必须与列表共用筛选条件构造器"
    );

    // 4. 完整性判据是单点定义：行集与总数全等才算完整，判据里不得掺行数上限。
    let so_complete = &so_query[so_query
        .find("pub fn export_rows_complete")
        .unwrap_or_else(|| panic!("销售完整性判据缺失"))..];
    let body_open = so_complete.find('{').expect("判据体起点");
    let body_close = so_complete[body_open..].find('}').expect("判据体终点");
    let so_complete_body = &so_complete[body_open..body_open + body_close + 1];
    assert!(
        so_complete_body.contains("returned_rows as u64 == server_total"),
        "完整性判据必须是行集与全量总数的全等式：{so_complete_body}"
    );
}

// ---------------------------------------------------------------------------
// 静态锁二：handler 不设行数阀、留痕携真实值、路径与挂载逐字一致
// ---------------------------------------------------------------------------

#[test]
fn order_export_handlers_keep_no_row_gate_and_truthful_traces() {
    let so = include_str!("../src/handlers/sales_order_handler.rs").replace('\r', "");
    let po = include_str!("../src/handlers/purchase_order_handler.rs").replace('\r', "");

    // 1. 恒不可达的行数阈值常量与超限分支撤除后不得复现。
    assert_eq!(
        so.matches("MAX_SALES_ORDER_EXPORT_ROWS").count(),
        0,
        "销售导出再现行数阈值常量（恒不可达的假防护）"
    );
    assert_eq!(
        po.matches("MAX_PURCHASE_ORDER_EXPORT_ROWS").count(),
        0,
        "采购导出再现行数阈值常量（恒不可达的假防护）"
    );

    // 2. 两个 handler 必须解构三元返回并把服务端全量总数送进快照的 total 键，
    //    同时携带 exported_rows 与 export_complete 两键，交付行数与全量总数
    //    在留痕里是两个独立真值，不得由一个冒充。
    for (file_name, source) in [
        ("sales_order_handler.rs", &so),
        ("purchase_order_handler.rs", &po),
    ] {
        assert!(
            source.contains("let (headers, rows, server_total)"),
            "handler 必须消费服务端全量总数：{file_name}"
        );
        assert!(
            source.contains("\"total\": server_total"),
            "留痕快照 total 必须是服务端全量总数而非交付行数：{file_name}"
        );
        assert!(
            source.contains("\"exported_rows\"") && source.contains("\"export_complete\""),
            "留痕快照缺交付行数或完整性键：{file_name}"
        );

        // 3. 审计描述与水印两处各携带同一组真实数字，锚点串逐字钉死。
        assert_eq!(
            source.matches("交付 {} 条，服务端全量 {} 条，{}").count(),
            2,
            "留痕描述与水印必须各出现一次真实数字格式串：{file_name}"
        );

        // 4. 两侧同型注入条数水印（条款页已公示导出带操作员时间与条数水印）。
        assert!(
            source.contains("build_xlsx_response_with_watermark("),
            "handler 必须以带水印形态交付 xlsx：{file_name}"
        );
    }

    // 5. 留痕路径与真实挂载逐字一致，挂载前缀是单数，复数路径按路径检索恒查不到。
    assert!(
        so.contains(format!("\"{SALES_EXPORT_PATH}\"").as_str()),
        "销售留痕路径必须与挂载逐字一致"
    );
    assert!(
        po.contains(format!("\"{PURCHASE_EXPORT_PATH}\"").as_str()),
        "采购留痕路径必须与挂载逐字一致（前缀单数）"
    );
    assert_eq!(
        po.matches(format!("\"{PURCHASE_EXPORT_PATH_WRONG_PLURAL}\"").as_str())
            .count(),
        0,
        "采购留痕路径回潮复数形态（审计面与访问面不同源）"
    );
}

// ---------------------------------------------------------------------------
// 行为锁（真库）：三值全等与单数挂载真实可检索
// ---------------------------------------------------------------------------

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("order_export_rowset_lock".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("订单导出行集锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
}

async fn seed_customer(db: &Arc<DatabaseConnection>) -> i32 {
    let c = customer::ActiveModel {
        customer_code: Set("CUS-EXPROW".to_string()),
        customer_name: Set("订单导出行集锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户插入失败: {e}"));
    c.id
}

/// suppliers 属封存参照表不被夹具清空，显式主键会撞历史行，自增插入后回读真实主键。
async fn seed_supplier(db: &Arc<DatabaseConnection>) -> i32 {
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-EXPROW-{}", now().timestamp_micros())),
        supplier_name: Set("订单导出行集锁供应商".to_string()),
        supplier_short_name: Set("行供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00001X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000001".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    s.id
}

async fn seed_warehouse(db: &Arc<DatabaseConnection>) -> i32 {
    let w = WarehouseActiveModel {
        warehouse_code: Set("WH-EXPROW".to_string()),
        name: Set("订单导出行集锁仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子仓库插入失败: {e}"));
    w.id
}

async fn seed_sales_order(db: &Arc<DatabaseConnection>, seq: u64, status: &str, customer_id: i32) {
    let total = Decimal::from(seq);
    sales_order::ActiveModel {
        order_no: Set(format!("SO-EXPROW-{seq}")),
        customer_id: Set(customer_id),
        order_date: Set(now()),
        required_date: Set(Some(now())),
        status: Set(status.to_string()),
        subtotal: Set(total),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(total),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(total),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子销售订单 {seq} 插入失败: {e}"));
}

async fn seed_purchase_order(
    db: &Arc<DatabaseConnection>,
    seq: u64,
    status: &str,
    supplier_id: i32,
    warehouse_id: i32,
) {
    purchase_order::ActiveModel {
        order_no: Set(format!("PO-EXPROW-{seq}")),
        supplier_id: Set(supplier_id),
        order_date: Set(now().date_naive()),
        warehouse_id: Set(warehouse_id),
        department_id: Set(1),
        purchaser_id: Set(OPERATOR_ID),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(status.to_string()),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子采购订单 {seq} 插入失败: {e}"));
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

fn build_app(db: &Arc<DatabaseConnection>) -> Router {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let state = AppState {
        db: db.clone(),
        search_client,
        ..Default::default()
    };
    let auth = AuthContext {
        user_id: OPERATOR_ID,
        username: "order_export_rowset_lock".to_string(),
        role_id: None,
        department_id: Some(1),
        // 行级范围取 all：本锁只钉导出行数口径，行级门另有专项锁
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    Router::new()
        .route(
            "/sales/orders/export",
            get(sales_order_handler::export_orders),
        )
        .route(
            "/purchase/orders/export",
            get(purchase_order_handler::export_orders),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_bytes(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .expect("请求构造失败"),
        )
        .await
        .expect("handler 调用失败");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .expect("读取响应体失败");
    (status, bytes.to_vec())
}

/// 审计走生产同源的异步通道，回读按 100 毫秒步进轮询至多 10 秒；
/// 同一路径出现多行即夹具异常，直接判红不取首行掩盖。
async fn wait_export_audit(db: &Arc<DatabaseConnection>, path: &str) -> audit_log::Model {
    for tick in 0..100 {
        // 导出留痕走异步总线：先把截至此刻已入队的事件排空再查表。
        // 不排空就直接查会在真实 CI 上稳定假红——消费者落库要一次连接，
        // 而用例的连接池在断言之后的收尾里就被关掉，消费者拿到 Disconnected，
        // 留痕永远进不了表，轮询到超时才炸，看着像"导出没留痕"其实是没等总线。
        let drained =
            bingxi_backend::services::audit_log_service::flush(Duration::from_millis(200)).await;
        assert_eq!(
            drained.failed, 0,
            "审计总线第 {tick} 轮排空出现写失败 {} 条，留痕丢失不是路径不同源",
            drained.failed
        );
        assert_eq!(
            drained.panicked, 0,
            "审计总线第 {tick} 轮排空出现消费者 panic {} 次，按缺陷处理",
            drained.panicked
        );
        let rows = audit_log::Entity::find()
            .filter(audit_log::Column::RequestPath.eq(path))
            .all(db.as_ref())
            .await
            .unwrap_or_else(|e| panic!("审计留痕回读失败 {path}: {e}"));
        match rows.len() {
            0 => tokio::time::sleep(Duration::from_millis(100)).await,
            1 => return rows.into_iter().next().expect("单行审计取出"),
            n => panic!("导出路径 {path} 应恰有一条留痕，实得 {n} 条"),
        }
    }
    panic!(
        "10 秒内未读到 {path} 的导出留痕（已逐轮排空审计总线仍无行：留痕路径与真实挂载不同源，或该导出腿未接留痕）"
    );
}

async fn count_table(db: &Arc<DatabaseConnection>, table: &str) -> u64 {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT COUNT(*) FROM {table}"),
        ))
        .await
        .unwrap_or_else(|e| panic!("{table} 计数失败: {e}"))
        .expect("COUNT 聚合必须返回一行");
    let n: i64 = row
        .try_get_by_index::<i64>(0)
        .unwrap_or_else(|e| panic!("{table} 计数解码失败: {e}"));
    n.try_into().expect("计数非负")
}

fn snapshot_of(log: &audit_log::Model) -> Value {
    log.after_snapshot
        .as_ref()
        .map(|v| v.0.clone())
        .unwrap_or_else(|| panic!("导出留痕必须携带 after_snapshot: {log:?}"))
}

/// 断一条导出腿的三值全等：交付行数、留痕总数、数据库独立计数，
/// 外加完整性键为真与描述数字。expected 是种子行数，三值任何一值偏离都判红。
fn assert_rowset_equality(label: &str, log: &audit_log::Model, sql_count: u64, expected: u64) {
    let snap = snapshot_of(log);
    assert_eq!(
        snap["format"].as_str(),
        Some("xlsx"),
        "{label} 留痕格式键必须是 xlsx: {snap}"
    );
    let exported_rows = snap["exported_rows"].as_u64().unwrap_or_else(|| {
        panic!("{label} 留痕缺 exported_rows 键（退回以行数冒充全量的旧形态）: {snap}")
    });
    let snapshot_total = snap["total"]
        .as_u64()
        .unwrap_or_else(|| panic!("{label} 留痕缺 total 键: {snap}"));
    assert!(
        snap["export_complete"].as_bool() == Some(true),
        "{label} 全量一致时 export_complete 必须为真: {snap}"
    );

    assert_eq!(exported_rows, expected, "{label} 交付行数应等于种子行数");
    assert_eq!(
        exported_rows, snapshot_total,
        "{label} 交付行数与留痕总数必须相等（完整性等式第一环）"
    );
    assert_eq!(
        snapshot_total, sql_count,
        "{label} 留痕总数与数据库独立计数必须相等（完整性等式第二环）"
    );

    let description = log
        .description
        .as_deref()
        .unwrap_or_else(|| panic!("{label} 留痕描述缺失"));
    let truth_text = format!("交付 {expected} 条，服务端全量 {expected} 条，完整");
    assert!(
        description.contains(&truth_text),
        "{label} 留痕描述必须携带交付数与全量数的真实值: {description}"
    );
    let resource_name = log
        .resource_name
        .as_deref()
        .unwrap_or_else(|| panic!("{label} 留痕资源名缺失"));
    assert!(
        resource_name.ends_with(".xlsx"),
        "{label} 交付物应为 xlsx 文件: {resource_name}"
    );
}

#[tokio::test]
async fn sales_and_purchase_export_rowset_equals_server_total_equals_sql_count() {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_operator(&db).await;
    let customer_id = seed_customer(&db).await;
    let supplier_id = seed_supplier(&db).await;
    let warehouse_id = seed_warehouse(&db).await;
    seed_sales_order(&db, 1, so_status::DRAFT, customer_id).await;
    seed_sales_order(&db, 2, so_status::PENDING, customer_id).await;
    seed_sales_order(&db, 3, so_status::APPROVED, customer_id).await;
    seed_purchase_order(&db, 1, po_status::DRAFT, supplier_id, warehouse_id).await;
    seed_purchase_order(&db, 2, po_status::SUBMITTED, supplier_id, warehouse_id).await;

    let app = build_app(&db);

    // 销售腿：交付必须是 xlsx 字节流，三值全等由留痕与独立计数互证。
    let (status, body) = get_bytes(&app, "/sales/orders/export").await;
    assert_eq!(status, StatusCode::OK, "销售订单导出应 200");
    assert!(
        body.starts_with(&[0x50, 0x4b]),
        "销售导出响应体必须是 xlsx 的 zip 魔数起始"
    );
    let sales_audit = wait_export_audit(&db, SALES_EXPORT_PATH).await;
    let sales_count = count_table(&db, "sales_orders").await;
    assert_rowset_equality("销售", &sales_audit, sales_count, SEED_SALES_ROWS);

    // 采购腿：同上，且留痕必须以单数挂载路径可检索，复数路径必须零命中。
    let (status, body) = get_bytes(&app, "/purchase/orders/export").await;
    assert_eq!(status, StatusCode::OK, "采购订单导出应 200");
    assert!(
        body.starts_with(&[0x50, 0x4b]),
        "采购导出响应体必须是 xlsx 的 zip 魔数起始"
    );
    let purchase_audit = wait_export_audit(&db, PURCHASE_EXPORT_PATH).await;
    let purchase_count = count_table(&db, "purchase_orders").await;
    assert_rowset_equality("采购", &purchase_audit, purchase_count, SEED_PURCHASE_ROWS);

    let strays = audit_log::Entity::find()
        .filter(audit_log::Column::RequestPath.eq(PURCHASE_EXPORT_PATH_WRONG_PLURAL))
        .all(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("复数路径留痕检索失败: {e}"));
    assert_eq!(
        strays.len(),
        0,
        "复数挂载路径必须零命中（留痕与访问面同源，否则按路径检索恒查不到）"
    );
}

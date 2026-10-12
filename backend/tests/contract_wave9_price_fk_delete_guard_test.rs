//! 价目 FK 删除守卫契约锁（真 PG，无 #[ignore]，无 sqlite 回退）
//!
//! 功能：钉死产品删除路径的「价目引用预检 + 23503 兜底映射」与供应商删除
//! 路径的「采购价目点名」两处收紧形态。证据：
//! - migration::domain::price_fk 给 sales_prices.product_id /
//!   purchase_prices.product_id 施加了指向 products.id 的 NO ACTION 外键，
//!   修复前删除被价目引用的产品会撞 SQLSTATE 23503，经 utils::error::From<DbErr>
//!   拍平成裸 500 DATABASE_ERROR（脱敏固定文案，用户不知道被谁引用）；
//! - 正解通道照 warehouse_service::delete 先例：预检与删除同事务、
//!   父行 lock_exclusive 后统计，命中即 AppError::business_displayable
//!   （HTTP 400 / BUSINESS_ERROR，可外显业务族），绝不 500；
//! - 兜底映射 ProductService::map_product_fk_error 形状照
//!   supplier_service::map_supplier_fk_error / warehouse_service::map_warehouse_fk_error；
//!   supplier_service::find_supplier_reference 补 purchase_prices 分支后，
//!   被供应商价目引用的供应商拒绝文案可点名「采购价目」。
//!
//! 本锁钉三件事：
//! ① 产品被 sales_prices / purchase_prices 引用时删除被拒：判 400 +
//!   信封机器码 BUSINESS_ERROR + 落库后产品行与价目行都仍在
//!   （拒绝不级联删价目、不软删价目、不动产品）；
//! ② 无引用时删除成功（产品行消失 + 审计行真实落库）；
//! ③ 兜底映射判据：以真库驱动上报的 SQLSTATE 23503 DbErr 为输入，
//!   映射产出 400/BUSINESS_ERROR 而非 500/DATABASE_ERROR。竞态窗口
//!   （预检与 DELETE 之间并发新建价目行）在单连接测试中不可控，
//!   直测映射函数本身是竞态不可控时的确定性做法；另配非 FK 错误
//!   （真 23505）腿钉住「非 FK 故障保留 500，不吞不改道」。
//!
//! 调用方：CI 集成阶段 cargo test（真库夹具 test_common::setup_test_db，
//! 已迁移 PostgreSQL，缺 TEST_DATABASE_URL 直接 panic，禁静默回退 sqlite）。
//! 断言红线：只判 HTTP 状态码与信封机器码（含错误枚举族归属），
//! 不判拒绝文案原文（脱敏红线，同 contract_wave8_price_ref_existence_test 口径）。

mod test_common;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use bingxi_backend::models::product;
use bingxi_backend::models::status::sales::price_approval;
use bingxi_backend::models::{audit_log, purchase_price, sales_price, supplier};
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::product_service::ProductService;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::err_msg;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, SqlErr,
};
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9700;
const PRODUCT_SALES_REF_ID: i32 = 9701;
const PRODUCT_PURCHASE_REF_ID: i32 = 9702;
const PRODUCT_FREE_ID: i32 = 9703;
/// 不可能存在的产品 ID：products 每例被夹具 TRUNCATE 后从 1 起自增，
/// 该值远大于任何夹具种子 ⇒ 「引用不存在」判定与"库恰好为空"无关，
/// 用于从真库驱动取回原生 SQLSTATE 23503 DbErr（兜底映射判据的输入腿）。
const MISSING_PRODUCT_ID: i32 = 2_000_000_001;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn eff_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 1).expect("夹具日期常量")
}

fn est_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")
}

async fn seed_product(db: &DatabaseConnection, id: i32, code: &str) {
    product::ActiveModel {
        id: Set(id),
        name: Set(format!("价目FK守卫产品 {id}")),
        code: Set(code.to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子产品 {id} 插入失败: {e}"));
}

/// 种子销售价目行：status 用价格域权威词表常量（models::status::sales::price_approval），
/// customer_id 置 NULL 走标准价语义（FK 天然允许 NULL，不引入 customers 引用前置）。
async fn seed_sales_price(db: &DatabaseConnection, product_id: i32) {
    sales_price::ActiveModel {
        product_id: Set(product_id),
        customer_id: Set(None),
        price: Set(Decimal::new(10000, 2)),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(Decimal::ONE),
        price_type: Set("standard".to_string()),
        effective_date: Set(eff_date()),
        status: Set(price_approval::PENDING.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子销售价目行插入失败(product_id={product_id}): {e}"));
}

/// 供应商 ID：suppliers 属迁移种子参照表（夹具 TRUNCATE 白名单外、不被清空），
/// 优先取已有种子行，避免自插撞 suppliers.supplier_code UNIQUE 的跨运行残留；
/// 极端空表时自插一条最小合法行（NOT NULL 列清单照 models/supplier.rs 逐列核对）。
async fn ensure_supplier(db: &DatabaseConnection) -> i32 {
    if let Some(s) = supplier::Entity::find()
        .one(db)
        .await
        .expect("读取种子供应商失败")
    {
        return s.id;
    }
    let inserted = supplier::ActiveModel {
        supplier_code: Set("SUP-FK-GUARD-0001".to_string()),
        supplier_name: Set("价目FK守卫供应商".to_string()),
        supplier_short_name: Set("守供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(est_date()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("供应商价目守卫自插种子失败");
    inserted.id
}

/// 种子采购价目行：purchase_prices.supplier_id NOT NULL 且已被
/// migration::domain::price_fk 施加 FK ⇒ 必须引用真实供应商行。
async fn seed_purchase_price(db: &DatabaseConnection, product_id: i32) {
    let supplier_id = ensure_supplier(db).await;
    purchase_price::ActiveModel {
        product_id: Set(product_id),
        supplier_id: Set(supplier_id),
        price: Set(Decimal::new(8000, 2)),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(Decimal::ONE),
        price_type: Set("standard".to_string()),
        effective_date: Set(eff_date()),
        status: Set(price_approval::PENDING.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子采购价目行插入失败(product_id={product_id}): {e}"));
}

fn product_service(db: &DatabaseConnection) -> ProductService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    ProductService::new(Arc::new(db.clone()), search_client)
}

/// 拒绝形态完整判据：business_displayable 族（可外显通道，非脱敏 business）
/// + 机器码 BUSINESS_ERROR + HTTP 400，并显式排除 500/DATABASE_ERROR 回潮。
/// 只判族归属与码，不判文案原文（脱敏红线）。
fn expect_business_400(err: &AppError) {
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "价目引用拒绝必须走 business_displayable（可外显业务族），实际: {err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR", "实际: {err:?}");
    let resp = err.to_response();
    assert_eq!(
        resp.code, "BUSINESS_ERROR",
        "出参信封机器码必须 BUSINESS_ERROR，实际: {resp:?}"
    );
    let http = err.clone().into_response();
    assert_eq!(
        http.status(),
        StatusCode::BAD_REQUEST,
        "价目引用拒绝必须 400，禁止退化为 500"
    );
    assert_ne!(http.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_ne!(err.error_code(), "DATABASE_ERROR", "禁止裸 FK 500 形态回潮");
}

// ---------------------------------------------------------------------------
// ①-a 产品被销售价目引用 → 删除被拒（400/BUSINESS_ERROR），
//     落库后产品行仍在、价目行未被级联删除/软删
// ---------------------------------------------------------------------------
#[tokio::test]
async fn delete_product_referenced_by_sales_price_is_rejected_and_rows_survive() {
    let db = setup_test_db().await;
    seed_product(&db, PRODUCT_SALES_REF_ID, "PRD-FK-SALES").await;
    seed_sales_price(&db, PRODUCT_SALES_REF_ID).await;

    let svc = product_service(&db);
    let err = svc
        .delete_product(PRODUCT_SALES_REF_ID, OPERATOR_ID)
        .await
        .expect_err("被销售价目引用的产品不可删除（修复前此处为 FK 裸 500 DATABASE_ERROR）");
    expect_business_400(&err);

    assert!(
        product::Entity::find_by_id(PRODUCT_SALES_REF_ID)
            .one(&db)
            .await
            .expect("回读产品失败")
            .is_some(),
        "拒绝路径不得删掉产品行"
    );
    let price_rows = sales_price::Entity::find()
        .filter(sales_price::Column::ProductId.eq(PRODUCT_SALES_REF_ID))
        .count(&db)
        .await
        .expect("统计销售价目行失败");
    assert_eq!(price_rows, 1, "拒绝路径不得级联删除/软删价目行");
}

// ---------------------------------------------------------------------------
// ①-b 产品被采购价目引用 → 同上（purchase_prices 腿，FK 同为 price_fk 所加）
// ---------------------------------------------------------------------------
#[tokio::test]
async fn delete_product_referenced_by_purchase_price_is_rejected_and_rows_survive() {
    let db = setup_test_db().await;
    seed_product(&db, PRODUCT_PURCHASE_REF_ID, "PRD-FK-PURCHASE").await;
    seed_purchase_price(&db, PRODUCT_PURCHASE_REF_ID).await;

    let svc = product_service(&db);
    let err = svc
        .delete_product(PRODUCT_PURCHASE_REF_ID, OPERATOR_ID)
        .await
        .expect_err("被采购价目引用的产品不可删除（修复前此处为 FK 裸 500 DATABASE_ERROR）");
    expect_business_400(&err);

    assert!(
        product::Entity::find_by_id(PRODUCT_PURCHASE_REF_ID)
            .one(&db)
            .await
            .expect("回读产品失败")
            .is_some(),
        "拒绝路径不得删掉产品行"
    );
    let price_rows = purchase_price::Entity::find()
        .filter(purchase_price::Column::ProductId.eq(PRODUCT_PURCHASE_REF_ID))
        .count(&db)
        .await
        .expect("统计采购价目行失败");
    assert_eq!(price_rows, 1, "拒绝路径不得级联删除/软删价目行");
}

// ---------------------------------------------------------------------------
// ② 无价目引用的产品删除成功：产品行消失 + DELETE 审计行真实落库
// ---------------------------------------------------------------------------
#[tokio::test]
async fn delete_unreferenced_product_succeeds_and_writes_audit() {
    let db = setup_test_db().await;
    seed_product(&db, PRODUCT_FREE_ID, "PRD-FK-FREE").await;

    let svc = product_service(&db);
    svc.delete_product(PRODUCT_FREE_ID, OPERATOR_ID)
        .await
        .expect("无价目引用必须删除成功（预检不得误杀）");

    assert!(
        product::Entity::find_by_id(PRODUCT_FREE_ID)
            .one(&db)
            .await
            .expect("回读产品失败")
            .is_none(),
        "产品已硬删除（products 为 FK 父表，无残留引用）"
    );
    let audits = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq("product"))
        .filter(audit_log::Column::ResourceId.eq(PRODUCT_FREE_ID.to_string()))
        .all(&db)
        .await
        .expect("查询审计日志失败");
    assert_eq!(audits.len(), 1, "删除成功必须落且只落一条审计");
    assert_eq!(audits[0].action, "DELETE");
    assert_eq!(
        audits[0].user_id,
        Some(OPERATOR_ID),
        "审计操作人须为真实 user_id"
    );
}

// ---------------------------------------------------------------------------
// ③-a 兜底映射判据（真库取回原生 23503 DbErr 后直测映射）：
//     SQLSTATE 23503 FK 违例经 From<DbErr> 归类为 DatabaseError(DB_RELATION)
//     （修复前即在此形态裸落 500），再经 map_product_fk_error 必须产出
//     400/BUSINESS_ERROR/business_displayable 族。
//     说明：预检与 DELETE 之间的并发窗口在单连接测试里不可控，
//     以「insert 引用不存在 product_id 的价目行」从驱动取回同一 SQLSTATE
//     23503 的真实 DbErr，直测映射函数本身，是竞态不可控时的确定性做法。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn map_product_fk_error_on_real_23503_db_err_is_400_not_500() {
    let db = setup_test_db().await;

    let db_err = sales_price::ActiveModel {
        product_id: Set(MISSING_PRODUCT_ID),
        customer_id: Set(None),
        price: Set(Decimal::new(10000, 2)),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(Decimal::ONE),
        price_type: Set("standard".to_string()),
        effective_date: Set(eff_date()),
        status: Set(price_approval::PENDING.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect_err("引用不存在产品的价目行必须被 FK 拒绝（migration::domain::price_fk 所加约束）");

    assert!(
        matches!(
            db_err.sql_err(),
            Some(SqlErr::ForeignKeyConstraintViolation(_))
        ),
        "输入腿必须是驱动上报的 SQLSTATE 23503 外键违例，实际: {db_err:?}"
    );

    // 归类前置条件：From<DbErr> 把 23503 归入 DatabaseError(DB_RELATION)——
    // 这正是修复前 500 脱敏文案的来源形态
    let raw = AppError::from(db_err);
    assert!(
        matches!(&raw, AppError::DatabaseError(m) if m == err_msg::DB_RELATION),
        "23503 必须经 From<DbErr> 归类为 DatabaseError(DB_RELATION)，实际: {raw:?}"
    );

    let mapped = ProductService::map_product_fk_error(raw);
    expect_business_400(&mapped);
}

// ---------------------------------------------------------------------------
// ③-b 兜底映射负腿（不吞不改道）：非 FK 类数据库错误（真库 23505 唯一违例，
//     经 From<DbErr> 归类为 DatabaseError(DB_DUPLICATE)）经映射必须原样保留
//     DatabaseError/500——兜底只降级 FK 引用冲突，不拍平/吞掉真实故障。
//     同时锁死 crud 侧不得出现 AppError::internal 构造（wave4 内部拍平扫描族）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn map_product_fk_error_keeps_non_fk_database_errors_as_500() {
    let db = setup_test_db().await;
    seed_product(&db, PRODUCT_FREE_ID, "PRD-FK-DUP").await;

    let dup_err = product::ActiveModel {
        name: Set("价目FK守卫产品重复码腿".to_string()),
        code: Set("PRD-FK-DUP".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect_err("同码产品必须撞 products.code UNIQUE（23505）");
    assert!(
        matches!(
            dup_err.sql_err(),
            Some(SqlErr::UniqueConstraintViolation(_))
        ),
        "负腿输入必须是驱动上报的唯一约束违例（非 FK），实际: {dup_err:?}"
    );

    let mapped = ProductService::map_product_fk_error(AppError::from(dup_err));
    assert!(
        matches!(&mapped, AppError::DatabaseError(m) if m == err_msg::DB_DUPLICATE),
        "非 FK 数据库错误必须原样保留 DatabaseError，禁止被兜底映射吞成业务拒绝，实际: {mapped:?}"
    );
    assert_eq!(mapped.error_code(), "DATABASE_ERROR");
    assert_eq!(
        mapped.clone().into_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "真实故障保留 500，不吞异常"
    );
}

// ---------------------------------------------------------------------------
// ③-c 源码扫描锁（无 DB）：产品删除落点不得回潮
//     AppError::internal / AppError::database 手工重包装（wave4 internal_flatten
//     与 wave8 价目引用锁同族口径），且预检必须位于删除审计调用之前
// ---------------------------------------------------------------------------
#[test]
fn source_scan_delete_product_precheck_precedes_delete_and_no_flatten() {
    let src = include_str!("../src/services/product_ops/crud.rs").replace('\r', "");
    let delete_at = src
        .find("Self::find_product_price_references(id, &txn)")
        .expect("产品删除的价目引用预检不得丢失");
    let delete_exec_at = src
        .find("\"product\", id, Some(user_id)")
        .expect("delete_with_audit 锚点不得消失");
    assert!(
        delete_at < delete_exec_at,
        "价目引用预检必须位于删除写库之前"
    );
    assert!(
        src.contains("Self::map_product_fk_error(e)"),
        "删除阶段 23503 兜底映射不得丢失"
    );
    assert!(
        !src.contains("AppError::internal("),
        "product_ops/crud.rs 禁止 AppError::internal（业务拒绝拍平 500 回潮，wave4 internal_flatten 族）"
    );
    assert!(
        !src.contains("AppError::database(format!("),
        "product_ops/crud.rs 禁止 AppError::database(format!(…)) 重包装 DbErr（错误原文外泄风险）"
    );
}

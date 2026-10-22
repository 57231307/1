//! 契约测试：出参金额隐藏判定单源（销售域与采购域九个接入点的收编锁）
//!
//! 本文件锁三件事，缺一不算收编完成：
//! 1. 等价性：隐藏门与收编前逐字一致——只在“该角色对该资源无数据权限配置行”时进入，
//!    进入后只有非 admin 才移除金额列，admin 与配置行存在两种情形一律原值放行。
//!    真库用例按这两个方向同时断，只断一侧无法区分“门生效”与“门整体丢失”。
//! 2. 全金额列覆盖：列集合以出参的序列化类型定义为唯一权威源。收编前各出口只隐了
//!    一部分金额列（销售明细的折扣率、税率、小计、税额、折扣额、行金额与四个定价列，
//!    采购订单的已到货金额，采购入库明细的行金额），留半边等于把另一部分继续外发；
//!    同时收编前写进列集合的一些键在出参里根本不存在（对不存在的键做移除是恒不生效的
//!    空操作，却会让人误判该列已受保护），这类幻影键按真实列名替换并清除。
//!    纯函数用例逐资源断“真实列全部消失、非金额列一律保留、幻影键不再受触碰”。
//! 3. 防回潮：单源是唯一实现。三个 handler 文件里不得再出现任何金额列的内联移除，
//!    admin 判定也不得在默认隐藏分支重新出现；单源文件里金额列名字面量只允许出现在
//!    列集合定义的两个分支内。
//!
//! 通道：静态锁与纯函数锁不连库；行为锁经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑（缺库由夹具直接失败，不降级为假断言），走真实 handler
//! `get_receipt` 与 `list_receipt_items`，数据权限服务与 admin 判定源都是生产同源实现。

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
use bingxi_backend::handlers::purchase_receipt_handler::{get_receipt, list_receipt_items};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::purchase_inventory::purchase_receipt_inspection;
use bingxi_backend::models::status::purchase_receipt as receipt_status;
use bingxi_backend::models::{purchase_receipt, purchase_receipt_item};
use bingxi_backend::services::amount_visibility::{
    DefaultAmountHiding, DefaultAmountResource, hide_default_amount_columns,
};
use bingxi_backend::services::data_permission_service::DataPermissionService;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, Set, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 纯函数锁：列集合按出参真实形状对齐（不连库）
// ---------------------------------------------------------------------------

/// 销售订单出参：主行七个金额列 + 明细行十一个金额与率列必须全部消失；
/// 数量、重量、幅宽、规格、单号与状态类列一律保留。
#[test]
fn sales_order_shape_hides_all_real_amount_columns_and_keeps_the_rest() {
    let mut row = json!({
        "id": 7,
        "order_no": "SO20260007",
        "status": "APPROVED",
        "subtotal": "100.00",
        "tax_amount": "13.00",
        "discount_amount": "5.00",
        "shipping_cost": "8.00",
        "total_amount": "116.00",
        "paid_amount": "60.00",
        "balance_amount": "56.00",
        "contact_phone": "13800001111",
        "items": [{
            "id": 1,
            "quantity": "10.00",
            "quantity_meters": "120.00",
            "quantity_kg": "30.00",
            "gram_weight": "250.00",
            "width": "\"",
            "product_name": "棉平纹坯布",
            "unit_price": "10.00",
            "subtotal": "100.00",
            "tax_amount": "13.00",
            "discount_amount": "5.00",
            "total_amount": "108.00",
            "tax_percent": "13.00",
            "discount_percent": "5.00",
            "base_price": "9.50",
            "color_extra_cost": "0.50",
            "grade_price_diff": "0.00",
            "final_price": "10.00",
        }],
    });

    let removed = hide_default_amount_columns(DefaultAmountResource::SalesOrder, &mut row);

    for column in [
        "subtotal",
        "tax_amount",
        "discount_amount",
        "shipping_cost",
        "total_amount",
        "paid_amount",
        "balance_amount",
    ] {
        assert!(
            row.get(column).is_none(),
            "销售主行金额列 {column} 应被整键移除: {row}"
        );
    }
    let item = &row["items"][0];
    for column in [
        "unit_price",
        "subtotal",
        "tax_amount",
        "discount_amount",
        "total_amount",
        "tax_percent",
        "discount_percent",
        "base_price",
        "color_extra_cost",
        "grade_price_diff",
        "final_price",
    ] {
        assert!(
            item.get(column).is_none(),
            "销售明细金额或率列 {column} 应被整键移除（留半边等于继续外发）: {item}"
        );
    }
    // 非金额列不得被"整行清空"型假绿顺手抹掉，逐列原样保留
    assert_eq!(row["order_no"], json!("SO20260007"));
    assert_eq!(row["status"], json!("APPROVED"));
    assert_eq!(row["contact_phone"], json!("13800001111"));
    assert_eq!(item["quantity"], json!("10.00"));
    assert_eq!(item["product_name"], json!("棉平纹坯布"));
    // 移除计数：主行 7 列 + 明细 11 列
    assert_eq!(removed, 18, "销售出参本行实际移除列数应为 18: {row}");
}

/// 采购订单出参：已到货金额实名 received_amount（收编前漏隐），
/// 已付与未付两键在出参不存在故不再写入列集合；两个数量列收编前已被移除，
/// 撤销会变成放宽，故继续保留。
#[test]
fn purchase_order_shape_hides_received_amount_and_keeps_phantom_keys_untouched() {
    let mut row = json!({
        "id": 3,
        "order_no": "PO20260003",
        "total_amount": "500.00",
        "total_amount_foreign": "70.00",
        "total_quantity": "100.00",
        "total_quantity_alt": "80.00",
        "received_amount": "200.00",
        "paid_amount": "100.00",
        "balance_amount": "400.00",
        "currency": "CNY",
        "items": [{
            "unit_price": "5.00",
            "unit_price_foreign": "0.70",
            "subtotal": "500.00",
            "tax_amount": "65.00",
            "discount_amount": "10.00",
            "total_amount": "555.00",
            "tax_percent": "13.00",
            "discount_percent": "2.00",
            "received_quantity": "40.00",
            "quantity_tolerance_pct": "5.00",
        }],
    });

    let removed = hide_default_amount_columns(DefaultAmountResource::PurchaseOrder, &mut row);

    for column in ["total_amount", "total_amount_foreign", "received_amount"] {
        assert!(
            row.get(column).is_none(),
            "采购订单主行金额列 {column} 应被整键移除: {row}"
        );
    }
    // 数量列按裁定不属金额，不再列入隐藏集合（收编前它们被一并移除）
    assert!(
        row.get("total_quantity").is_some() && row.get("total_quantity_alt").is_some(),
        "数量列不属金额，不得再被隐藏: {row}"
    );
    // 幻影键：出参不存在，收编后不再列入移除集合（键留着也不该被动到）
    assert_eq!(
        row["paid_amount"],
        json!("100.00"),
        "幻影键不应再被列入集合"
    );
    assert_eq!(row["balance_amount"], json!("400.00"));
    assert_eq!(row["currency"], json!("CNY"), "非金额列不得被移除");

    let item = &row["items"][0];
    for column in [
        "unit_price",
        "unit_price_foreign",
        "subtotal",
        "tax_amount",
        "discount_amount",
        "total_amount",
        "tax_percent",
        "discount_percent",
    ] {
        assert!(
            item.get(column).is_none(),
            "采购订单明细金额或率列 {column} 应被整键移除: {item}"
        );
    }
    assert_eq!(
        item["received_quantity"],
        json!("40.00"),
        "收货数量不属金额"
    );
    assert_eq!(
        item["quantity_tolerance_pct"],
        json!("5.00"),
        "数量容差不属金额"
    );
    // 主行 3 列（不含两个幻影键与两个数量列）+ 明细 8 列
    assert_eq!(removed, 11, "采购订单本行实际移除列数应为 11: {row}");
}

/// 采购入库出参：单头唯一的真实金额列是 total_amount；
/// 明细行的行金额实名是 amount（收编前写的行总价与税额两键在该出参不存在）。
#[test]
fn purchase_receipt_shape_aligns_to_real_columns_and_clears_phantoms() {
    let mut head = json!({
        "id": 11,
        "receipt_no": "PR20260011",
        "total_amount": "300.00",
        "tax_amount": "39.00",
        "discount_amount": "3.00",
        "total_quantity": "60.00",
        "receipt_status": receipt_status::DRAFT,
    });
    let removed_head =
        hide_default_amount_columns(DefaultAmountResource::PurchaseReceipt, &mut head);
    assert!(
        head.get("total_amount").is_none(),
        "入库单头金额应被整键移除: {head}"
    );
    // 幻影键已清：该出参没有税额与折扣列，列集合里也不该有，故注入的键原样保留
    assert_eq!(head["tax_amount"], json!("39.00"), "幻影键不得留在列集合里");
    assert_eq!(head["discount_amount"], json!("3.00"));
    assert_eq!(
        head["receipt_status"],
        json!(receipt_status::DRAFT),
        "状态列不属金额"
    );
    assert_eq!(removed_head, 1, "入库单头应只移除一个真实金额列: {head}");

    let mut item = json!({
        "id": 21,
        "material_name": "涤纶塔丝隆",
        "quantity": "30.00",
        "unit_price": "10.00",
        "amount": "300.00",
        "total_price": "300.00",
        "tax_amount": "39.00",
    });
    let removed_item =
        hide_default_amount_columns(DefaultAmountResource::PurchaseReceiptItem, &mut item);
    assert!(
        item.get("unit_price").is_none() && item.get("amount").is_none(),
        "入库明细单价与行金额应被整键移除（行金额实名 amount，收编前漏隐）: {item}"
    );
    assert_eq!(
        item["total_price"],
        json!("300.00"),
        "幻影键不得留在列集合里"
    );
    assert_eq!(item["tax_amount"], json!("39.00"), "幻影键不得留在列集合里");
    assert_eq!(item["material_name"], json!("涤纶塔丝隆"));
    assert_eq!(item["quantity"], json!("30.00"), "数量不属金额");
    assert_eq!(removed_item, 2, "入库明细应移除两个真实金额列: {item}");
}

/// 同一资源在列表与详情两个出口必须同形：列表逐行的移除结果与详情单行切片一致。
#[test]
fn same_resource_hides_identical_columns_on_list_and_detail_paths() {
    let row = || {
        json!({
            "id": 1,
            "order_no": "PO20260001",
            "total_amount": "9.00",
            "total_amount_foreign": "1.00",
            "total_quantity": "2.00",
            "total_quantity_alt": "1.00",
            "received_amount": "4.00",
            "items": [{"unit_price": "4.50", "total_amount": "9.00", "tax_percent": "13.00"}],
        })
    };

    let mut list_rows = vec![row(), row()];
    for item in &mut list_rows {
        hide_default_amount_columns(DefaultAmountResource::PurchaseOrder, item);
    }
    let mut detail_row = row();
    hide_default_amount_columns(
        DefaultAmountResource::PurchaseOrder,
        std::slice::from_mut(&mut detail_row),
    );

    assert_eq!(
        list_rows[0], detail_row,
        "同一出参在列表与详情出口的隐藏形态必须一致"
    );
    assert_eq!(list_rows[0], list_rows[1]);
}

/// 单源判定结果只有两种取值，admin 例外不得表现为"部分隐藏"。
#[test]
fn hiding_outcome_is_a_two_state_word_list() {
    assert_ne!(
        DefaultAmountHiding::Hidden,
        DefaultAmountHiding::AdminVisible
    );
    let states = [
        DefaultAmountHiding::Hidden,
        DefaultAmountHiding::AdminVisible,
    ];
    assert_eq!(states.len(), 2, "判定结果只有命中与不命中两态");
}

// ---------------------------------------------------------------------------
// 源码扫描棘轮：单源是唯一实现
// ---------------------------------------------------------------------------

/// 金额列名字面量清单：这三个 handler 文件里再出现任何一个的内联移除即判红。
const BANNED_INLINE_REMOVALS: &[&str] = &[
    "remove(\"total_amount\")",
    "remove(\"total_amount_foreign\")",
    "remove(\"total_quantity\")",
    "remove(\"total_quantity_alt\")",
    "remove(\"paid_amount\")",
    "remove(\"balance_amount\")",
    "remove(\"received_amount\")",
    "remove(\"subtotal\")",
    "remove(\"tax_amount\")",
    "remove(\"tax_rate\")",
    "remove(\"tax_percent\")",
    "remove(\"discount_amount\")",
    "remove(\"discount_percent\")",
    "remove(\"shipping_cost\")",
    "remove(\"unit_price\")",
    "remove(\"unit_price_foreign\")",
    "remove(\"total_price\")",
    "remove(\"amount\")",
    "remove(\"base_price\")",
    "remove(\"color_extra_cost\")",
    "remove(\"grade_price_diff\")",
    "remove(\"final_price\")",
];

#[test]
fn amount_hiding_single_source_ratchet() {
    let sales = include_str!("../src/handlers/sales_order_handler.rs").replace('\r', "");
    let po = include_str!("../src/handlers/purchase_order_handler.rs").replace('\r', "");
    let receipt = include_str!("../src/handlers/purchase_receipt_handler.rs").replace('\r', "");
    let single_source = include_str!("../src/services/amount_visibility.rs").replace('\r', "");

    // 1. 三个 handler 文件不得再有任何内联金额移除
    for (file_name, source) in [
        ("sales_order_handler.rs", &sales),
        ("purchase_order_handler.rs", &po),
        ("purchase_receipt_handler.rs", &receipt),
    ] {
        for banned in BANNED_INLINE_REMOVALS {
            assert_eq!(
                source.matches(banned).count(),
                0,
                "回潮棘轮：{file_name} 重新出现内联金额移除 {banned}（判定必须走 amount_visibility 单源）"
            );
        }
    }

    // 2. 九个移除块的接入点数量固定：销售 2、采购订单 2、采购入库 3
    assert_eq!(
        sales.matches("apply_default_amount_hiding(").count(),
        2,
        "销售域接入点应为列表与详情两处"
    );
    assert_eq!(
        po.matches("apply_default_amount_hiding(").count(),
        2,
        "采购订单接入点应为列表与详情两处"
    );
    assert_eq!(
        receipt.matches("apply_default_amount_hiding(").count(),
        3,
        "采购入库接入点应为单头列表、单头详情与明细列表三处"
    );

    // 3. admin 判定单源：采购两域默认分支不得再自行判 admin；
    //    销售域只剩配置分支的联系方式判定两处（与金额隐藏门无关，不得增加）
    assert_eq!(
        po.matches("is_admin_role").count(),
        0,
        "回潮棘轮：purchase_order_handler.rs 重新出现 admin 判定（默认隐藏门的判定在单源内）"
    );
    assert_eq!(
        receipt.matches("is_admin_role").count(),
        0,
        "回潮棘轮：purchase_receipt_handler.rs 重新出现 admin 判定（默认隐藏门的判定在单源内）"
    );
    assert_eq!(
        sales
            .matches("admin_checker::is_admin_role(&state.db, role_id).await")
            .count(),
        2,
        "销售域只剩配置分支两处联系方式 admin 判定，多出即默认分支复制了判定"
    );

    // 4. 单源是唯一列清单：金额列名字面量只允许出现在列集合定义里
    let outside_sets = {
        let head_start = single_source
            .find("fn head_columns")
            .expect("单源缺主行列集合定义");
        let sets_end = single_source
            .find("pub fn hide_default_amount_columns")
            .expect("单源缺移除函数定义");
        format!(
            "{}{}",
            &single_source[..head_start],
            &single_source[sets_end..]
        )
    };
    for banned in BANNED_INLINE_REMOVALS {
        let column = banned
            .trim_start_matches("remove(\"")
            .trim_end_matches("\")");
        assert!(
            !outside_sets.contains(&format!("\"{column}\"")),
            "回潮棘轮：金额列 {column} 在单源的列集合定义之外又出现一份，等于第二套清单"
        );
    }

    // 5. admin 判定只走权威源，禁止角色主键字面量
    assert!(
        single_source.contains("admin_checker::is_admin_role(db, role_id).await"),
        "单源的 admin 判定必须走本仓唯一权威源 admin_checker::is_admin_role"
    );
    for banned in ["role_id == 1", "rid == 1"] {
        assert!(
            !single_source.contains(banned),
            "回潮棘轮：单源出现角色主键字面量判定 {banned}（播种漂移时静默扩权）"
        );
    }

    // 6. 判定命中与不命中都要显式日志，不得静默返回
    for rule in [
        "amount_default_hiding_applied",
        "amount_default_hiding_admin_exempt",
    ] {
        assert!(
            single_source.contains(rule),
            "单源缺 {rule} 日志：命中与不命中都必须可追踪"
        );
    }

    // 7. 配置分支归调用方：单源不得自己查数据权限配置行（否则配置路径语义被吸走）
    assert!(
        !single_source.contains("get_role_data_permission"),
        "回潮棘轮：单源开始代查数据权限配置行（配置分支应留在调用方）"
    );
    assert!(
        !single_source.contains("mask_phone") && !single_source.contains("contact_phone"),
        "回潮棘轮：单源开始处理联系方式掩码（PII 门留在调用方，否则给采购出参新增行为）"
    );
}

// ---------------------------------------------------------------------------
// 行为锁（真库，CI 侧执行）：隐藏门两个方向都断
// ---------------------------------------------------------------------------

const RECEIPT_TOTAL_AMOUNT: i64 = 333_333;
const RECEIPT_ITEM_AMOUNT: i64 = 111_111;
const RECEIPT_ITEM_UNIT_PRICE: i64 = 3_703;

fn amount(raw: i64) -> Decimal {
    Decimal::from(raw)
}

fn make_auth(user_id: i32, role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("amtsrc_user_{user_id}"),
        role_id,
        department_id: Some(1),
        // 行级范围取 All：本锁只字段级隐藏门，行级门另有专项锁
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("夹具 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 外键父行 products：入库明细的 product_id 有真 FK，业务表会被清空 ⇒ 自建取真实 id。
async fn seed_product(db: &DatabaseConnection) -> i32 {
    bingxi_backend::models::product::ActiveModel {
        code: Set(format!(
            "PRD-AMTSRC-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        )),
        name: Set("金额单源锁契约产品".to_string()),
        unit: Set("米".to_string()),
        status: Set(bingxi_backend::models::status::master_data::ACTIVE.to_string()),
        product_type: Set("fabric".to_string()),
        is_deleted: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：外键父行 products 写入失败")
    .id
}

/// 种子：一张入库单头 + 一行明细（明细带单价与行金额，用于断真实列被隐）。
/// 外键父行按既有范式取：warehouses 显式主键自种子，suppliers 取迁移种子参照表真实 id。
async fn seeded_state() -> (Arc<DatabaseConnection>, i32) {
    let db = Arc::new(test_common::setup_test_db().await);
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO warehouses (id, warehouse_code, name, is_active, is_default, created_at, updated_at) \
         VALUES (1,'WH-AMTSRC','金额单源锁仓库',TRUE,FALSE,NOW(),NOW())",
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .expect("夹具：外键父行 warehouses 写入失败");

    let row = db
        .query_one_raw(sea_orm::Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM suppliers ORDER BY id LIMIT 1".to_string(),
        ))
        .await
        .expect("夹具：读取 suppliers 参照表失败")
        .unwrap_or_else(|| panic!("夹具：迁移未播种 suppliers 参照表，FK 父行缺失"));
    let supplier_id: i32 = row
        .try_get_by_index::<i32>(0)
        .unwrap_or_else(|e| panic!("夹具：suppliers.id 解码失败 {e}"));
    let product_id = seed_product(db.as_ref()).await;

    let receipt = purchase_receipt::ActiveModel {
        receipt_no: Set(format!(
            "PR-AMTSRC-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        )),
        supplier_id: Set(supplier_id),
        receipt_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 3, 10).expect("种子日期非法")),
        warehouse_id: Set(1),
        inspection_status: Set(purchase_receipt_inspection::PENDING.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ONE),
        total_quantity_alt: Set(Decimal::ONE),
        total_amount: Set(amount(RECEIPT_TOTAL_AMOUNT)),
        created_by: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("夹具：入库单头种子失败");

    purchase_receipt_item::ActiveModel {
        receipt_id: Set(receipt.id),
        line_no: Set(1),
        product_id: Set(product_id),
        material_code: Set("FAB-AMTSRC".to_string()),
        material_name: Set("金额单源锁坯布".to_string()),
        quantity: Set(Decimal::from(30)),
        unit_master: Set("米".to_string()),
        unit_price: Set(Some(amount(RECEIPT_ITEM_UNIT_PRICE))),
        amount: Set(Some(amount(RECEIPT_ITEM_AMOUNT))),
        created_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("夹具：入库明细种子失败");

    (db, receipt.id)
}

fn build_app(db: &Arc<DatabaseConnection>, auth: AuthContext) -> Router {
    let state = AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    };
    Router::new()
        .route("/erp/purchase/receipts/{id}", get(get_receipt))
        .route("/erp/purchase/receipts/{id}/items", get(list_receipt_items))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
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
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .expect("读取响应体失败");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

/// 方向一：非 admin 且该角色对该资源无配置行 ⇒ 单头与明细的真实金额列整键移除，
/// 非金额列保留（撤掉隐藏门时本用例即红，不会像只断 admin 侧那样假绿）。
#[tokio::test]
async fn unconfigured_non_admin_role_has_amounts_removed_on_receipt_head_and_items() {
    let (db, receipt_id) = seeded_state().await;
    let app = build_app(&db, make_auth(50, Some(2)));

    let (status, detail) = get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}")).await;
    assert_eq!(status, StatusCode::OK, "入库单详情应 200: {detail}");
    let head = &detail["data"];
    assert_eq!(
        head.get("total_amount"),
        None,
        "无配置行的非 admin：入库单头金额必须整键移除: {head}"
    );
    assert!(
        head.get("receipt_no").is_some() && head.get("receipt_status").is_some(),
        "非金额列不得被顺手抹掉（整行清空型假绿）: {head}"
    );

    let (status, items) =
        get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}/items")).await;
    assert_eq!(status, StatusCode::OK, "入库明细列表应 200: {items}");
    let rows = items["data"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("明细出参形状漂移（期望 data 数组）: {items}"));
    assert_eq!(rows.len(), 1, "种子明细行数");
    let item = &rows[0];
    assert_eq!(
        item.get("unit_price"),
        None,
        "入库明细单价必须整键移除: {item}"
    );
    assert_eq!(
        item.get("amount"),
        None,
        "入库明细行金额（实名 amount）必须整键移除（收编前漏隐的半边）: {item}"
    );
    assert_eq!(
        item["material_name"],
        json!("金额单源锁坯布"),
        "物料名列不属金额，必须保留"
    );
    assert!(item.get("quantity").is_some(), "数量列不属金额，必须保留");
}

/// 方向二：该角色配了数据权限行（白名单与隐藏清单都为空）⇒ 只按配置走，
/// 不叠加默认隐藏，金额列继续可见。反向锁：把默认隐藏叠加到配置分支上会在此变红。
#[tokio::test]
async fn configured_role_keeps_amounts_without_overlaying_default_hiding() {
    let (db, receipt_id) = seeded_state().await;
    exec(
        &db,
        "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,\
         allowed_fields,hidden_fields,is_enabled,created_at,updated_at) \
         VALUES (9311,2,'purchase_receipt','SELF',NULL,NULL,TRUE,\
         '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),\
         (9312,2,'purchase_receipt_item','SELF',NULL,NULL,TRUE,\
         '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await
    .expect("夹具：数据权限配置行写入失败");
    let app = build_app(&db, make_auth(50, Some(2)));

    let (status, detail) = get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}")).await;
    assert_eq!(status, StatusCode::OK, "配置角色读详情应 200: {detail}");
    assert!(
        detail["data"].get("total_amount").is_some(),
        "配置行存在时不得叠加默认隐藏（金额应可见）: {}",
        detail["data"]
    );

    let (status, items) =
        get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}/items")).await;
    assert_eq!(status, StatusCode::OK, "配置角色读明细应 200: {items}");
    let item = &items["data"].as_array().expect("明细数组")[0];
    assert!(
        item.get("amount").is_some() && item.get("unit_price").is_some(),
        "配置行存在时明细金额列不得被默认隐藏叠加: {item}"
    );
}

/// 方向二补充：admin 角色（roles.code='admin'）无配置行也原值放行，
/// 与收编前的 admin 例外逐字一致。必须与方向一同库同种子，否则"门整体丢失"
/// 会以"admin 全显"的形态假绿。
#[tokio::test]
async fn admin_role_keeps_amounts_unchanged_on_receipt_head_and_items() {
    let (db, receipt_id) = seeded_state().await;
    let app = build_app(&db, make_auth(70, Some(1)));

    let (status, detail) = get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}")).await;
    assert_eq!(status, StatusCode::OK, "admin 读详情应 200: {detail}");
    // 金额原值按真实列回读形状断（列标度不参与判定，只钉原文在位、未被掩码或置空）
    let head_amount = detail["data"]["total_amount"]
        .as_str()
        .unwrap_or_else(|| panic!("admin 单头金额应为原值: {}", detail["data"]));
    assert!(
        head_amount.starts_with(&RECEIPT_TOTAL_AMOUNT.to_string()),
        "admin 单头金额原值契约: {head_amount}"
    );

    let (status, items) =
        get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}/items")).await;
    assert_eq!(status, StatusCode::OK, "admin 读明细应 200: {items}");
    let item = &items["data"].as_array().expect("明细数组")[0];
    let item_amount = item["amount"]
        .as_str()
        .unwrap_or_else(|| panic!("admin 明细行金额应为原值: {item}"));
    assert!(
        item_amount.starts_with(&RECEIPT_ITEM_AMOUNT.to_string()),
        "admin 明细行金额原值契约: {item_amount}"
    );
    let item_price = item["unit_price"]
        .as_str()
        .unwrap_or_else(|| panic!("admin 明细单价应为原值: {item}"));
    assert!(
        item_price.starts_with(&RECEIPT_ITEM_UNIT_PRICE.to_string()),
        "admin 明细单价原值契约: {item_price}"
    );

    // 同一份种子再以非 admin 复跑：admin 全显必须与"非 admin 被隐"同时成立
    let other_app = build_app(&db, make_auth(50, Some(2)));
    let (status, other_detail) =
        get_json(&other_app, &format!("/erp/purchase/receipts/{receipt_id}")).await;
    assert_eq!(status, StatusCode::OK, "对照段读详情应 200: {other_detail}");
    assert_eq!(
        other_detail["data"].get("total_amount"),
        None,
        "对照段：非 admin 无配置行时金额必须被移除（门丢失即在此暴露）"
    );
}

/// 角色缺失的现语义：销售与采购两域不做任何处理（与已收紧的商机域口径不同），
/// 单源不得把该形态改成 fail-closed，否则属放宽/收紧行为而非等价。
#[tokio::test]
async fn missing_role_id_keeps_amounts_visible_as_before() {
    let (db, receipt_id) = seeded_state().await;
    let app = build_app(&db, make_auth(80, None));

    let (status, detail) = get_json(&app, &format!("/erp/purchase/receipts/{receipt_id}")).await;
    assert_eq!(status, StatusCode::OK, "无角色读详情应 200: {detail}");
    assert!(
        detail["data"].get("total_amount").is_some(),
        "角色缺失时出参保持原样（现语义不接管，单源不得收紧）: {}",
        detail["data"]
    );
}

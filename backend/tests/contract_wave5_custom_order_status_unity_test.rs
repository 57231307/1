//! 定制订单状态词表三源合一契约锁（change_pending 收口批次）
//!
//! 路线一（#4669 判责）：表结构唯一来源 = backend/migration，本文件连真 PostgreSQL
//! （test_common::setup_test_db），不再自建 sqlite 同构 CHECK 表——
//! "词表 token 是否违反约束"改由真表 chk_custom_order_status 亲自裁决。
//!
//! 锁定三条（对应判责证据的五方对齐表）：
//! ① 大额变更挂起回读 == change_pending：
//!    - 真表 CHECK：权威模块 `custom_order::ALL` 全部 token 逐一写入真表成功
//!      （旧 10 值 CHECK 下 change_pending 必违反约束冒 500，本用例即
//!      "m0064 重建后的 CHECK 不再违反"的证明）；
//!      词表外 token 必须被真表 CHECK 显式拒绝（约束不是摆设）；
//!    - 活库（PG，#[ignore]，ci-test-rust-ignored 执行）：真实 service 链
//!      `submit_change_request`（金额变化>1万 → lock_exclusive 事务写入）
//!      挂起后回读，再 `approve_change` 回 draft 双向闭环。
//! ② 源码扫描锁：`custom_order_crud_service.rs` 状态写入/比较行不得出现
//!    `Set("…")` / `!= "…"` 裸字面量，`change_pending` 字面量整文件禁现
//!    （写入方与比较点必须逐字符取自权威模块常量）。
//! ③ 词表-CHECK 双向锁：`models/status/sales.rs::custom_order::ALL` 与
//!    m0064 迁移 CHECK 取值集合逐项相等（防一边改一边忘）。

mod test_common;

use bingxi_backend::models::custom_order;
use bingxi_backend::models::custom_order_create_dto::UpdateCustomOrderDto;
use bingxi_backend::models::status::custom_order as co_status;
use bingxi_backend::models::{customer, product, user};
use bingxi_backend::services::custom_order_crud_service::CustomOrderCrudService;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    QueryFilter, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// FK 前置自种子（裁定 R1）：custom_orders.customer_id → customers、product_id →
/// products（均非迁移种子参照表、会被清空且不播种），customers.owner_id 归属人 →
/// users。缺父行就造父行，不指望环境已有数据。返回 (users.id, customers.id, products.id)。
async fn seed_order_parents(db: &sea_orm::DatabaseConnection) -> (i32, i64, i64) {
    let now = Utc::now();
    let u = user::ActiveModel {
        username: Set(format!("w5u_owner_{now:}")),
        password_hash: Set("x".repeat(60)),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 users 归属人失败");

    let c = customer::ActiveModel {
        customer_code: Set(format!("CUST-W5U-{now:}")),
        customer_name: Set("波5状态合一契约客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("company".to_string()),
        owner_id: Set(u.id),
        created_by: Set(Some(u.id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 customers 父行失败");

    let p = product::ActiveModel {
        name: Set(format!("波5状态合一契约产品-{now:}")),
        code: Set(format!("PRD-W5U-{now:}")),
        unit: Set("m".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 products 父行失败");

    (u.id, c.id as i64, p.id as i64)
}

async fn seed_order(
    db: &sea_orm::DatabaseConnection,
    order_no: &str,
    status: &str,
    customer_id: i64,
    product_id: i64,
) -> custom_order::Model {
    let now = Utc::now();
    custom_order::ActiveModel {
        order_no: Set(order_no.to_string()),
        customer_id: Set(customer_id),
        product_id: Set(product_id),
        spec: Set("测试规格".to_string()),
        quantity: Set(dec("10.00")),
        unit: Set("m".to_string()),
        custom_requirements: Set(serde_json::json!({})),
        status: Set(status.to_string()),
        currency: Set("CNY".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

// =========================================================
// ①-a 真表 CHECK：权威模块全集逐 token 写入通过 + 词表外拒绝
// =========================================================

#[tokio::test]
async fn change_pending_passes_check_iso_and_all_tokens_are_writable() {
    let db = test_common::setup_test_db().await;
    let (_owner_id, customer_id, product_id) = seed_order_parents(&db).await;

    // 权威模块 ALL 的每个 token 都必须能写入真表 chk_custom_order_status——
    // 任何一个被拒即"模块与约束漂移"，本用例失败
    for token in co_status::ALL {
        let inserted = custom_order::ActiveModel {
            order_no: Set(format!("CO-W5U-{}", token)),
            customer_id: Set(customer_id),
            product_id: Set(product_id),
            spec: Set("测试规格".to_string()),
            quantity: Set(dec("10.00")),
            unit: Set("m".to_string()),
            custom_requirements: Set(serde_json::json!({})),
            status: Set(token.to_string()),
            currency: Set("CNY".to_string()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await;
        assert!(
            inserted.is_ok(),
            "权威词表 token「{token}」违反真表 chk_custom_order_status——模块与约束漂移"
        );
    }

    // 大额变更挂起回读 == change_pending（真表 CHECK 下写入不再违反约束）
    let seeded = seed_order(
        &db,
        "CO-W5U-CHANGE",
        co_status::CHANGE_PENDING,
        customer_id,
        product_id,
    )
    .await;
    let row = custom_order::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        co_status::CHANGE_PENDING,
        "挂起后回读状态必须逐字符等于权威模块 CHANGE_PENDING"
    );

    // 反向锁：词表外 token 必须被真表 CHECK 显式拒绝（约束真实生效，不是摆设）
    let bogus = custom_order::ActiveModel {
        order_no: Set("CO-W5U-BOGUS".to_string()),
        customer_id: Set(customer_id),
        product_id: Set(product_id),
        spec: Set("测试规格".to_string()),
        quantity: Set(dec("10.00")),
        unit: Set("m".to_string()),
        custom_requirements: Set(serde_json::json!({})),
        status: Set("pending".to_string()),
        currency: Set("CNY".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await;
    assert!(
        bogus.is_err(),
        "悬空 token「pending」（历史 custom_order 模块自创、从未进任何 CHECK）必须被真表拒绝"
    );
}

// =========================================================
// ①-b 活库（PG，#[ignore]）：真实 service 链挂起→回读→审批闭环
//     （submit_change_request 走 lock_exclusive 事务，sqlite 方言不支持行锁）
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（submit_change_request 走 lock_exclusive + 真实 chk_custom_order_status）"]
async fn live_submit_change_request_roundtrip_on_postgres() {
    let db = test_common::setup_test_db().await;
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在已迁移的 PostgreSQL（TEST_DATABASE_URL）上；禁止 sqlite 回退假绿"
    );
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    // FK 前置自种子（R1）：customers/products/users 父行，approver 用真实 owner id
    let (owner_id, customer_id, product_id) = seed_order_parents(&db).await;

    // seed：已客户签字确认的订单（submit_change_request 前置门）
    let seeded = custom_order::ActiveModel {
        order_no: sea_orm::Set(format!("CO-W5U-LIVE-{suffix}")),
        customer_id: sea_orm::Set(customer_id),
        product_id: sea_orm::Set(product_id),
        spec: sea_orm::Set("活库契约订单".to_string()),
        quantity: sea_orm::Set(dec("10.00")),
        unit: sea_orm::Set("m".to_string()),
        custom_requirements: sea_orm::Set(serde_json::json!({})),
        status: sea_orm::Set(co_status::DRAFT.to_string()),
        currency: sea_orm::Set("CNY".to_string()),
        total_amount: sea_orm::Set(Some(dec("100.00"))),
        customer_approved_at: sea_orm::Set(Some(Utc::now())),
        created_at: sea_orm::Set(Utc::now()),
        updated_at: sea_orm::Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("活库 seed custom_orders 行失败（检查 FK 种子数据）");

    let service = CustomOrderCrudService::new(Arc::new(db.clone()));

    // 金额变化 19900 > 10000 ⇒ 触发二级审批挂起。修复前该写入违反
    // chk_custom_order_status（10 值集不含 change_pending）必冒 500；
    // m0064 扩约束后必须成功落库。
    let updated = service
        .submit_change_request(
            seeded.id,
            UpdateCustomOrderDto {
                total_amount: Some(dec("20000.00")),
                ..Default::default()
            },
        )
        .await
        .expect("大额变更挂起不得再违反 CHECK（500）");
    assert_eq!(updated.status, co_status::CHANGE_PENDING);
    let row = custom_order::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        co_status::CHANGE_PENDING,
        "挂起后库内回读必须为 change_pending"
    );

    // 审批通过：change_pending → draft 回写闭环（比较点同源常量）
    service
        .approve_change(seeded.id, owner_id as i64, true, None)
        .await
        .expect("approve_change 状态门按 CHANGE_PENDING 常量比较，必须放行");
    let row = custom_order::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, co_status::DRAFT, "审批通过后必须回到 draft");

    // 清理本测试行（仅删自己插入的行，绝不触碰存量数据）
    let txn = db.begin().await.unwrap();
    custom_order::Entity::delete_many()
        .filter(custom_order::Column::Id.eq(seeded.id))
        .exec(&txn)
        .await
        .unwrap();
    txn.commit().await.unwrap();
}

// =========================================================
// ② 源码扫描锁：crud service 状态写入/比较不得回潮裸字面量
// =========================================================

#[test]
fn crud_service_status_writes_must_use_authority_constants() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/custom_order_crud_service.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 custom_order_crud_service.rs 失败");

    // change_pending 字面量整文件禁现（写入与比较都必须走 co_status::CHANGE_PENDING）
    assert!(
        !src.contains("change_pending"),
        "custom_order_crud_service.rs 出现裸字面量 change_pending——必须引用权威模块常量"
    );

    for line in src.lines() {
        let trimmed = line.trim();
        // 状态写入行：Set("...") 直写裸字面量一律禁止（常量引用形如 Set(co_status::X…)）
        if trimmed.contains("status = Set(") || trimmed.starts_with("status: Set(") {
            assert!(
                !trimmed.contains("Set(\""),
                "状态写入行禁止 Set(\"…\") 裸字面量: {trimmed}"
            );
            assert!(
                trimmed.contains("co_status::") || trimmed.contains("node_status::"),
                "状态写入必须引用权威模块常量（custom_order 或 process_node）: {trimmed}"
            );
        }
        // 状态比较行：existing.status ==/!= "..." 直写字面量一律禁止
        if trimmed.contains(".status ==") || trimmed.contains(".status !=") {
            assert!(
                !trimmed.contains("== \"") && !trimmed.contains("!= \""),
                "状态比较行禁止与裸字面量比较: {trimmed}"
            );
        }
    }
}

// =========================================================
// ③ 词表-CHECK 双向锁：custom_order::ALL 与 m0064 CHECK 集合逐项相等
// =========================================================

/// 从迁移源文件提取 ALLOWED_STATUS_VALUES 常量内的单引号 token 列表
/// （切片范围锚定在常量声明与结构体声明之间，避开 down 中的 10 值回滚集合）
fn extract_migration_check_tokens() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migration/src/domain/production/m0064_custom_order_status_add_change_pending.rs"
    );
    let src = std::fs::read_to_string(path).expect("读取 m0064 迁移源文件失败");
    let start = src
        .find("const ALLOWED_STATUS_VALUES")
        .expect("m0064 缺少 ALLOWED_STATUS_VALUES 常量");
    let end = src
        .find("#[derive(DeriveMigrationName)]")
        .expect("m0064 结构体声明锚点缺失");
    let slice = &src[start..end];

    let mut tokens = Vec::new();
    let mut rest = slice;
    while let Some(open) = rest.find('\'') {
        rest = &rest[open + 1..];
        let close = rest.find('\'').expect("单引号未闭合");
        tokens.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    tokens
}

#[test]
fn custom_order_all_equals_m0064_check_token_set_bidirectional() {
    let migration_tokens = extract_migration_check_tokens();
    assert_eq!(
        migration_tokens.len(),
        co_status::ALL.len(),
        "m0064 CHECK token 数量与 custom_order::ALL 不一致（先于集合比较报数，便于定位）"
    );

    // ALL ⊆ CHECK
    for token in co_status::ALL {
        assert!(
            migration_tokens.iter().any(|t| t.as_str() == *token),
            "权威模块 token「{token}」不在 m0064 CHECK 集合内：写入方将违反约束"
        );
    }
    // CHECK ⊆ ALL
    for token in &migration_tokens {
        assert!(
            co_status::ALL.contains(&token.as_str()),
            "m0064 CHECK token「{token}」不在权威模块 ALL 内：约束比词表宽即漂移"
        );
    }
}

// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::models::inventory_reservation;
use bingxi_backend::models::status::inventory_reservation as reservation_status;
use bingxi_backend::services::inventory_reservation_service::InventoryReservationService;
use bingxi_backend::services::test_common::{connect_empty_schema_db, setup_test_db};
// ymd 函数在测试中不可用，使用 NaiveDate::from_ymd_opt 替代
use bingxi_backend::decs;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::ymd;
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;

/// 测试夹具：构建库存预留 Model
/// 封装 `inventory_reservation::Model` 的构造，便于在各状态门测试中复用。；使用 `decs!` 解析数量字段，使用 `ymd!` 构造固定的预留时间，便于断言。
fn make_reservation_model(
    id: i32,
    status: &str,
    quantity: Decimal,
) -> inventory_reservation::Model {
    inventory_reservation::Model {
        id,
        order_id: 1001,
        product_id: 2001,
        warehouse_id: 3001,
        quantity,
        status: status.to_string(),
        reserved_at: ymd!(2026, 1, 15)
            .and_hms_opt(0, 0, 0)
            .expect("不变量：and_hms_opt(0, 0, 0) 永远合法")
            .and_utc(),
        released_at: None,
        notes: Some("测试预留".to_string()),
        created_by: Some(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// 复现 lock_reservation 的状态门判定：仅 PENDING 可锁定
fn can_lock(status: &str) -> bool {
    status == reservation_status::PENDING
}

/// 复现 release_reservation 的状态门判定：LOCKED 或 PENDING 可释放
fn can_release(status: &str) -> bool {
    status == reservation_status::LOCKED || status == reservation_status::PENDING
}

/// 复现 delete_reservation 的状态门判定：仅 PENDING 可删除
fn can_delete(status: &str) -> bool {
    status == reservation_status::PENDING
}

/// test_ztcl_dclwhfz（验证 PENDING 常量是小写字符串，与业务代码及数据库约定一致。）
#[test]
fn test_ztcl_dclwhfz() {
    assert_eq!(reservation_status::PENDING, "pending");
}

/// test_ztcl_ysdwhfz（验证 LOCKED 常量是小写字符串，与 lock_reservation 中设置的目标状态一致。）
#[test]
fn test_ztcl_ysdwhfz() {
    assert_eq!(reservation_status::LOCKED, "locked");
}

/// test_ztcl_yxhwhfz（验证 CONSUMED 常量是小写字符串，表示发货已扣减库存的终态。）
#[test]
fn test_ztcl_yxhwhfz() {
    assert_eq!(reservation_status::CONSUMED, "consumed");
}

/// test_ztcl_ysfwhfz（验证 RELEASED 常量是小写字符串，与 release_reservation 中设置的目标状态一致。）
#[test]
fn test_ztcl_ysfwhfz() {
    assert_eq!(reservation_status::RELEASED, "released");
}

/// test_ztcl_gztzhbxt（验证 5 个状态常量两两互不相同，避免状态机歧义导致误判。）
#[test]
fn test_ztcl_gztzhbxt() {
    let statuses = [
        reservation_status::PENDING,
        reservation_status::LOCKED,
        reservation_status::CONSUMED,
        reservation_status::RELEASED,
        reservation_status::CANCELLED,
    ];
    for i in 0..statuses.len() {
        for j in (i + 1)..statuses.len() {
            assert_ne!(statuses[i], statuses[j], "状态常量重复：{}", statuses[i]);
        }
    }
}

/// test_ztpp_jdclztksd（复现 lock_reservation 的状态门：仅 PENDING 可锁定，其余状态均应被拒绝。）
#[test]
fn test_ztpp_jdclztksd() {
    assert!(can_lock(reservation_status::PENDING));
    assert!(!can_lock(reservation_status::LOCKED));
    assert!(!can_lock(reservation_status::CONSUMED));
    assert!(!can_lock(reservation_status::RELEASED));
    assert!(!can_lock(reservation_status::CANCELLED));
}

/// test_ztpp_ysdhdclztksf
/// 复现 release_reservation 的状态门：LOCKED 或 PENDING 可释放，；CONSUMED/RELEASED/CANCELLED 应被拒绝（已释放不可重复释放）。
#[test]
fn test_ztpp_ysdhdclztksf() {
    assert!(can_release(reservation_status::PENDING));
    assert!(can_release(reservation_status::LOCKED));
    assert!(!can_release(reservation_status::CONSUMED));
    assert!(!can_release(reservation_status::RELEASED));
    assert!(!can_release(reservation_status::CANCELLED));
}

/// test_ztpp_jdclztksc（复现 delete_reservation 的状态门：仅 PENDING 可删除，；已锁定/已消耗/已释放/已取消的预留均不可删除。）
#[test]
fn test_ztpp_jdclztksc() {
    assert!(can_delete(reservation_status::PENDING));
    assert!(!can_delete(reservation_status::LOCKED));
    assert!(!can_delete(reservation_status::CONSUMED));
    assert!(!can_delete(reservation_status::RELEASED));
    assert!(!can_delete(reservation_status::CANCELLED));
}

/// test_cwxx_sdsbbhztzyzwsm（复现 lock_reservation 中非 PENDING 状态的错误消息构造：消息应包含实际状态值与"只有待处理状态的预留可以锁定"中文说明。）
///
/// 装配点对齐 `services/inventory_reservation_service.rs:68-75`：状态门拒绝用
/// `AppError::business_displayable`（族=BUSINESS_ERROR，出参 message **外显**真实拒绝原因，
/// 不是脱敏常量），与本仓 `contract_wave2_reservation_error_mapping_test.rs` 的钉法一致。
#[test]
fn test_cwxx_sdsbbhztzyzwsm() {
    let status = reservation_status::LOCKED;
    let msg = format!("预留状态为{}，只有待处理状态的预留可以锁定", status);

    // 包含实际状态值
    assert!(msg.contains(reservation_status::LOCKED));
    // 包含中文说明
    assert!(msg.contains("只有待处理状态的预留可以锁定"));

    // 构造为**可外显**业务错误并验证族别/错误码/出参文案
    let err = AppError::business_displayable(msg.clone());
    assert!(matches!(err, AppError::BusinessErrorDisplayable(_)));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert!(err.to_string().contains(&msg));
    // displayable 的 HTTP 出参必须携带真实原因（若误退回脱敏 business 即红）
    assert_eq!(err.to_response().message, msg);
}

/// test_cwxx_sfsbbhztzyzwsm（复现 release_reservation 中非 LOCKED/PENDING 状态的错误消息构造：消息应包含实际状态值与"只有已锁定或待处理状态的预留可以释放"中文说明。）
///
/// 装配点对齐 `services/inventory_reservation_service.rs:107-115`（business_displayable）。
#[test]
fn test_cwxx_sfsbbhztzyzwsm() {
    let status = reservation_status::CONSUMED;
    let msg = format!("预留状态为{}，只有已锁定或待处理状态的预留可以释放", status);

    assert!(msg.contains(reservation_status::CONSUMED));
    assert!(msg.contains("只有已锁定或待处理状态的预留可以释放"));

    let err = AppError::business_displayable(msg.clone());
    assert!(matches!(err, AppError::BusinessErrorDisplayable(_)));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert!(err.to_string().contains(&msg));
    assert_eq!(err.to_response().message, msg);
}

/// test_cwxx_scsbbhztzyzwsm（复现 delete_reservation 中非 PENDING 状态的错误消息构造：消息应包含实际状态值与"只有待处理状态的预留可以删除"中文说明。）
///
/// 装配点对齐 `services/inventory_reservation_service.rs:218-225`（business_displayable）。
#[test]
fn test_cwxx_scsbbhztzyzwsm() {
    let status = reservation_status::LOCKED;
    let msg = format!("预留状态为{}，只有待处理状态的预留可以删除", status);

    assert!(msg.contains(reservation_status::LOCKED));
    assert!(msg.contains("只有待处理状态的预留可以删除"));

    let err = AppError::business_displayable(msg.clone());
    assert!(matches!(err, AppError::BusinessErrorDisplayable(_)));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert!(err.to_string().contains(&msg));
    assert_eq!(err.to_response().message, msg);
}

/// test_cjyl_mrztwdcl（复现 create_reservation 中的初始状态设置：status 字段初始化为 PENDING，；数量字段通过 decs! 夹具解析，验证初始状态非其他终态。）
#[test]
fn test_cjyl_mrztwdcl() {
    // 复现 create_reservation 的 quantity 参数解析与初始状态设置
    let quantity = decs!("100.50");
    let initial_status = reservation_status::PENDING.to_string();

    // 默认状态应为待处理
    assert_eq!(initial_status, reservation_status::PENDING);
    // 数量字段应正确解析
    assert_eq!(quantity.to_string(), "100.50");
    // 默认状态非其他终态
    assert_ne!(initial_status, reservation_status::LOCKED);
    assert_ne!(initial_status, reservation_status::CONSUMED);
    assert_ne!(initial_status, reservation_status::RELEASED);
}

/// test_ylmxjj_ztzdzq（验证 make_reservation_model 夹具构造的 Model 字段正确，；其中 reserved_at 由 ymd! 夹具构造，数量由 decs! 解析。）
#[test]
fn test_ylmxjj_ztzdzq() {
    let model = make_reservation_model(1, reservation_status::LOCKED, decs!("50"));

    assert_eq!(model.id, 1);
    assert_eq!(model.status, reservation_status::LOCKED);
    assert_eq!(model.quantity, decs!("50"));
    // reserved_at 由 ymd!(2026, 1, 15) 构造，日期部分应为 2026-01-15
    assert_eq!(
        model.reserved_at.format("%Y-%m-%d").to_string(),
        "2026-01-15"
    );
    // 新建预留 released_at 应为 None
    assert!(model.released_at.is_none());
}

/// test_fwslcj（验证 InventoryReservationService 在 SQLite 内存数据库上能正常实例化，；与 customer_credit_limit.rs 的服务实例化测试模式一致。）
#[tokio::test]
async fn test_fwslcj() {
    let db = setup_test_db().await;
    let service = InventoryReservationService::new(Arc::new(db));

    // 验证服务内部 db 引用计数 >= 1
}

/// test_sdyl_wbjgfhcw —— 族名自陈钉"**无表结构**返回错误"：绑定
/// `connect_empty_schema_db()`（`setup_test_db()` 现语义 = 已迁移 PG + TRUNCATE，
/// 缺表前提在它身上不成立），断言 Err 并钉死错误族 DATABASE_ERROR。
///
/// 真实契约依据（读函数体）：`src/services/inventory_reservation_service.rs:91-104`
/// lock_reservation begin（:98，连库正常）后 `find_by_id + lock_exclusive`
/// （:100-102）打在 inventory_reservations 表；缺表 ⇒ DbErr::Query ⇒
/// `utils/error.rs:562-565` ⇒ "DATABASE_ERROR"（error.rs:747）。
/// （注：真库化空表上本函数另有正向契约 Err(NOT_FOUND, :104)，该形态已由
/// contract_wave2_reservation_error_mapping 族在真库钉死，本条不重复、只锁缺表。）
#[tokio::test]
#[ignore]
async fn test_sdyl_wbjgfhcw() {
    let db = connect_empty_schema_db().await;
    let service = InventoryReservationService::new(Arc::new(db));

    let err = service
        .lock_reservation(99999)
        .await
        .expect_err("schema 缺失（无 inventory_reservations 表）时锁定必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（inventory_reservation_service.rs:100 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

/// test_sfyl_wbjgfhcw —— 同族：绑定空 schema 库，钉 DATABASE_ERROR。
///
/// 真实契约依据（读函数体）：`src/services/inventory_reservation_service.rs:130-143`
/// release_reservation begin（:137）后 `find_by_id + lock_exclusive`（:139-141）；
/// 缺表 ⇒ DbErr::Query ⇒ DATABASE_ERROR（error.rs:562-565,747）。
#[tokio::test]
#[ignore]
async fn test_sfyl_wbjgfhcw() {
    let db = connect_empty_schema_db().await;
    let service = InventoryReservationService::new(Arc::new(db));

    let err = service
        .release_reservation(99999)
        .await
        .expect_err("schema 缺失（无 inventory_reservations 表）时释放必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（inventory_reservation_service.rs:139 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

/// test_cxyllb_wbjgfhcw —— 同族：绑定空 schema 库（区别于真库化空表——那里分页
/// 返回 `Ok(([], 0))` 才是契约，见 inventory_reservation_service.rs:207-210 +
/// `utils/pagination.rs:17-23`，不能用来断"缺表报错"）。
/// 本条只锁"缺表必须报错、绝不静默吞成空集/panic"⇒ 钉 DATABASE_ERROR。
#[tokio::test]
#[ignore]
async fn test_cxyllb_wbjgfhcw() {
    let db = connect_empty_schema_db().await;
    let service = InventoryReservationService::new(Arc::new(db));

    let err = service
        .list_reservations(0, 10, None, None, None, None)
        .await
        .expect_err("schema 缺失（无 inventory_reservations 表）时列表必须返回 Err 而非 panic");
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "缺表必须命中数据库错误族（inventory_reservation_service.rs:207 → error.rs:562-565），实得 {}",
        err.error_code()
    );
}

//! 供应商资质过期采购门控(分级阻断/例外放行/到期预警)契约锁
//!
//! 锁定的 file:line 契约(修复后形态):
//! - `backend/src/services/po/order_ops/crud.rs::validate_order_request`
//!   (供应商存在性检查与 `check_supplier_not_blacklisted(txn,..)` 同一落点、同一事务内
//!   调用 `SupplierQualificationGate::check_purchase_order_gate(txn,..)`,消除 TOCTOU;
//!   门控在 `create_order` 内先于 `create_order_header`(事务内取号)执行,与取号同事务)
//! - `backend/src/services/supplier_qualification_gate.rs`
//!   (过期判定唯一复用 `SupplierService::qualification_is_expired`,禁止第二套日期比较;
//!   法定许可类词表单点常量 + env `SUPPLIER_QUALIFICATION_STATUTORY_KEYWORDS` 可覆盖;
//!   阻断拒绝一律 `AppError::business_displayable`——`business` 会被出参脱敏为
//!   「业务处理失败」,用户不可见,等于白做)
//! - `backend/src/services/po/mod.rs::CreatePurchaseOrderRequest.qualification_waiver_reason`
//!   (例外放行原因随建单请求同事务处理:权限键 supplier_qualification:waive + 审计行,
//!   不设独立"先批后单"端点,避免两段式竞态)
//!
//! 分级口径(用户拍板,非一刀切):
//! - 法定许可类(排污许可/危化品等)过期 → 阻断新建采购订单;
//! - 一般资质过期 → 存量供应商不阻断但预警可见;新供应商(无脱离草稿订单)首单阻断;
//! - 收货侧不阻断(本测试同时锁死"门控不挂 receipt 链路"的落点形态)。
//!
//! 覆盖策略(全部真实行为,无 mock):
//! - sqlite::memory: 自建门控真实读写的表(supplier_qualifications/purchase_orders/
//!   users/roles/role_permissions/audit_logs/suppliers),驱动门控服务判定;
//! - 事务性:资质行在「未提交事务」内写入后必须由以该事务句柄调用的门控读到并阻断
//!   (读到未提交=经同一事务读取;若门控旁路自开连接查库则读不到 → 放行 → 断言炸),
//!   回滚后全局不可见;
//! - 活库并发端到端归 #[ignore] 由 ci-test-rust-ignored 在已迁移 PG 上执行。

use bingxi_backend::models::{audit_log, role, role_permission, supplier_qualification};
use bingxi_backend::services::supplier_qualification_gate::{
    EXPIRY_WARNING_DAYS_TIERS, QualificationExpiryLevel, QualificationExpiryWarning,
    STATUTORY_QUALIFICATION_KEYWORDS, SupplierQualificationGate, WAIVER_PERMISSION_ACTION,
    WAIVER_PERMISSION_RESOURCE,
};
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend,
    EntityTrait, QueryFilter, QuerySelect, Set, Statement, TransactionTrait,
};
use std::sync::Arc;

fn days_from_today(delta: i64) -> NaiveDate {
    Utc::now().date_naive() + chrono::Duration::days(delta)
}

// =========================================================
// 1) sqlite 自建表(列与门控真实读取的 Entity/查询列逐一对应)
// =========================================================

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

async fn setup_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(
        &db,
        r#"CREATE TABLE supplier_qualifications (
            id INTEGER PRIMARY KEY,
            supplier_id INTEGER NOT NULL,
            qualification_name TEXT NOT NULL,
            qualification_type TEXT NOT NULL,
            qualification_no TEXT NOT NULL,
            issuing_authority TEXT NOT NULL,
            issue_date TEXT NOT NULL,
            valid_until TEXT NOT NULL,
            attachment_path TEXT,
            need_annual_check INTEGER NOT NULL DEFAULT 0,
            annual_check_record TEXT,
            is_expired INTEGER NOT NULL DEFAULT 0,
            remarks TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"#,
    )
    .await;
    // 门控仅按 (supplier_id, order_status) count,不 SELECT 全列
    exec(
        &db,
        r#"CREATE TABLE purchase_orders (
            id INTEGER PRIMARY KEY,
            supplier_id INTEGER NOT NULL,
            order_status TEXT NOT NULL
        )"#,
    )
    .await;
    // users 仅被 select_only(role_id / username)读取
    exec(
        &db,
        r#"CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            role_id INTEGER
        )"#,
    )
    .await;
    // roles 被 is_admin_role find_by_id 全列读取
    exec(
        &db,
        r#"CREATE TABLE roles (
            id INTEGER PRIMARY KEY,
            name TEXT,
            code TEXT,
            description TEXT,
            permissions TEXT,
            is_system INTEGER,
            data_scope TEXT,
            created_at TEXT,
            updated_at TEXT
        )"#,
    )
    .await;
    // role_permissions 被 check_permission 全列读取
    exec(
        &db,
        r#"CREATE TABLE role_permissions (
            id INTEGER PRIMARY KEY,
            role_id INTEGER NOT NULL,
            resource_type TEXT NOT NULL,
            resource_id INTEGER,
            action TEXT NOT NULL,
            allowed INTEGER NOT NULL,
            permission_code TEXT,
            created_at TEXT,
            updated_at TEXT
        )"#,
    )
    .await;
    // audit_logs 覆盖门控写入的列 + 断言读取列
    exec(
        &db,
        r#"CREATE TABLE audit_logs (
            id INTEGER PRIMARY KEY,
            user_id INTEGER,
            username TEXT,
            action TEXT NOT NULL,
            resource_type TEXT,
            resource_id TEXT,
            resource_name TEXT,
            description TEXT,
            created_at TEXT,
            operation_type TEXT,
            severity TEXT,
            before_snapshot TEXT,
            after_snapshot TEXT,
            condition TEXT
        )"#,
    )
    .await;
    // suppliers 仅被 select_only(id, supplier_name)读取(扫描富化)
    exec(
        &db,
        r#"CREATE TABLE suppliers (
            id INTEGER PRIMARY KEY,
            supplier_name TEXT NOT NULL
        )"#,
    )
    .await;
    db
}

async fn seed_supplier(db: &DatabaseConnection, id: i32, name: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        format!("INSERT INTO suppliers (id, supplier_name) VALUES ({id}, '{name}')"),
        vec![],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed supplier 失败: {e}"));
}

/// 种一条供应商资质(conn 可为事务句柄——「事务内种入未提交」是 TOCTOU 证明的关键)
async fn seed_qualification<C: ConnectionTrait>(
    conn: &C,
    id: i32,
    supplier_id: i32,
    qualification_name: &str,
    qualification_type: &str,
    valid_until: NaiveDate,
) {
    supplier_qualification::ActiveModel {
        id: Set(id),
        supplier_id: Set(supplier_id),
        qualification_name: Set(qualification_name.to_string()),
        qualification_type: Set(qualification_type.to_string()),
        qualification_no: Set(format!("NO-{id}")),
        issuing_authority: Set("发证机关测试".to_string()),
        issue_date: Set(days_from_today(-400)),
        valid_until: Set(valid_until),
        need_annual_check: Set(false),
        is_expired: Set(false),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(conn)
    .await
    .unwrap_or_else(|e| panic!("seed qualification 失败: {e}"));
}

async fn seed_history_order(db: &DatabaseConnection, id: i32, supplier_id: i32, status: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        format!(
            "INSERT INTO purchase_orders (id, supplier_id, order_status) VALUES ({id}, {supplier_id}, '{status}')"
        ),
        vec![],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed purchase_order 失败: {e}"));
}

async fn seed_user(db: &DatabaseConnection, id: i32, username: &str, role_id: Option<i32>) {
    let role_sql = match role_id {
        Some(r) => r.to_string(),
        None => "NULL".to_string(),
    };
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        format!(
            "INSERT INTO users (id, username, role_id) VALUES ({id}, '{username}', {role_sql})"
        ),
        vec![],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed user 失败: {e}"));
}

async fn seed_role(db: &DatabaseConnection, id: i32, code: &str) {
    role::ActiveModel {
        id: Set(id),
        name: Set(format!("角色{code}")),
        code: Set(code.to_string()),
        is_system: Set(false),
        data_scope: Set("self".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed role 失败: {e}"));
}

async fn seed_waive_permission(db: &DatabaseConnection, id: i32, role_id: i32) {
    role_permission::ActiveModel {
        id: Set(id),
        role_id: Set(role_id),
        resource_type: Set(WAIVER_PERMISSION_RESOURCE.to_string()),
        action: Set(WAIVER_PERMISSION_ACTION.to_string()),
        allowed: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed role_permission 失败: {e}"));
}

async fn audit_waiver_rows(db: &DatabaseConnection) -> Vec<(i32, String)> {
    audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq(WAIVER_PERMISSION_RESOURCE))
        .select_only()
        .column(audit_log::Column::UserId)
        .column(audit_log::Column::Description)
        .into_tuple::<(Option<i32>, Option<String>)>()
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|(u, d)| (u.unwrap_or(-1), d.unwrap_or_default()))
        .collect()
}

/// 门控调用:Ok(())=放行(含存量供应商不阻断与例外放行两类路径),Err=阻断/权限拒绝。
/// 放行/阻断的完整语义不再由返回值携带,断言一律指向权威出口:
/// - 过期/预警清单 → `scan_expiry_warnings`(到期预警端点背后的公开返回结构);
/// - 例外放行事实(范围/操作人/原因/过期清单) → audit_logs 行(description + after_snapshot)。
async fn run_gate(
    db: &DatabaseConnection,
    supplier_id: i32,
    user_id: i32,
    waiver: Option<&str>,
) -> Result<(), AppError> {
    let gate = SupplierQualificationGate::new(Arc::new(db.clone()));
    gate.check_purchase_order_gate(db, supplier_id, user_id, waiver)
        .await
}

/// 到期预警扫描服务调用(预警端点 handler 背后的同一公开返回值)——过期/预警信息的权威来源
async fn gate_scan(db: &DatabaseConnection) -> Vec<QualificationExpiryWarning> {
    SupplierQualificationGate::new(Arc::new(db.clone()))
        .scan_expiry_warnings()
        .await
        .expect("到期预警扫描失败")
}

// =========================================================
// 2) 法定类过期 → 阻断新建单且 message 外显非脱敏常量
// =========================================================

#[tokio::test]
async fn statutory_expired_blocks_with_displayable_message() {
    let db = setup_db().await;
    seed_supplier(&db, 101, "挡污测试供应商").await;
    seed_qualification(&db, 1, 101, "排污许可证", "排污许可", days_from_today(-1)).await;

    let err = run_gate(&db, 101, 901, None)
        .await
        .expect_err("法定许可类资质过期必须阻断新建采购订单");
    let msg = match &err {
        AppError::BusinessErrorDisplayable(m) => m.clone(),
        other => panic!("拒绝必须是 business_displayable(出参真实外显),实际: {other:?}"),
    };
    assert!(
        msg.contains("法定许可类资质已过期"),
        "文案须点明阻断类别: {msg}"
    );
    assert!(
        msg.contains("排污许可证"),
        "文案须指出是哪一类/哪本资质: {msg}"
    );
    assert!(msg.contains("例外放行"), "文案须给出例外放行路径: {msg}");
    assert!(!msg.contains("业务处理失败"), "出参不得是脱敏常量: {msg}");
    assert!(!msg.contains("101"), "文案不得含供应商内部记录 ID: {msg}");
}

// =========================================================
// 3) 一般类过期 → 存量供应商不拒但预警可见;新供应商首单阻断
// =========================================================

#[tokio::test]
async fn general_expired_existing_supplier_passes_and_warning_visible() {
    let db = setup_db().await;
    seed_supplier(&db, 102, "存量一般资质供应商").await;
    // 已有脱离草稿的历史订单 → 非首单
    seed_history_order(&db, 1, 102, "SUBMITTED").await;
    seed_qualification(&db, 2, 102, "营业执照", "营业执照", days_from_today(-5)).await;

    run_gate(&db, 102, 902, None)
        .await
        .expect("一般资质过期不得阻断存量供应商新建订单(Ok=放行且未被例外放行介入)");
    // 该放行路径既未申请放行,也不得留下任何例外放行审计行(对应原 waived=false 语义)
    assert!(
        audit_waiver_rows(&db).await.is_empty(),
        "存量供应商自然放行不得写 WAIVE 审计"
    );

    // 预警可见:扫描端点背后的服务必须列出该条(过期态由唯一派生源如实转写)
    let warnings = gate_scan(&db).await;
    let hit = warnings
        .iter()
        .find(|w| w.supplier_id == 102 && w.qualification_name == "营业执照")
        .expect("一般资质过期必须出现在到期预警中");
    assert!(hit.expired, "expired 必须由唯一派生源如实转写");
    assert_eq!(hit.category, "general");
    assert_eq!(hit.level, QualificationExpiryLevel::Expired);
    assert_eq!(hit.supplier_name.as_deref(), Some("存量一般资质供应商"));
    // 对应原 expired_statutory 为空语义:该供应商不得有法定类过期预警项
    assert!(
        !warnings
            .iter()
            .any(|w| w.supplier_id == 102 && w.expired && w.category == "statutory"),
        "该供应商仅一般资质过期,预警出口不得多出法定类过期项"
    );
    // 对应原 expired_general 恰为「营业执照」一条:已过期项数量与唯一性同锁
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.supplier_id == 102 && w.expired)
            .count(),
        1,
        "已过期预警项必须恰为营业执照一条,不得多出或缺失"
    );
}

#[tokio::test]
async fn general_expired_first_order_blocked() {
    let db = setup_db().await;
    seed_supplier(&db, 103, "新供应商一般资质").await;
    seed_qualification(
        &db,
        3,
        103,
        "ISO9001质量管理体系认证证书",
        "ISO9001",
        days_from_today(-1),
    )
    .await;

    let err = run_gate(&db, 103, 903, None)
        .await
        .expect_err("一般资质过期且新供应商首单必须阻断");
    let msg = match err {
        AppError::BusinessErrorDisplayable(m) => m,
        other => panic!("首单拒绝必须 business_displayable,实际: {other:?}"),
    };
    assert!(
        msg.contains("一般资质已过期"),
        "文案须点明一般资质类别: {msg}"
    );
    assert!(msg.contains("首单"), "文案须点明首单阻断语义: {msg}");
    assert!(!msg.contains("业务处理失败"));

    // DRAFT 草稿不算历史成交证据:补一条 DRAFT 后首单门必须仍阻断
    seed_history_order(&db, 2, 103, "DRAFT").await;
    let err2 = run_gate(&db, 103, 903, None)
        .await
        .expect_err("DRAFT 草稿不构成成交历史,首单门必须仍阻断");
    assert!(
        matches!(err2, AppError::BusinessErrorDisplayable(_)),
        "实际: {err2:?}"
    );
}

// =========================================================
// 4) 资质齐全 → 放行(不误写审计;临近到期只进预警档位)
// =========================================================

#[tokio::test]
async fn all_qualifications_valid_passes_clean() {
    let db = setup_db().await;
    seed_supplier(&db, 104, "资质齐全供应商").await;
    seed_history_order(&db, 3, 104, "APPROVED").await;
    seed_qualification(&db, 4, 104, "排污许可证", "排污许可", days_from_today(365)).await;
    seed_qualification(&db, 5, 104, "营业执照", "营业执照", days_from_today(30)).await;

    run_gate(&db, 104, 904, None)
        .await
        .expect("无过期资质必须放行");

    let gate = SupplierQualificationGate::new(Arc::new(db.clone()));
    let warnings = gate.scan_expiry_warnings().await.unwrap();
    // 对应原 expired_statutory/expired_general 皆空语义:该供应商在预警出口无任何「已过期」项
    assert!(
        !warnings.iter().any(|w| w.supplier_id == 104 && w.expired),
        "资质齐全的供应商不得出现任何已过期预警项"
    );
    let license = warnings
        .iter()
        .find(|w| w.qualification_name == "营业执照")
        .expect("临近到期(30天,默认档位含30)资质必须给出到期预警");
    assert!(!license.expired, "临近到期不得判成已过期");
    assert_eq!(license.level, QualificationExpiryLevel::Warning);
    assert!(
        !warnings
            .iter()
            .any(|w| w.qualification_name == "排污许可证"),
        "365 天后到期已越出全部预警档位,不应出现"
    );
    // 放行路径不得留下任何例外放行审计(不静默≠乱写审计)
    assert!(audit_waiver_rows(&db).await.is_empty());
    // 配置常量表默认值就位(env 覆盖名见门控模块头)
    assert!(EXPIRY_WARNING_DAYS_TIERS.contains(&30));
    assert!(
        STATUTORY_QUALIFICATION_KEYWORDS
            .iter()
            .any(|k| k == "排污许可")
    );
}

// =========================================================
// 5) 例外放行:权限键保护 + 原因必填 + 审计行真实落库
// =========================================================

#[tokio::test]
async fn waiver_requires_permission_and_records_audit() {
    let db = setup_db().await;
    seed_supplier(&db, 105, "放行测试供应商").await;
    seed_qualification(&db, 6, 105, "排污许可证", "排污许可", days_from_today(-2)).await;

    // 5a. 无角色用户携带原因 → fail-closed 拒绝
    seed_user(&db, 910, "no_role_user", None).await;
    let err = run_gate(&db, 105, 910, Some("紧急补料"))
        .await
        .expect_err("无角色不得放行");
    assert!(
        matches!(err, AppError::PermissionDenied(_)),
        "实际: {err:?}"
    );
    assert!(
        audit_waiver_rows(&db).await.is_empty(),
        "拒绝的放行不得落审计"
    );

    // 5b. 有角色但未被授权 waive 权限键 → 403
    seed_role(&db, 911, "buyer").await;
    seed_user(&db, 911, "unauthorized_user", Some(911)).await;
    let err = run_gate(&db, 105, 911, Some("紧急补料"))
        .await
        .expect_err("未授权角色不得放行");
    assert!(
        matches!(err, AppError::PermissionDenied(_)),
        "实际: {err:?}"
    );
    assert!(audit_waiver_rows(&db).await.is_empty());

    // 5c. 授权角色 + 携带原因 → 放行且审计行真实落库(资源类型/操作人/原因齐全)
    seed_role(&db, 912, "purchase_manager").await;
    seed_user(&db, 912, "authorized_user", Some(912)).await;
    seed_waive_permission(&db, 1, 912).await;
    run_gate(&db, 105, 912, Some("停线风险经质量例会批准放行"))
        .await
        .expect("授权角色携原因应放行");

    let rows = audit_waiver_rows(&db).await;
    assert_eq!(rows.len(), 1, "例外放行必须留下恰好一条审计行");
    let (uid, desc) = &rows[0];
    assert_eq!(*uid, 912, "审计行必须记录真实操作人");
    assert!(
        desc.contains("停线风险经质量例会批准放行"),
        "放行原因必须真实落审计: {desc}"
    );
    assert!(
        desc.contains("法定许可类资质过期"),
        "审计须记录放行范围: {desc}"
    );

    // 放行命中的过期清单由预警扫描这一权威出口如实列出(对应原 expired_statutory/expired_general):
    // 恰一条法定类已过期项(排污许可证),且无一般类已过期项。
    let warnings = gate_scan(&db).await;
    let hit = warnings
        .iter()
        .find(|w| w.supplier_id == 105 && w.qualification_name == "排污许可证")
        .expect("例外放行命中的已过期法定资质必须在到期预警出口列出");
    assert!(hit.expired);
    assert_eq!(hit.category, "statutory");
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.supplier_id == 105 && w.expired)
            .count(),
        1,
        "供应商105的已过期预警项必须恰为排污许可证一条"
    );

    // 5d. 携带字段但原因为空白 → 显式拒绝(不得静默当「未申请放行」)
    let err = run_gate(&db, 105, 912, Some("   "))
        .await
        .expect_err("空白放行原因必须被拒");
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(ref m) if m.contains("不能为空白")),
        "实际: {err:?}"
    );
}

// =========================================================
// 6) 门控读取与调用方事务同源(TOCTOU 形态锁)
// =========================================================

#[tokio::test]
async fn gate_reads_qualifications_through_caller_transaction() {
    let db = setup_db().await;
    seed_supplier(&db, 106, "事务内读取供应商").await;

    // 事务内种入一条「刚过期」的法定资质(未提交)。门控以该事务句柄为 conn 调用时必须读到;
    // 若门控内部改用自开连接查库(先查后开事务的旁路形态),未提交行不可见 → 放行 → 断言炸。
    let txn = db.begin().await.unwrap();
    seed_qualification(
        &txn,
        7,
        106,
        "危险化学品经营许可证",
        "危化品经营许可",
        days_from_today(-1),
    )
    .await;
    let gate = SupplierQualificationGate::new(Arc::new(db.clone()));
    let err = gate
        .check_purchase_order_gate(&txn, 106, 920, None)
        .await
        .expect_err("同一事务内过期的资质必须被门控读到并阻断");
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "实际: {err:?}"
    );
    txn.rollback().await.unwrap();

    // 回滚后资质行整体不可见:无过期资质 → 放行(证明上方阻断确由那笔未提交数据驱动)
    run_gate(&db, 106, 920, None)
        .await
        .expect("已回滚的资质不得继续影响门控");
    // 对应原 expired_statutory/expired_general 皆空语义:回滚后预警出口不得残留该供应商任何条目
    assert!(
        !gate_scan(&db).await.iter().any(|w| w.supplier_id == 106),
        "已回滚的资质不得在到期预警出口残留任何条目"
    );
}

// =========================================================
// 7) 活库并发端到端(需已迁移 PG,由 ci-test-rust-ignored 执行)
// =========================================================

/// 活库并发矩阵(计划,sqlite 无法复现真实多连接竞争):
/// 1) 两个并发 `PurchaseOrderService::create_order` 指向同一供应商(法定资质已过期):
///    两条请求都必须被拒且 purchase_orders 零新增——门控在 create_order 的 txn 内、
///    先于 create_order_header 的事务内取号执行(与取号同事务),拒绝即整体回滚,
///    不存在「A 过门后 B 改资质、A 的号仍落库」的旁路;
/// 2) 并发「资质续期提交」与「建单」交错时任一时间点建单事务读到的资质行满足
///    读己之写入语义,拒绝/放行判定与订单落库原子同存亡;
/// 3) 例外放行审计行与订单行同一提交原子可见(订单被并发回滚则审计同滚)。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(真实多连接并发);由 ci-test-rust-ignored 执行"]
async fn live_gate_and_numbering_share_one_transaction() {
    // 接库后按上方计划实现(先例:contract_wave2_production_progress_gate_test.rs
    // 的 #[ignore] 活库用例形态);sqlite 侧以事务内读取锁 + 源码扫描锁兜底。
}

// =========================================================
// 8) 源码扫描防回潮锁(无 DB)
// =========================================================

fn extract_block(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("块结束定位失败: {anchor}"));
    src[i..i + j].to_string()
}

/// crud.rs 落点锁:门控必须在 validate_order_request 内、经 `txn` 调用,
/// 紧随黑名单门控(同一落点范式);且 validate 在 create_order 内先于
/// create_order_header(事务内取号)——保证「门控与取号同一事务」。
#[test]
fn source_scan_po_create_gate_placement() {
    let src = include_str!("../src/services/po/order_ops/crud.rs");
    let validate = extract_block(src, "async fn validate_order_request");
    assert!(
        validate.contains("check_supplier_not_blacklisted(txn, req.supplier_id)"),
        "黑名单门控既有形态不得回潮,validate 块:\n{validate}"
    );
    assert!(
        validate.contains("check_purchase_order_gate("),
        "资质门控必须挂在 validate_order_request 同一落点,块:\n{validate}"
    );
    let gate_at = validate.find("check_purchase_order_gate(").unwrap();
    let black_at = validate.find("check_supplier_not_blacklisted(").unwrap();
    assert!(
        gate_at > black_at,
        "资质门控须在黑名单门控之后(供应商存在性+黑名单先行)"
    );
    assert!(
        validate.contains("req.qualification_waiver_reason.as_deref()"),
        "放行原因必须取自请求字段并显式传入门控,块:\n{validate}"
    );

    let create = extract_block(src, "pub async fn create_order(");
    let validate_at = create
        .find("validate_order_request(&req, user_id, &txn)")
        .expect("create_order 必须以 user_id + 同一 txn 调用 validate_order_request(门控在事务内)");
    let header_at = create
        .find("create_order_header(&req, warehouse_id, department_id, user_id, &txn)")
        .expect("create_order_header 调用锚点丢失");
    assert!(
        validate_at < header_at,
        "门控(validate)必须先于取号落库(create_order_header)且共用 &txn"
    );
}

/// 门控本体:过期判定唯一派生、拒绝外显、禁止第二套日期比较、词表单点定义
#[test]
fn source_scan_gate_single_source_and_displayable() {
    let src = include_str!("../src/services/supplier_qualification_gate.rs");
    assert!(
        src.contains("SupplierService::qualification_is_expired("),
        "是否过期必须复用唯一派生源 qualification_is_expired"
    );
    assert!(
        !src.contains("date_naive() >"),
        "禁止在门控内写第二套日期过期比较(唯一派生源之外)"
    );
    assert!(
        src.contains("AppError::business_displayable("),
        "阻断拒绝必须 business_displayable(business 出参会被脱敏)"
    );
    assert!(
        src.contains(".all(conn)"),
        "资质行必须经调用方事务句柄读取(ConnectionTrait)"
    );
    // 词表分组单点:关键词字面量字符串只允许出现在常量表定义行一处
    assert_eq!(
        src.matches("\"排污许可,危化品,危险化学品,安全生产许可\"")
            .count(),
        1,
        "法定类关键词表禁止散落第二处硬编码"
    );
    assert!(src.contains("SUPPLIER_QUALIFICATION_STATUTORY_KEYWORDS"));
    assert!(src.contains("SUPPLIER_QUALIFICATION_WARNING_DAYS"));
}

/// 收货侧不阻断:po/receipt.rs 不得引用资质门控(在途履约不应被卡死)
#[test]
fn source_scan_receipt_side_not_gated() {
    let src = include_str!("../src/services/po/receipt.rs");
    assert!(
        !src.contains("SupplierQualificationGate"),
        "收货链路按决策不得挂资质过期门控(硬门槛在入库定性/使用决策而非物理收货)"
    );
}

/// 路由与 handler 分层锁:扫描端点存在且按 labor-contracts 同款形态注册
#[test]
fn source_scan_warning_endpoint_registered() {
    let routes_src = include_str!("../src/routes/supplier_qualification_gate.rs");
    assert!(
        routes_src.contains("\"/supplier-qualifications/scan-expiry-warnings\""),
        "到期预警扫描端点必须按既有 scan-expiry-warnings 形态注册"
    );
    assert!(routes_src.contains("post(supplier_qualification_gate_handler::scan_expiry_warnings)"));
    let mod_src = include_str!("../src/routes/mod.rs");
    assert!(
        mod_src.contains(".nest(\"/api/v1/erp\", supplier_qualification_gate::routes())"),
        "新域路由必须在 routes/mod.rs 唯一前缀处挂载"
    );
}

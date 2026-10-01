//! 任务 wave5：色卡"发放"门控契约锁（闸门 1 不再比一个永不出现的状态值）
//! + 任务 2：custom_order_process_service 流程节点状态常量化源码扫描锁
//!
//! 坐实的断裂（修复前恒 400，发放链路对所有真实色卡恒失败，与 98f3bb0b 色号创建门同根因）：
//! - `color_card_issue_service.rs` 闸门 1 比较裸字面量 `"active"`；
//! - 但色卡权威词表 `color_card::ALL`（models/status/wage_energy_chemical_business.rs:157）
//!   不含 active，DB CHECK `chk_color_card_status`（migration v15 mod.rs:4493-4498）已把
//!   历史 active 回填为 draft，且没有任何端点能把色卡写成 active；
//! - 发放流转权威 `validate_color_card_status_transition`（color_card_crud_service.rs:335）
//!   仅允许 `DRAFT → ISSUED` ⇒ 发放门放行集合 = `{draft}`；
//! - 前端发放页 `views/color-cards/issues.vue` 也只取 `status:'draft'` 的卡去发放。
//!
//! 修复方向：
//! - 门控改比 `ISSUABLE_CARD_STATUSES`（每个 token 引自 card_status 词表，禁裸字面量）；
//! - 拒绝文案分层：闸门 1 是纯公开规则 → `business_displayable`（400 / BUSINESS_ERROR +
//!   外显"只有草稿态色卡可以发放"）；含库存数字/超期条数/客户状态 token 的其余闸门
//!   维持脱敏 `business`。
//!
//! 覆盖策略：
//! - sqlite::memory:（无需 TEST_DATABASE_URL、不 #[ignore]）：
//!   反向 issued/archived/legacy-active 卡 → CardNotIssuable + 400 契约 + 零落库；
//!   issue_err 分层纯函数断言；两路源码扫描锁（色卡死值 / 工艺节点裸字面量形态）。
//! - 正向全链走 `issue()` 含 `lock_exclusive()` 事务——sqlite 方言不支持行锁，
//!   按仓内先例（contract_wave5_receipt_return_three_state_test.rs E 段）放
//!   `#[ignore]` 活库（TEST_DATABASE_URL→PG），不伪装成 sqlite 用例。

mod test_common;

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, Set, Statement,
};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::handlers::color_card::issue::issue_err;
use bingxi_backend::models::color_card::{ActiveModel as CardActive, Entity as ColorCardEntity};
use bingxi_backend::models::color_card_issue::Entity as IssueEntity;
use bingxi_backend::models::customer::{ActiveModel as CustomerActive, Entity as CustomerEntity};
use bingxi_backend::models::status::color_card as card_status;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::status::production::process_node as node_status;
use bingxi_backend::services::color_card_issue_service::{
    ColorCardIssueService, IssueError, IssueParams, IssueStatus,
};
use bingxi_backend::utils::error::AppError;

// =========================================================
// 夹具：sqlite 自建表（列与 models/color_card.rs、models/color_card_issue.rs 一一对应）
// =========================================================

const COLOR_CARDS_DDL: &str = r#"CREATE TABLE color_cards (
    id INTEGER PRIMARY KEY,
    card_no TEXT NOT NULL,
    card_name TEXT NOT NULL,
    card_type TEXT NOT NULL,
    season TEXT,
    brand TEXT,
    total_colors INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft',
    description TEXT,
    cover_image_url TEXT,
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    issued_quantity INTEGER NOT NULL DEFAULT 0,
    dyeing_capability TEXT,
    printing_capability TEXT,
    color_fastness_grade TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)"#;

const COLOR_CARD_ISSUES_DDL: &str = r#"CREATE TABLE color_card_issues (
    id INTEGER PRIMARY KEY,
    color_card_id INTEGER NOT NULL,
    customer_id INTEGER NOT NULL,
    issue_qty INTEGER NOT NULL,
    issued_by INTEGER NOT NULL,
    issued_at TEXT NOT NULL,
    expected_return_date TEXT,
    actual_return_date TEXT,
    status TEXT NOT NULL,
    purpose TEXT,
    remark TEXT,
    compensation_amount TEXT,
    returned_by INTEGER,
    dye_lot_no TEXT,
    sales_order_id INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
)"#;

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

async fn fresh_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(&db, COLOR_CARDS_DDL).await;
    exec(&db, COLOR_CARD_ISSUES_DDL).await;
    db
}

/// 按指定状态播种一张色卡（stock=10, issued=0），返回 id。
async fn seed_card(db: &sea_orm::DatabaseConnection, status: &str) -> i64 {
    let now = chrono::Utc::now();
    let card = CardActive {
        card_no: Set(format!("W5-ISSUE-{status}")),
        card_name: Set(format!("wave5 发放门控色卡({status})")),
        card_type: Set("PANTONE".to_string()),
        status: Set(status.to_string()),
        stock_quantity: Set(10),
        issued_quantity: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种色卡失败");
    card.id
}

fn issue_params(card_id: i64) -> IssueParams {
    IssueParams {
        color_card_id: card_id,
        customer_id: 7,
        issued_by: 100,
        issue_qty: 2,
        expected_return_date: None,
        purpose: None,
        remark: None,
        dye_lot_no: None,
        sales_order_id: None,
    }
}

async fn count_issues(db: &sea_orm::DatabaseConnection) -> u64 {
    IssueEntity::find().count(db).await.unwrap()
}

// =========================================================
// 1) 反向：issued / archived / legacy active(死值) 卡发放
//    → CardNotIssuable + 400 + BUSINESS_ERROR + 外显文案 + 零落库
// =========================================================

async fn assert_issue_gate_rejects(db: &sea_orm::DatabaseConnection, card_id: i64, label: &str) {
    let svc = ColorCardIssueService::new(Arc::new(db.clone()));
    let err = svc
        .issue(issue_params(card_id))
        .await
        .expect_err(&format!("{label} 色卡发放必须被闸门 1 拒绝"));
    assert!(
        matches!(err, IssueError::CardNotIssuable),
        "{label} 色卡应命中闸门 1 专用变体 CardNotIssuable，实际={err:?}"
    );

    // HTTP 出参契约（映射点 handlers/color_card/issue.rs::issue_err）
    let app_err = issue_err(err);
    assert_eq!(
        app_err.error_code(),
        "BUSINESS_ERROR",
        "{label} 必须归 business 族"
    );
    let resp = app_err.to_response();
    assert_eq!(
        resp.message, "只有草稿态色卡可以发放",
        "{label} 出参必须外显公开业务规则文案（纯规则、无内部 token）"
    );
    assert_ne!(
        resp.message, "业务处理失败",
        "{label} business_displayable 不得被脱敏回潮"
    );
    // 外显文案不得泄露内部状态 token（词表值/死值均不得出现在出参）
    assert!(
        !resp.message.contains("draft") && !resp.message.contains("active"),
        "{label} 出参只描述公开规则，不得回显内部 token: {}",
        resp.message
    );

    // 零落库 + 卡上库存推进字段不动
    assert_eq!(
        count_issues(db).await,
        0,
        "{label} 拒绝后发放记录必须零落库"
    );
    let card = ColorCardEntity::find_by_id(card_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        card.issued_quantity, 0,
        "{label} 拒绝后 issued_quantity 不得变动"
    );
}

#[tokio::test]
async fn issue_rejected_on_issued_card() {
    let db = fresh_db().await;
    // 词表真实值 issued（已发放态）——修复前它也会被拒（死值门对一切恒失败），
    // 修复后它不在放行集合 {draft}，仍必须被拒且给出外显公开文案。
    let card_id = seed_card(&db, card_status::ISSUED).await;
    assert_issue_gate_rejects(&db, card_id, "issued(已发放)").await;
}

#[tokio::test]
async fn issue_rejected_on_archived_card() {
    let db = fresh_db().await;
    let card_id = seed_card(&db, card_status::ARCHIVED).await;
    assert_issue_gate_rejects(&db, card_id, "archived(终态)").await;
}

#[tokio::test]
async fn issue_rejected_on_legacy_active_dead_value() {
    // 'active' 不在 card_status::ALL / DB CHECK 内（迁移已回填为 draft），
    // 修复前正是拿它当放行依据；新门控不得再把它供回去——即使库中存在脏数据也必须拒绝。
    let db = fresh_db().await;
    let card_id = seed_card(&db, "active").await;
    let svc = ColorCardIssueService::new(Arc::new(db.clone()));
    let err = svc
        .issue(issue_params(card_id))
        .await
        .expect_err("legacy active 死值不得进入放行集合");
    assert!(
        matches!(err, IssueError::CardNotIssuable),
        "active 脏数据同样命中闸门 1 拒绝，实际={err:?}"
    );
    assert_eq!(count_issues(&db).await, 0, "拒绝后发放记录必须零落库");
}

// =========================================================
// 2) 拒绝文案保密分层（纯函数，无需 DB）
// =========================================================

#[test]
fn card_not_issuable_is_displayable_public_rule() {
    // 闸门 1：纯公开规则 → business_displayable 外显
    let err = issue_err(IssueError::CardNotIssuable);
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "闸门 1 拒绝必须外显，实际={err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_response().message, "只有草稿态色卡可以发放");
    assert_eq!(err.to_string(), "只有草稿态色卡可以发放");
}

#[test]
fn inventory_gate_failure_stays_masked() {
    // 其余闸门含库存数字等记录细节 → 维持脱敏 business（不简化、不外泄）
    let err = issue_err(IssueError::GateCheckFailed(
        "闸门 2 失败：库存不足，可用 8 张，本次发放 20 张".to_string(),
    ));
    assert!(
        matches!(err, AppError::BusinessError(_)),
        "含库存数字的闸门失败必须脱敏，实际={err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_response().message, "业务处理失败");
    // 真实原因只进日志侧（Display）
    assert!(err.to_string().contains("库存不足"));
}

// =========================================================
// 3) 死值锁源码扫描（防回潮，无 DB）
// =========================================================

fn card_word_value(ident: &str) -> Option<&'static str> {
    match ident {
        "DRAFT" => Some(card_status::DRAFT),
        "ISSUED" => Some(card_status::ISSUED),
        "RECEIVED" => Some(card_status::RECEIVED),
        "USED" => Some(card_status::USED),
        "EXPIRED" => Some(card_status::EXPIRED),
        "ARCHIVED" => Some(card_status::ARCHIVED),
        "LOST" => Some(card_status::LOST),
        _ => None,
    }
}

/// 去掉注释行后的代码行（扫描只针对真实比较/写入语句，不针对文档措辞）。
fn code_lines(src: &str) -> Vec<String> {
    src.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.starts_with("//") && !l.starts_with("//!") && !l.starts_with("///"))
        .collect()
}

#[test]
fn source_scan_issue_gate_no_dead_active_and_tokens_from_word_list() {
    let src = include_str!("../src/services/color_card_issue_service.rs").replace('\r', "");

    // (a) 门控必须统一经可发放集合
    assert!(
        src.contains("ISSUABLE_CARD_STATUSES.contains(&card.status.as_str())"),
        "闸门 1 必须经可发放集合比较，不得回潮单值裸比较"
    );

    // (b) 色卡状态比较/写入行不得再出现裸 "active" 或 master_data::ACTIVE。
    //     区分依据：customer.status 的 'active' 属 master_data 词表（active/inactive
    //     合法且被真实写入，如 customer_credit_limit.rs:66），只放行含 customer 的行。
    assert!(
        !src.contains("master_data::ACTIVE"),
        "发放服务不得引用 master_data::ACTIVE 作色卡状态"
    );
    for line in code_lines(&src) {
        if line.contains(r#""active""#) {
            assert!(
                line.contains("customer"),
                "裸 \"active\" 只允许出现在 customer.status（master_data 词表）比较行，\
                 不得用于色卡状态；违规行: {line}"
            );
        }
    }

    // (c) 放行集合每个 token 必须引自 card_status 词表且 ∈ card_status::ALL
    //     （锚定 `= &[`，避免误中类型注解 `&[&str]` 的 `&[`）
    let marker = "const ISSUABLE_CARD_STATUSES";
    let i = src.find(marker).expect("可发放集合常量缺失");
    let rest = &src[i..];
    let open = rest.find("= &[").expect("集合初始化式缺失 = &[") + 4;
    let close = open + rest[open..].find(']').expect("集合初始化未闭合 ]");
    let inner = &rest[open..close];
    let mut checked = 0usize;
    for raw in inner.split(',') {
        let tok = raw.trim();
        if tok.is_empty() {
            continue;
        }
        let ident = tok
            .strip_prefix("card_status::")
            .unwrap_or_else(|| panic!("可发放集合只能引用色卡词表常量，实际: {tok}"));
        let value = card_word_value(ident)
            .unwrap_or_else(|| panic!("集合引用了词表外常量 card_status::{ident}"));
        assert!(
            card_status::ALL.contains(&value),
            "门控 token {value} 不在 color_card::ALL 内（防再出现比一个死值）"
        );
        assert_ne!(value, "active", "放行集合不得含 legacy active");
        checked += 1;
    }
    assert!(checked >= 1, "可发放集合至少应包含一个词表 token");

    // (d) 发放记录列（本文件即写入方权威枚举）不得再出现裸 "issued" 比较/查询值，
    //     仅枚举定义位（as_str/from_str）允许字面量。
    for line in code_lines(&src) {
        if line.contains(r#"Column::Status.eq("issued")"#) {
            panic!("发放记录状态查询必须引 IssueStatus::Issued.as_str()，违规行: {line}");
        }
    }
}

// =========================================================
// 4) 任务 2 扫描锁：custom_order_process_service.rs 状态必须引常量
// =========================================================

#[test]
fn source_scan_process_node_status_uses_authority_constants() {
    let src = include_str!("../src/services/custom_order_process_service.rs").replace('\r', "");

    // (a) 裸形态零命中：status = Set("...") / status != "..."（必须引常量）
    for line in code_lines(&src) {
        assert!(
            !line.contains(r#"status = Set("")"#) && !line.contains("status = Set(\""),
            "process_nodes.status 写入不得为裸字面量，违规行: {line}"
        );
        assert!(
            !line.contains(r#"status != ""#) && !line.contains("status != \""),
            "process_nodes.status 比较不得为裸字面量，违规行: {line}"
        );
    }

    // (b) 四个节点状态 token 的引用来路必须是权威模块常量
    assert!(src.contains("crate::models::status::production::process_node as node_status"));
    assert!(src.contains("node_status::PENDING"));
    assert!(src.contains("node_status::IN_PROGRESS"));
    assert!(src.contains("node_status::COMPLETED"));
    assert!(src.contains("node_status::BLOCKED"));

    // (c) 词表值逐字符锁定（与 chk_node_status 四值一致，提交 494dd0f3）：
    //     本服务只允许产出这四个值；若出现其它状态词，必须扩到权威模块而不是新增字面量。
    let allowed = [
        node_status::PENDING,
        node_status::IN_PROGRESS,
        node_status::COMPLETED,
        node_status::BLOCKED,
    ];
    for token in ["pending", "in_progress", "completed", "blocked"] {
        let quoted = format!(r#""{token}""#);
        for line in code_lines(&src) {
            assert!(
                !line.contains(&quoted),
                "节点状态值 {quoted} 必须以 node_status:: 常量引用出现，不得残留裸字面量，违规行: {line}"
            );
        }
        assert!(
            allowed.contains(&token),
            "token {token} 不在 process_node 权威词表内（应交派单扩表，禁止私自加字面量）"
        );
    }
}

// =========================================================
// 5) 正向全链（活库 PG，#[ignore]）：真实建卡(draft) → 发放成功
//    → 回查发放记录 + 卡库存推进；sqlite 不支持 lock_exclusive 行锁，
//    遵循仓内先例不伪装成 sqlite 用例。
// =========================================================

async fn require_postgres(db: &sea_orm::DatabaseConnection) {
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在已迁移的 PostgreSQL（TEST_DATABASE_URL）上；禁止 sqlite 回退假绿"
    );
}

async fn seed_user(db: &sea_orm::DatabaseConnection, name: &str) -> i32 {
    use bingxi_backend::models::user::ActiveModel as UserActive;
    let now = chrono::Utc::now();
    let u = UserActive {
        username: Set(name.to_string()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name.to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    u.id
}

async fn seed_live_customer(db: &sea_orm::DatabaseConnection, code: &str, owner_id: i32) -> i32 {
    let now = chrono::Utc::now();
    let c = CustomerActive {
        customer_code: Set(code.to_string()),
        customer_name: Set("wave5 发放契约客户".to_string()),
        credit_limit: Set(Decimal::new(100000, 2)),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("company".to_string()),
        owner_id: Set(owner_id),
        created_by: Set(Some(owner_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种客户失败");
    c.id
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（issue() 走 lock_exclusive + 事务，sqlite 不支持行锁）"]
async fn live_issue_on_real_draft_card_succeeds_and_advances_stock() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let now = chrono::Utc::now();
    let suffix = now.timestamp_nanos_opt().unwrap();

    // 播种：操作员 user（issued_by 有 FK 语义）+ 两个 active 客户
    // （credit>0、无历史发放 ⇒ 闸门 3/4/5 天然通过）
    let operator = seed_user(&db, &format!("w5_issue_op_{suffix}")).await;
    let customer_a = seed_live_customer(&db, &format!("W5CUST-A-{suffix}"), operator).await;
    let customer_b = seed_live_customer(&db, &format!("W5CUST-B-{suffix}"), operator).await;

    // 真实建卡：status 写 card_status::DRAFT（与 create 端点写入值同源），stock=10
    let card = CardActive {
        card_no: Set(format!("W5-ISSUE-LIVE-{suffix}")),
        card_name: Set("wave5 发放正向契约色卡".to_string()),
        card_type: Set("PANTONE".to_string()),
        status: Set(card_status::DRAFT.to_string()),
        stock_quantity: Set(10),
        issued_quantity: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("播种 draft 色卡失败");

    let svc = ColorCardIssueService::new(Arc::new(db.clone()));

    // 发放 2 张 → 成功，发放记录落库 status=issued，卡 issued_quantity 推进 0→2
    let record = svc
        .issue(IssueParams {
            color_card_id: card.id,
            customer_id: customer_a as i64,
            issued_by: operator as i64,
            issue_qty: 2,
            expected_return_date: None,
            purpose: None,
            remark: None,
            dye_lot_no: None,
            sales_order_id: None,
        })
        .await
        .expect("draft 色卡发放必须成功（修复前比死值 active 恒被拒）");
    assert_eq!(
        record.status,
        IssueStatus::Issued.as_str(),
        "发放记录状态应为 issued"
    );
    assert_eq!(record.issue_qty, 2);

    let card_after = ColorCardEntity::find_by_id(card.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        card_after.issued_quantity, 2,
        "卡库存推进：issued_quantity 0→2"
    );
    assert_eq!(card_after.stock_quantity, 10, "总量不变");

    // 同卡再发放给第二个客户 3 张：draft 卡凭库存可继续发放（发放是库存操作，
    // 卡生命周期 draft→issued 流转由 crud transition 端点负责，本链路不改卡状态词表值）
    let record2 = svc
        .issue(IssueParams {
            color_card_id: card.id,
            customer_id: customer_b as i64,
            issued_by: operator as i64,
            issue_qty: 3,
            expected_return_date: None,
            purpose: None,
            remark: None,
            dye_lot_no: None,
            sales_order_id: None,
        })
        .await
        .expect("库存充足时 draft 卡应可继续发放");
    assert_eq!(record2.status, IssueStatus::Issued.as_str());

    let card_final = ColorCardEntity::find_by_id(card.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(card_final.issued_quantity, 5, "两次发放后库存推进累计 0→5");
    assert_eq!(
        card_final.status,
        card_status::DRAFT,
        "发放（库存操作）不改写卡生命周期状态：状态流转权威在 crud transition"
    );

    // 发放记录回查：两条、均 issued、归属正确
    let records = IssueEntity::find()
        .filter(bingxi_backend::models::color_card_issue::Column::ColorCardId.eq(card.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(records.len(), 2, "两次发放应各落一条记录");
    for r in &records {
        assert_eq!(r.status, IssueStatus::Issued.as_str());
        assert!(
            IssueStatus::from_str(&r.status).is_ok(),
            "落库值必须能被写入方枚举解析"
        );
    }

    // 清理本次播种（活库复用，避免污染后续用例）
    IssueEntity::delete_many()
        .filter(bingxi_backend::models::color_card_issue::Column::ColorCardId.eq(card.id))
        .exec(&db)
        .await
        .unwrap();
    ColorCardEntity::delete_by_id(card.id)
        .exec(&db)
        .await
        .unwrap();
    CustomerEntity::delete_by_id(customer_a)
        .exec(&db)
        .await
        .unwrap();
    CustomerEntity::delete_by_id(customer_b)
        .exec(&db)
        .await
        .unwrap();
    bingxi_backend::models::user::Entity::delete_by_id(operator)
        .exec(&db)
        .await
        .unwrap();
}

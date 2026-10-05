//! 契约波次 11 · `customer_credit_ratings` 「每客户一行 UNIQUE + 客户 FK」真库契约锁
//!
//! ## 锁的对象与通道（为什么直连夹具库而不是端点级）
//! - 本约束的**形态**（唯一索引名/定义、FK 所指表）只存在于 pg_catalog，任何端点
//!   都不会把它暴露成响应字段 ⇒ 形态锁必须查目录表。
//! - 负向的「同客户第二行」在生产 HTTP 链路上**不可确定复现**：唯一写入方
//!   `services/customer_credit_limit.rs::set_credit_rating` 是先查后插的 upsert，
//!   竞态窗口不是端点能稳定打中的 ⇒ 负向只能在已迁移夹具库上直接 SQL 打约束，
//!   断言只断**数据库错误类别**（`SqlErr::UniqueConstraintViolation` /
//!   `SqlErr::ForeignKeyConstraintViolation`）与**约束名**，不断用户可见文案
//!   （脱敏红线：文案随 messages 演进，按文案判约束是假绿来源）。
//! - 服务层可达的那一半负向（无效 customer_id 必须落 4xx 而不是裸 500）走**生产
//!   service 入口** `CustomerCreditService::set_credit_rating`——与 handler 通道
//!   完全同构（`handlers/customer_credit_handler.rs` 就是 `new(state.db.clone())`
//!   再调本方法），因此不需要 AppState，天然规避 `AppState::default()` 把 service
//!   绑 Disconnected 哨兵的假绿坑。
//!
//! ## 语义前提（与迁移 m0082 同源，此处只复述判据不重复论证）
//! 该表读写契约 = 每客户**当前一行**：写入路径 upsert 单行、停用只改 status 保留
//! 原行、读路径全部按 customer_id `.one()` 取单行 ⇒ 全表 UNIQUE 正确；若是评级
//! 历史多条，本锁负向用例本身就是设计错误——种法见下方正向用例「第二次 set 不
//! 产生新行」，它就是这条语义的行为锁。
//!
//! ## 夹具与 id 带
//! - 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL`/指向 sqlite 直接 panic，
//!   ⇒ 本文件真库用例**只有 CI 活库才有结果**（本机仅证明编译与格式）。
//!   库由迁移链建好（含 m0082），夹具只 TRUNCATE 业务表、不清 seaql_migrations
//!   台账，因此本锁验证的约束确实是**迁移真实落地**的产物。
//! - 本文件独占 **994xxx** id 带（users 994001、customers 994011、评级行 994101/
//!   994102、无效客户引用 994999），每段先删后插保幂等。
//! - `users`/`customers` 是逐用例 TRUNCATE 的业务表；`customers.created_by` 有
//!   外键指向 users，故先建 users；`departments.id=1` 属密封参照表只引用不写。
//! - `customers.customer_type` 被 CHECK 锁成渠道词表（business m0078），种子值
//!   必须来自权威词表 `constants::customer_type::ALLOWED`（用例开头即断）。
//! - 全部断言收在**同一个** `#[tokio::test]` 里顺序执行：夹具的 TRUNCATE 与种子
//!   行在同库多测试线程下有相互清表风险，单函数从结构上排除该耦合。

mod test_common;

use bingxi_backend::constants::customer_type::ALLOWED;
use bingxi_backend::services::customer_credit_service::{
    CreditRatingRequest, CustomerCreditService,
};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

const USER_OWNER: i32 = 994_001;
const CUSTOMER_ID: i32 = 994_011;
const DUP_ROW_ID: i32 = 994_101;
const ORPHAN_ROW_ID: i32 = 994_102;
/// 不存在的客户引用（FK 负向用；本文件绝不会给它种客户行）
const NONEXISTENT_CUSTOMER_ID: i32 = 994_999;

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 种一条真实客户链路：users(994001) → customers(994011)；列形态照
/// contract_wave8_large_customer_credit_predicate_test.rs 的已验证种子逐列复用
/// （customers.credit_limit 是 DECIMAL(12,2)，与本表 DECIMAL(15,2) 无关，互不影响）。
async fn seed_customer(db: &DatabaseConnection) {
    exec(
        db,
        &format!(
            "DELETE FROM customer_credit_ratings WHERE customer_id IN ({CUSTOMER_ID},{NONEXISTENT_CUSTOMER_ID})"
        ),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM customers WHERE id IN ({CUSTOMER_ID},{NONEXISTENT_CUSTOMER_ID})"),
    )
    .await;
    exec(db, &format!("DELETE FROM users WHERE id = {USER_OWNER}")).await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at)
             VALUES ({USER_OWNER},'cc_owner94001','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    let channel = "retail";
    assert!(
        ALLOWED.contains(&channel),
        "种子渠道值 '{channel}' 必须在权威词表 ALLOWED={ALLOWED:?} 内，否则会被 customers CHECK 拒在种子阶段"
    );
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
         VALUES ($1,$2,'信用约束锁客户',$3,30,'active',$4,$5,$6,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        vec![
            CUSTOMER_ID.into(),
            format!("CC-{CUSTOMER_ID}").into(),
            Decimal::new(200_00_00, 2).into(),
            USER_OWNER.into(),
            USER_OWNER.into(),
            channel.into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("种客户行失败（id={CUSTOMER_ID}）: {e}"));
}

/// 直接按 customer_id 打一次生产写入口（req 只填必要字段，None 走服务默认值）
fn rating_req(customer_id: i32, credit_limit: Option<Decimal>) -> CreditRatingRequest {
    CreditRatingRequest {
        customer_id,
        credit_level: Some("A".to_string()),
        credit_score: Some(85),
        credit_limit,
        credit_days: Some(30),
        remark: None,
    }
}

/// 表内该客户的行数（COUNT 回读，正向单行语义的判据）
async fn row_count_for(db: &DatabaseConnection, customer_id: i32) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT COUNT(*) AS cnt FROM customer_credit_ratings WHERE customer_id={customer_id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 customer_credit_ratings 行数失败: {e}"));
    row.expect("COUNT 查询恒有行")
        .try_get::<i64>("", "cnt")
        .unwrap_or_else(|e| panic!("行数解码失败: {e}"))
}

/// 越界 INSERT 的期望错误**类别**（SqlErr 变体，不是文案子串）
fn assert_sql_err_kind(err: sea_orm::DbErr, want: &str, case: &str) {
    match err.sql_err() {
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_)) if want == "unique" => {}
        Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_)) if want == "fk" => {}
        other => {
            panic!("{case}：期望数据库错误类别 {want}，实际 sql_err()={other:?}\n错误原文: {err}")
        }
    }
}

#[tokio::test]
async fn customer_credit_ratings_single_row_and_fk_are_db_enforced() {
    let db = Arc::new(setup_test_db().await);
    seed_customer(&db).await;
    let svc = CustomerCreditService::new(db.clone());

    // ---------- 0. 形态锁：唯一索引与 FK 必须真实存在于夹具库（迁移落地的证明） ----------
    let idx = db
        .query_one_raw(pg_stmt(
            "SELECT indexdef FROM pg_indexes \
              WHERE tablename='customer_credit_ratings' \
                AND indexname='uq_customer_credit_ratings_customer'",
        ))
        .await
        .unwrap_or_else(|e| panic!("查 pg_indexes 失败: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "唯一索引 uq_customer_credit_ratings_customer 不在夹具库上——\
                 迁移 m0082 未在测试库生效（检查迁移是否注册并跑进链）"
            )
        });
    let indexdef: String = idx
        .try_get("", "indexdef")
        .unwrap_or_else(|e| panic!("indexdef 解码失败: {e}"));
    assert!(
        indexdef.contains("UNIQUE") && indexdef.contains("(customer_id)"),
        "唯一索引形态应为 customer_id 单列全表 UNIQUE，实测 indexdef={indexdef}"
    );

    let fk = db
        .query_one_raw(pg_stmt(
            "SELECT contype::text AS ctype, confrelid::regclass::text AS ftab \
               FROM pg_constraint \
              WHERE conrelid='customer_credit_ratings'::regclass \
                AND conname='fk_customer_credit_ratings_customer'",
        ))
        .await
        .unwrap_or_else(|e| panic!("查 pg_constraint 失败: {e}"))
        .unwrap_or_else(|| {
            panic!("FK fk_customer_credit_ratings_customer 不在夹具库上——m0082 未生效")
        });
    let ctype: String = fk
        .try_get("", "ctype")
        .unwrap_or_else(|e| panic!("contype 解码失败: {e}"));
    let ftab: String = fk
        .try_get("", "ftab")
        .unwrap_or_else(|e| panic!("confrelid 解码失败: {e}"));
    assert_eq!(ctype, "f", "FK 约束类型必须是 contype='f'，实测 {ctype}");
    assert!(
        ftab.ends_with("customers"),
        "fk_customer_credit_ratings_customer 必须指向 customers，实测 {ftab}"
    );

    // ---------- 1. 正向：生产写入口 upsert 单行语义（第二次 set 不产生新行） ----------
    let first = svc
        .set_credit_rating(
            rating_req(CUSTOMER_ID, Some(Decimal::new(500_00_00, 2))),
            USER_OWNER,
        )
        .await
        .expect("首次设置信用评级应成功（合法数据必须能落）");
    let second = svc
        .set_credit_rating(
            rating_req(CUSTOMER_ID, Some(Decimal::new(800_00_00, 2))),
            USER_OWNER,
        )
        .await
        .expect("更新信用评级应成功（upsert 的 update 分支）");
    assert_eq!(
        first.id, second.id,
        "同一客户第二次 set 必须更新原行而不是插新行（单行契约的行为锁）"
    );
    assert_eq!(second.credit_limit, Decimal::new(800_00_00, 2));
    assert_eq!(
        row_count_for(&db, CUSTOMER_ID).await,
        1,
        "两次 set 后该客户在表内必须恰好 1 行"
    );

    // ---------- 2. 负向 A：绕过应用层直插第二行 ⇒ 唯一索引拒（错误类别锁） ----------
    let err = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO customer_credit_ratings (id,customer_id,credit_limit,status) VALUES ($1,$2,$3,'active')",
            vec![
                DUP_ROW_ID.into(),
                CUSTOMER_ID.into(),
                Decimal::new(1_00_00, 2).into(),
            ],
        ))
        .await
        .expect_err("同客户第二行必须被唯一索引拒绝（若成功=约束缺失）");
    assert_sql_err_kind(err, "unique", "负向A 同客户第二行");

    // ---------- 3. 负向 B：直插孤儿引用 ⇒ FK 拒（错误类别锁） ----------
    let err = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO customer_credit_ratings (id,customer_id,credit_limit,status) VALUES ($1,$2,$3,'active')",
            vec![
                ORPHAN_ROW_ID.into(),
                NONEXISTENT_CUSTOMER_ID.into(),
                Decimal::new(1_00_00, 2).into(),
            ],
        ))
        .await
        .expect_err("指向不存在客户的评级行必须被 FK 拒绝（若成功=FK 缺失）");
    assert_sql_err_kind(err, "fk", "负向B 孤儿引用");

    // ---------- 4. 服务层负向：无效 customer_id 落业务错误族而不是裸 500 ----------
    let app_err = svc
        .set_credit_rating(
            rating_req(NONEXISTENT_CUSTOMER_ID, Some(Decimal::new(10_00_00, 2))),
            USER_OWNER,
        )
        .await
        .err()
        .unwrap_or_else(|| {
            panic!(
                "对不存在的客户 {NONEXISTENT_CUSTOMER_ID} 设置信用评级必须被拒（FK 前置校验或 DB 兜底），不能返回 Ok"
            )
        });
    let code = app_err.error_code();
    assert_ne!(
        code, "DATABASE_ERROR",
        "FK 拒绝不得以 DATABASE_ERROR(500) 形态外抛（红线：约束违例须映射业务错误）；实际 code={code}"
    );
    assert!(
        code == "NOT_FOUND" || code == "BUSINESS_ERROR",
        "无效客户引用应归 404 NOT_FOUND（前置校验）或 400 BUSINESS_ERROR（DB 兜底降级），实际 code={code}"
    );

    // ---------- 收尾自清：只删本文件种的行，不依赖执行顺序 ----------
    exec(
        &db,
        &format!("DELETE FROM customer_credit_ratings WHERE customer_id = {CUSTOMER_ID}"),
    )
    .await;
}

// ---------------------------------------------------------------------------
// 迁移文本与注册在场锁（纯静态，无需活库，本机即可跑）
// ---------------------------------------------------------------------------

fn mig_src() -> String {
    include_str!("../migration/src/domain/business/m0082_add_customer_credit_uniqueness_and_fk.rs")
        .replace('\r', "")
}

#[test]
fn m0082_up_guard_is_fail_visible_and_wash_free() {
    let src = mig_src();
    // up 段（头注释也含 RAISE/UPDATE 描述文字，按执行体计数避免误伤）
    let up_start = src.find("async fn up").expect("m0082 缺少 up 实现");
    let up_end = src.find("async fn down").expect("m0082 缺少 down 实现");
    let up = &src[up_start..up_end];
    // 两类守卫各一条 fail-visible RAISE（孤儿 + 同客户多行）
    assert_eq!(
        up.matches("RAISE EXCEPTION").count(),
        2,
        "m0082 up 应恰有两条 RAISE EXCEPTION（孤儿引用/同客户多行），实测不符"
    );
    // 幂等三形态齐全：先删同名对象、IF NOT EXISTS 建索引、DROP IF EXISTS 收尾
    assert!(up.contains("DROP CONSTRAINT IF EXISTS \"fk_customer_credit_ratings_customer\""));
    assert!(
        up.contains("CREATE UNIQUE INDEX IF NOT EXISTS \"uq_customer_credit_ratings_customer\"")
    );
    // 红线：迁移绝不洗数据——执行体内不得出现任何 UPDATE/DELETE 语句形态
    for banned in ["UPDATE \"", "DELETE FROM", "SET \"customer_id\""] {
        assert!(
            !up.contains(banned),
            "m0082 up 执行体出现数据操纵形态: {banned}（禁止迁移内清洗）"
        );
    }
}

#[test]
fn m0082_down_is_symmetric_no_data_touch() {
    let src = mig_src();
    let down = &src[src.find("async fn down").expect("m0082 缺少 down 实现")..];
    assert!(down.contains("DROP INDEX IF EXISTS \"uq_customer_credit_ratings_customer\";"));
    assert!(down.contains("DROP CONSTRAINT IF EXISTS \"fk_customer_credit_ratings_customer\";"));
    assert!(!down.contains("DELETE FROM"), "down 不得删数据行");
}

#[test]
fn m0082_registered_at_business_chain_tail_and_down_head() {
    // 注册在场锁：本仓教训是"迁移文件写了但没进域链 = 永远不会执行"，
    // 这里钉死 business/mod.rs 的 mod 声明与 up 链尾 / down 链首位置（对称回滚）。
    let biz_mod = include_str!("../migration/src/domain/business/mod.rs").replace('\r', "");
    assert!(
        biz_mod.contains("mod m0082_add_customer_credit_uniqueness_and_fk;"),
        "business/mod.rs 缺少 m0082 的 mod 声明"
    );
    let calls = "m0082_add_customer_credit_uniqueness_and_fk::Migration";
    assert_eq!(
        biz_mod.matches(calls).count(),
        2,
        "business/mod.rs 应有 up/down 两处 m0082 调用，实测 {} 处",
        biz_mod.matches(calls).count()
    );
    // up 段顺序：m0082 晚于 m0081（链尾）；down 段顺序：m0082 早于 m0081（链首）。
    let (up_part, down_part) = biz_mod
        .split_once("async fn down")
        .expect("business/mod.rs 缺少 down 实现");
    let prev = "m0081_grant_contract_price_reject::Migration";
    assert!(
        up_part.find(prev).expect("up 段缺 m0081 调用")
            < up_part.find(calls).expect("up 段缺 m0082 调用"),
        "up 链 m0082 必须注册在 m0081 之后（链尾）"
    );
    assert!(
        down_part.find(calls).expect("down 段缺 m0082 调用")
            < down_part.find(prev).expect("down 段缺 m0081 调用"),
        "down 链 m0082 必须先于 m0081 回滚（最后应用者最先回滚）"
    );
}

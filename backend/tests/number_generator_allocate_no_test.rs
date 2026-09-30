//! 任务 #144：`DocumentNumberGenerator::allocate_no`（逐位占用探测）真实行为契约测试
//!
//! 覆盖 `backend/src/utils/number_generator.rs` 中取号核心 `allocate_no` 的公开入口
//! （`generate_no` / `generate_no_with_txn`），全部打**真实 PostgreSQL 真实表**
//! （`customers.customer_code`，UNIQUE），不 mock、不用 sqlite 假绿：
//! 1. 号段有空洞（存量硬删除）：基数 `count+1` 落到被占用号位时逐位探测跳过，
//!    不产生 23505、不返回重复号；
//! 2. 同事务内「取号→插入→取号→插入」得到连续且不冲突的号（探测可见本事务
//!    未提交插入）；
//! 3. 号段被连续占满 `NO_PROBE_MAX=100` 个候选位：显式报错
//!    （`BusinessErrorDisplayable`，真实文案可外显），不静默跳过、不无界循环；
//! 4. 宽度上限（3 位流水到 999 后溢出）：如实记录**当前实现**行为——直接产出
//!    超宽的 4 位流水号（`1000`），既不报错也不自动扩宽。期望（待修复讨论）：
//!    应在溢出边界显式报错或按约定扩展位数，而不是静默改变单号总长度——
//!    下游按定长解析单号的逻辑会踩坑。此断言锁定现状，行为变更时本测试应
//!    同步改并留下痕迹。
//!
//! 全部用例需要已迁移 PG（`pg_advisory_xact_lock` 为 PG 专属），故整体
//! `#[ignore]`，由 CI 专用 job（`--run-ignored only`，先 `bingxi migrate run`）执行；
//! 依赖环境变量 `TEST_DATABASE_URL`。缺失即显式 panic，不静默回退 sqlite。
//!
//! 已知微小竞态：用例内多次使用 `Utc::now().format("%Y%m%d")`，跨 UTC 午夜
//! 运行时日期变化会导致断言失败（专用 job 低概率事件，失败为显式红而非假绿）。

use bingxi_backend::models::customer;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::number_generator::DocumentNumberGenerator;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};

/// 活库连接：显式要求 TEST_DATABASE_URL（已迁移 PostgreSQL），缺失即报错而非静默回退
async fn live_db() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("活库用例必须设置 TEST_DATABASE_URL（指向已跑完迁移的 PostgreSQL）");
    sea_orm::Database::connect(&url)
        .await
        .expect("测试夹具：活库连接失败")
}

fn today() -> String {
    Utc::now().format("%Y%m%d").to_string()
}

/// 用测试前缀 + 日期 + 指定位构造单号字面量（宽度 3，零填充）
fn code(pfx: &str, seq: u32) -> String {
    format!("{}{}{:03}", pfx, today(), seq)
}

/// 构造可插入的真实客户行（字段集与 contract_wave1_crm_lead_delete_reference_test.rs
/// 在活库上验证过的最小可用集一致，其余列走 DB 默认/可空）
fn make_active(no: String) -> customer::ActiveModel {
    let now = Utc::now();
    customer::ActiveModel {
        customer_code: Set(no.clone()),
        customer_name: Set(format!("取号探测客户 {no}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(100),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

async fn seed(db: &DatabaseConnection, codes: &[String]) {
    for c in codes {
        make_active(c.clone())
            .insert(db)
            .await
            .unwrap_or_else(|e| panic!("造数插入 {c} 失败: {e}"));
    }
}

async fn cleanup(db: &DatabaseConnection, pfx: &str) {
    customer::Entity::delete_many()
        .filter(customer::Column::CustomerCode.starts_with(pfx))
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("清理前缀 {pfx} 失败: {e}"));
}

/// 号段有空洞（001、003 存在，002 被硬删除）：
/// 基数 count(=2)+1 = 3 落到被占用的 003，逐位探测应跳过并给出 004，
/// 而不是返回 003 撞 23505、也不是回填 002（当前实现基数语义：不回填）。
#[tokio::test]
#[ignore = "需已迁移 PG（pg_advisory_xact_lock），由专用 job（--run-ignored only）执行"]
async fn allocate_skips_occupied_seq_over_deleted_hole() {
    let pfx = "NGA1";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    seed(&db, &[code(pfx, 1), code(pfx, 3)]);

    let no = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .expect("空洞形态取号必须成功（探测跳过被占用位），不得 23505/报错");
    assert_eq!(
        no,
        code(pfx, 4),
        "001/003 存在时（002 硬删除空洞），期望探测跳过被占用的 003 得到 004，实际 {no}"
    );

    cleanup(&db, pfx).await;
}

/// 同一事务内「取号→插入」连续三轮：探测查询与插入在同一连接同一事务，
/// 未提交行对本事务可见，三轮必须得到连续的 004/005/006，互不撞号。
/// 最后整体回滚（回滚后不残留数据；开头 cleanup 已兜住历史残留）。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock + 事务内可见性），由专用 job（--run-ignored only）执行"]
async fn allocate_is_consecutive_for_repeated_generate_insert_in_one_txn() {
    let pfx = "NGA2";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    seed(&db, &[code(pfx, 1), code(pfx, 3)]);

    let txn = db.begin().await.expect("开启外层事务失败");
    let mut got: Vec<String> = Vec::new();
    for round in 1..=3 {
        let no = DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            pfx,
            customer::Entity,
            customer::Column::CustomerCode,
        )
        .await
        .unwrap_or_else(|e| panic!("第 {round} 轮事务内取号失败: {e:?}"));
        make_active(no.clone())
            .insert(&txn)
            .await
            .unwrap_or_else(|e| panic!("第 {round} 轮插入 {no} 失败: {e}"));
        got.push(no);
    }
    assert_eq!(
        got,
        vec![code(pfx, 4), code(pfx, 5), code(pfx, 6)],
        "事务内取号→插入→取号必须连续不撞号（探测需看见本事务未提交插入）"
    );
    txn.rollback().await.expect("回滚外层事务失败");

    cleanup(&db, pfx).await;
}

/// 号段被占满：存量 100 行占据 101..200，基数 count(=100)+1 = 101 起
/// 连续 100 个候选位全部被占用 → 探测到 `NO_PROBE_MAX` 上限后**显式报错**
/// （BusinessErrorDisplayable，出参携带真实文案），绝不静默跳到 201 或裸 500。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock），由专用 job（--run-ignored only）执行"]
async fn allocate_errors_explicitly_when_segment_fully_occupied() {
    let pfx = "NGA3";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    let codes: Vec<String> = (101..=200).map(|n| code(pfx, n)).collect();
    seed(&db, &codes);

    let err = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .expect_err("连续 100 个候选位全被占用时必须显式报错（防止无界循环/静默跳号）");
    match &err {
        // 注意：必须是 business_displayable（真实文案可外显）；若是 business
        // （出参脱敏成"业务处理失败"）此处会变红，如实暴露变体误用。
        AppError::BusinessErrorDisplayable(m) => {
            assert!(
                m.contains("连续 100 个号位被占用"),
                "报错文案应说明号段占满与探测上限，实际文案: {m}"
            );
            assert!(
                m.contains(&format!("{}{}", pfx, today())),
                "报错文案应带上被占满的号段前缀，便于数据侧排查，实际文案: {m}"
            );
        }
        other => panic!("期望 AppError::BusinessErrorDisplayable，实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    cleanup(&db, pfx).await;
}

/// 宽度上限（3 位流水 999 溢出）——如实记录当前实现结果：
/// 存量 001..999，count+1 = 1000，`{:03}` 只补齐不截断，
/// 当前实现**静默产出超宽 4 位流水号** `{pfx}{date}1000`，不报错、不自动扩宽约定。
/// 期望（写进汇报的缺陷候选）：溢出应有显式行为（报错或声明式扩容），
/// 定长单号消费方（打印/对账解析）会被这种号打穿。行为若被修复，本断言
/// 应随之更新并留下变更痕迹——禁止悄悄放宽。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock），由专用 job（--run-ignored only）执行"]
async fn allocate_overflow_at_width_cap_current_behavior_is_silent_widening() {
    let pfx = "NGA4";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    let codes: Vec<String> = (1..=999).map(|n| code(pfx, n)).collect();
    seed(&db, &codes);

    let no = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .unwrap_or_else(|e| panic!("999 之后取号当前实现应静默产出超宽号而非报错，实际: {e:?}"));

    let expected = format!("{}{}1000", pfx, today());
    assert_eq!(
        no, expected,
        "宽度上限溢出：当前实现直接给出 4 位流水（超宽）号，锁定现状；修复为显式报错/扩容时更新本断言"
    );
    // 同步锁定"超宽"这一事实本身：号尾流水部分确实超过了 width=3 的约定
    let tail = &no[pfx.len() + 8..];
    assert_eq!(
        tail.len(),
        4,
        "当前实现产出的流水段长度 {tail}（期望记录用：溢出后变为 4 位）"
    );

    cleanup(&db, pfx).await;
}

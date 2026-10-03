//! 任务 #144：`DocumentNumberGenerator::allocate_no`（基数与真实号段同源 + 占用探测）
//! 真实行为契约测试
//!
//! 覆盖 `backend/src/utils/number_generator.rs` 中取号核心 `allocate_no` 的公开入口
//! （`generate_no` / `generate_no_with_txn`），全部打**真实 PostgreSQL 真实表**
//! （`customers.customer_code`，UNIQUE），不 mock、不用 sqlite 假绿：
//! 1. 基数与真实号段同源（提交 4517b1de 起，基数 = `max(当日真实单号后缀流水)+1`，
//!    **不再是** `count+1`）：空洞存量（001/003，002 硬删）下起点落在号段末尾后
//!    的空闲位 004，不回填空洞（NGA1）；
//! 2. 同事务内「取号→插入→取号→插入」得到连续且不冲突的号（投影与探测均可见
//!    本事务未提交插入）（NGA2）；
//! 3. 并发抢占候选号：起点 201 被他会话先落库后，下一次取号必须递增到 202
//!    成功，绝不静默返回已被占用的号，表内无重号（NGA3）；
//! 4. 探测被占到达上限 `NO_PROBE_MAX=100`：显式报错
//!    （`BusinessErrorDisplayable`，真实文案可外显），不静默跳过、不无界循环。
//!    新基数语义下"静态占满 101..200 从 101 探测"已不可达（起点恒为空闲位），
//!    上限路径的确定性构造改为病态饱和号段（后缀 = u64::MAX，见 NGA5）；
//! 5. 宽度上限（3 位流水到 999 后溢出）：如实记录**当前实现**行为——直接产出
//!    超宽的 4 位流水号（`1000`），既不报错也不自动扩宽。期望（待修复讨论）：
//!    应在溢出边界显式报错或按约定扩展位数，而不是静默改变单号总长度——
//!    下游按定长解析单号的逻辑会踩坑。此断言锁定现状，行为变更时本测试应
//!    同步改并留下痕迹（NGA4）。
//!
//! 另有一条**不依赖数据库、不 #[ignore]** 的源码扫描锁
//! （`number_cardinal_must_not_regress_to_count_based`）：禁止基数实现回潮为
//! 按行数 `count+1` 起探测，随常规 CI 分片 job 常跑，不等活库 job。
//!
//! 数据库用例需要已迁移 PG（`pg_advisory_xact_lock` 为 PG 专属），故逐条
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

/// 空洞存量（001、003 存在，002 被硬删除）：基数取真实号段末尾
/// max(001,003)+1 = 004，直接落在空闲位且**不回填** 002 空洞。
/// （旧 count+1 语义下为 2+1=3 撞占用后探测到 004，结果相同但成因不同；
/// 本断言在新旧语义下同值，保留用于锁"空洞不回填"这条不变契约。）
#[tokio::test]
#[ignore = "需已迁移 PG（pg_advisory_xact_lock），由专用 job（--run-ignored only）执行"]
async fn allocate_starts_after_max_seq_and_never_backfills_hole() {
    let pfx = "NGA1";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    seed(&db, &[code(pfx, 1), code(pfx, 3)]).await;

    let no = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .expect("空洞形态取号必须成功（基数落在号段末尾后的空闲位），不得 23505/报错");
    assert_eq!(
        no,
        code(pfx, 4),
        "001/003 存在（002 硬删空洞）时，基数应为 max+1=004 且不回填 002，实际 {no}"
    );

    cleanup(&db, pfx).await;
}

/// 同一事务内「取号→插入」连续三轮：基数投影与占用探测均在本事务连接上，
/// 未提交行对自身可见（软删/本事务插入都参与号段基数，与真实号段同源），
/// 三轮必须得到连续的 004/005/006，互不撞号。
/// 最后整体回滚（回滚后不残留数据；开头 cleanup 已兜住历史残留）。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock + 事务内可见性），由专用 job（--run-ignored only）执行"]
async fn allocate_is_consecutive_for_repeated_generate_insert_in_one_txn() {
    let pfx = "NGA2";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    seed(&db, &[code(pfx, 1), code(pfx, 3)]).await;

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

/// 并发抢占候选号（原 NGA3"号段占满显式报错"在新基数语义下的等价守卫）：
/// 存量 101..=200 占满低段 → 基数必须取自真实号段末尾 max(200)+1 = 201，
/// 首轮直接得到 201（若回潮旧 count+1=101 基数，将从 101 起连探 100 个
/// 全占位并落入显式报错，本断言即红——本用例同时锁死新基数行为）；
/// 随后模拟他会话抢先把候选 201 落库（等价于并发窗口内的旁路提交），
/// 第二轮取号必须递增到 202 成功返回，**绝不静默返回已被占用的 201**；
/// 202 真实 INSERT 落库不撞 23505，且表内 101..=202 全部唯一无重号。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock + customer_code 真实 UNIQUE），由专用 job（--run-ignored only）执行"]
async fn allocate_advances_past_preempted_candidate_without_duplicate() {
    let pfx = "NGA3";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    let codes: Vec<String> = (101..=200).map(|n| code(pfx, n)).collect();
    seed(&db, &codes).await;

    let first = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .expect("基数与号段同源时 101..=200 占满低段、起点应为空闲位 201，取号必须成功");
    assert_eq!(
        first,
        code(pfx, 201),
        "基数必须是 max(当日真实单号后缀流水)+1=201；若回潮 count+1=101 起探测将连撞占位并报错，此处应红"
    );

    // 模拟并发抢占：他会话在本会话第二轮取号前把候选 201 真实提交落库
    make_active(first.clone())
        .insert(&db)
        .await
        .unwrap_or_else(|e| panic!("夹具：模拟抢占插入 {first} 失败: {e}"));

    let second = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .expect("候选 201 被抢占后必须递增到 202 成功，不得报错");
    assert_eq!(
        second,
        code(pfx, 202),
        "被占候选必须递增避让，得 202，实际 {second}"
    );
    assert_ne!(
        second, first,
        "两次取号结果不得相同——绝不静默产出已被占用（UNIQUE 挡住）的号"
    );
    // 返回的 202 真实落库：若生成器产出了被占号，这一步会撞 23505 显式失败
    make_active(second.clone())
        .insert(&db)
        .await
        .unwrap_or_else(|e| panic!("夹具：落库本轮返回号 {second} 失败（疑似产出重复号）: {e}"));

    let mut in_table: Vec<String> = customer::Entity::find()
        .filter(customer::Column::CustomerCode.starts_with(pfx))
        .all(&db)
        .await
        .expect("查询 NGA3 号段存量失败")
        .into_iter()
        .map(|m| m.customer_code)
        .collect();
    assert_eq!(
        in_table.len(),
        102,
        "表内应为种子 101..=200 共 100 行 + 抢占行 201 + 本轮落库 202"
    );
    in_table.sort();
    in_table.dedup();
    assert_eq!(
        in_table.len(),
        102,
        "表内出现重号：生成器在并发抢占下不得产出重复号（UNIQUE 兜底之外的主动避让契约）"
    );

    cleanup(&db, pfx).await;
}

/// 探测被占到达上限 → **显式报错**（原 NGA3 意图"占满不得静默跳号/不得无界
/// 循环"的独立保留守卫）。新基数语义下起点恒等于 max+1（空闲位），静态占满
/// 101..=200 无法再触发探测上限；确定性可达路径是源码注释明说的病态饱和号段：
/// 存量含后缀恰为 `u64::MAX` 的单号 → 基数 `saturating_add(1)` 饱和停在
/// u64::MAX → 同一候选位被连探 `NO_PROBE_MAX=100` 次 → 显式
/// `BusinessErrorDisplayable`（真实文案可外显），绝不无界循环、绝不静默产号。
#[tokio::test]
#[ignore = "需已迁移 PG（advisory lock），由专用 job（--run-ignored only）执行"]
async fn allocate_errors_explicitly_when_probe_cap_exhausted() {
    let pfx = "NGA5";
    let db = live_db().await;
    cleanup(&db, pfx).await;

    // 后缀 20 位数字（u64::MAX 字面量），整号 4+8+20=32 字符，列宽 VARCHAR(50) 可容纳
    let saturated = format!("{}{}{}", pfx, today(), u64::MAX);
    seed(&db, &[saturated]).await;

    let err = DocumentNumberGenerator::generate_no(
        &db,
        pfx,
        customer::Entity,
        customer::Column::CustomerCode,
    )
    .await
    .expect_err(
        "候选位连续被占用达 NO_PROBE_MAX=100 上限时必须显式报错（防止无界循环/静默产错号）",
    );
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

/// 禁回潮静态锁（**不依赖数据库、不 #[ignore]**，随常规 CI 分片 job 常跑）：
/// `allocate_no` 的候选基数禁止回潮为按行数 `count + 1` 起探测——行数把软删行
/// 与旁路写入计入，起点既错又跳号（旧语义，已由与真实号段同源的 max+1 取代）。
/// 行为锁见 NGA3；此处源码扫描是第二道闸，基数实现一被改回 count 形态，
/// 不等活库 job 就在常规编译期/运行期红。
#[test]
fn number_cardinal_must_not_regress_to_count_based() {
    let src = include_str!("../src/utils/number_generator.rs");
    let start = src
        .find("async fn allocate_no")
        .expect("allocate_no 被重命名/删除：禁回潮锁需与新结构同步更新，禁止直接删本用例");
    let end_rel = src[start..]
        .find("is_document_no_taken")
        .expect("allocate_no 到登记函数的边界消失：需人工核对基数实现后同步本锁");
    let body = &src[start..start + end_rel];
    assert!(
        !body.contains(".count(") && !body.contains("count()"),
        "allocate_no 内检测到 count 调用：基数禁止回潮为 count+1（软删/旁路行计入行数导致起点错且跳号），必须与真实号段同源取 max+1"
    );
    assert!(
        body.contains("max_seq") && body.contains("saturating_add(1)"),
        "allocate_no 必须保持『基数 = max(当日真实单号后缀流水).saturating_add(1)』形态；max_seq/saturating_add 缺失说明基数实现被改动，需同步复核本锁与 NGA3"
    );
}

/// 宽度上限（3 位流水 999 溢出）——如实记录当前实现结果：
/// 存量 001..999，基数 max(999)+1 = 1000（旧 count+1 亦得 1000，结果同值），
/// `{:03}` 只补齐不截断，
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
    seed(&db, &codes).await;

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

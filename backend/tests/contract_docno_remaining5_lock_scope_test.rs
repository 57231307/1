//! 五站点取号锁作用域与并发不重号的活库契约锁
//!
//! 钉住单据号取号走带事务臂形态，前缀级 advisory 锁占用在调用方事务的整个
//! 存活期内，事务未结束前另一连接读 pg_locks 必须看到 advisory 锁条数比基线多，
//! 事务结束后必须回落到基线。取号一旦改回自开事务的形态，锁随内层事务提交
//! 立即释放，建单事务的 INSERT 之前重新出现并发窗口，两次并发建单会取到同一
//! 单号。覆盖五站点。cost_collections.collection_no（前缀 COST，经
//! CostCollectionService.generate_collection_no 服务级方法直接钉）、
//! production_recipe.recipe_no（RP，与采购入库 PR 分键，避免咨询锁跨域串行）、
//! production_recipe_addition.addition_no（PA）、
//! process_wage_rate.rate_no（PWR）、wage_record.record_no（WR）。
//! 后四者的服务侧取号函数对集成测试 crate 不可见，正向锁改在生成器
//! DocumentNumberGenerator::generate_no_with_txn 上以同一前缀、同一实体列钉
//! 服务实际依赖的原语作用域，接线形态由
//! contract_docno_remaining5_txn_arm_source_lock_test.rs 源码级锁钉住。
//! 反向钉为锁不在册或事务结束后仍在册（锁泄漏）都判红。
//! 变异自检逐前缀显式调用自开事务的 DocumentNumberGenerator::generate_no，
//! 断言其真的取到非空号后 advisory 锁条数与基线相等，证明主判据测的是锁的
//! 作用域而不是恒真读数，非空断言防空转。
//! 并发行为用例经各服务真实建单入口同时发起两次 create，钉并发不产生重复单号
//! 且两行都真实落库。
//! 真实唯一冲突行为用例钉撞约束路径返回含单号已存在语义的专属错误，
//! 不是生成失败请稍后重试类的拍平文案。
//! 调用方为集成测试 crate，仅在 CI 对已跑完迁移链的 PostgreSQL 执行，
//! 活体串行组逐条跑（--test-threads=1）。
//! 入参为 test_common::setup_test_db 注入业务连接，探测连接按 TEST_DATABASE_URL
//! 另开一条，确保与业务连接不在同一后端会话，才能看见对方持有的锁。
//! 判据结果交给断言本身，无下游。
//! 锁作用域用例不落任何业务行，取号只做号段探测并以回滚收尾；并发与冲突用例
//! 落出的探针行由 setup_test_db 的逐用例 TRUNCATE 回收。

mod test_common;

use bingxi_backend::models::status::wage_record_status;
use bingxi_backend::models::{
    cost_collection, process_route, process_wage_rate, production_recipe,
    production_recipe_addition, wage_record,
};
use bingxi_backend::services::cost_collection_service::{
    CostCollectionService, CreateCostCollectionRequest,
};
use bingxi_backend::services::production_recipe_service::{
    CreateProductionRecipeAdditionRequest, CreateProductionRecipeRequest,
    ProductionRecipeAdditionService, ProductionRecipeService,
};
use bingxi_backend::services::wage_service::{
    CreateWageRateRequest, CreateWageRecordRequest, WageRateService, WageRecordService,
};
use bingxi_backend::utils::number_generator::DocumentNumberGenerator;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend,
    EntityTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use std::sync::Arc;
use test_common::setup_test_db;

/// 另开一条探测连接（与业务连接不同会话，才能看见业务会话持有的锁）
async fn probe_connection() -> DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("探测 pg_locks 需要 TEST_DATABASE_URL（与业务夹具同一 PostgreSQL）");
    Database::connect(url)
        .await
        .expect("探测连接建立失败（PostgreSQL 不可达）")
}

/// 读当前会话集里 advisory 锁的条数（只读，不申请任何锁）
async fn advisory_lock_count(db: &DatabaseConnection) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*) AS cnt FROM pg_locks WHERE locktype = 'advisory'".to_string(),
        ))
        .await
        .expect("读取 pg_locks 的 advisory 锁条数失败");
    row.and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .expect("advisory 锁条数解码失败")
}

/// 当日单号日期段前缀（与生成器口径一致，前缀加 YYYYMMDD）
fn date_prefix(prefix: &str) -> String {
    format!("{}{}", prefix, Utc::now().format("%Y%m%d"))
}

/// 逐站点生成两对用例，正向钉带事务臂的锁作用域，变异自检钉自开事务形态放锁
///
/// 入参为用例名、前缀字面量、实体与列（与对应服务 impl_generate_no 或生成器
/// 直调处逐字符同源）。传给谁，判据结果交给断言本身，无下游，不落业务行。
macro_rules! lock_scope_pair_tests {
    ($positive:ident, $mutation:ident, $prefix:expr, $entity:expr, $column:expr) => {
        #[tokio::test]
        async fn $positive() {
            let db = setup_test_db().await;
            let probe = probe_connection().await;
            let baseline = advisory_lock_count(&probe).await;

            let txn = db.begin().await.expect("开启取号事务失败");
            let no = DocumentNumberGenerator::generate_no_with_txn(&txn, $prefix, $entity, $column)
                .await
                .expect("带事务臂取号失败");

            let dp = date_prefix($prefix);
            assert!(
                no.starts_with(&dp),
                "单号应以 {} 前缀与当日日期起头，实得 {}",
                $prefix,
                no
            );
            assert_eq!(
                no.len(),
                dp.len() + 3,
                "单号总长应为前缀日期段加三位流水，实得 {}",
                no
            );

            let held = advisory_lock_count(&probe).await;
            assert!(
                held > baseline,
                "取号锁必须存活到调用方事务结束，事务未提交前另一连接看到的 advisory 锁\
                 应比基线 {} 条更多，实得 {} 条",
                baseline,
                held
            );

            txn.rollback().await.expect("回滚取号事务失败");
            assert_eq!(
                advisory_lock_count(&probe).await,
                baseline,
                "调用方事务结束后取号锁必须释放，不得泄漏"
            );
        }

        #[tokio::test]
        async fn $mutation() {
            let db = setup_test_db().await;
            let probe = probe_connection().await;
            let baseline = advisory_lock_count(&probe).await;

            let no = DocumentNumberGenerator::generate_no(&db, $prefix, $entity, $column)
                .await
                .expect("自开事务形态取号失败");
            assert!(
                !no.is_empty(),
                "对照形态同样要真的取到号，否则下面的放锁断言是在空转"
            );

            assert_eq!(
                advisory_lock_count(&probe).await,
                baseline,
                "自开事务的取号返回后锁应已释放，这正是被取代的缺陷形态，\
                 调用方还没写库，锁已经不在，两次并发取号会落到同一个号"
            );
        }
    };
}

lock_scope_pair_tests!(
    recipe_no_with_txn_lock_holds_until_txn_end,
    recipe_mutation_self_opening_releases_lock_early,
    "RP",
    production_recipe::Entity,
    production_recipe::Column::RecipeNo
);
lock_scope_pair_tests!(
    addition_no_with_txn_lock_holds_until_txn_end,
    addition_mutation_self_opening_releases_lock_early,
    "PA",
    production_recipe_addition::Entity,
    production_recipe_addition::Column::AdditionNo
);
lock_scope_pair_tests!(
    rate_no_with_txn_lock_holds_until_txn_end,
    rate_mutation_self_opening_releases_lock_early,
    "PWR",
    process_wage_rate::Entity,
    process_wage_rate::Column::RateNo
);
lock_scope_pair_tests!(
    record_no_with_txn_lock_holds_until_txn_end,
    record_mutation_self_opening_releases_lock_early,
    "WR",
    wage_record::Entity,
    wage_record::Column::RecordNo
);

/// cost_collection 走服务级带事务臂方法正向钉锁作用域（宏接线由源码级锁另行钉）
#[tokio::test]
async fn cost_collection_no_service_txn_arm_lock_holds_until_txn_end() {
    let db = setup_test_db().await;
    let probe = probe_connection().await;
    let baseline = advisory_lock_count(&probe).await;

    let txn = db.begin().await.expect("开启取号事务失败");
    let service = CostCollectionService::new(Arc::new(db.clone()));
    let no = service
        .generate_collection_no(&txn)
        .await
        .expect("服务级带事务臂取号失败");

    let dp = date_prefix("COST");
    assert!(
        no.starts_with(&dp),
        "成本归集单号应以 COST 前缀与当日日期起头，实得 {}",
        no
    );

    let held = advisory_lock_count(&probe).await;
    assert!(
        held > baseline,
        "服务级取号锁必须存活到调用方事务结束，事务未提交前另一连接看到的 advisory 锁\
         应比基线 {} 条更多，实得 {} 条",
        baseline,
        held
    );

    txn.rollback().await.expect("回滚取号事务失败");
    assert_eq!(
        advisory_lock_count(&probe).await,
        baseline,
        "调用方事务结束后取号锁必须释放，不得泄漏"
    );
}

/// cost_collection 变异自检，自开事务形态返回即放锁，非空取号防空转
#[tokio::test]
async fn cost_collection_mutation_self_opening_releases_lock_early() {
    let db = setup_test_db().await;
    let probe = probe_connection().await;
    let baseline = advisory_lock_count(&probe).await;

    let no = DocumentNumberGenerator::generate_no(
        &db,
        "COST",
        cost_collection::Entity,
        cost_collection::Column::CollectionNo,
    )
    .await
    .expect("自开事务形态取号失败");
    assert!(
        !no.is_empty(),
        "对照形态同样要真的取到号，否则下面的放锁断言是在空转"
    );

    assert_eq!(
        advisory_lock_count(&probe).await,
        baseline,
        "自开事务的取号返回后锁应已释放，这正是被取代的缺陷形态，\
         调用方还没写库，锁已经不在，两次并发取号会落到同一个号"
    );
}

// ===== 并发行为用例，经真实建单入口钉并发不产生重复单号 =====

fn cost_req() -> CreateCostCollectionRequest {
    CreateCostCollectionRequest {
        collection_date: Utc::now().date_naive(),
        cost_object_type: None,
        cost_object_id: None,
        cost_object_no: None,
        batch_no: None,
        color_no: None,
        dye_lot_no: None,
        workshop: None,
        direct_material: Decimal::new(100, 0),
        direct_labor: Decimal::new(50, 0),
        manufacturing_overhead: Decimal::new(20, 0),
        processing_fee: Decimal::new(10, 0),
        dyeing_fee: Decimal::new(5, 0),
        output_quantity_meters: None,
        output_quantity_kg: None,
    }
}

#[tokio::test]
async fn cost_collection_concurrent_create_yields_distinct_no() {
    let db = Arc::new(setup_test_db().await);
    let svc_a = CostCollectionService::new(db.clone());
    let svc_b = CostCollectionService::new(db.clone());
    let (ra, rb) = (cost_req(), cost_req());
    let (a, b) = tokio::join!(svc_a.create(ra, 1), svc_b.create(rb, 1));
    let a = a.expect("并发建成本归集第一次应成功");
    let b = b.expect("并发建成本归集第二次应成功");
    assert_ne!(
        a.collection_no, b.collection_no,
        "并发建成本归集不得取到同一单号，实得两枚同为 {}",
        a.collection_no
    );
    let rows = cost_collection::Entity::find()
        .all(&*db)
        .await
        .expect("回读成本归集行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建单应各落一行，实得 {} 行",
        rows.len()
    );
}

#[tokio::test]
async fn recipe_concurrent_create_yields_distinct_no() {
    let db = Arc::new(setup_test_db().await);
    let svc_a = ProductionRecipeService::new(db.clone());
    let svc_b = ProductionRecipeService::new(db.clone());
    let mk = || CreateProductionRecipeRequest {
        work_order_id: None,
        dye_batch_id: None,
        source_recipe_id: None,
        lab_dip_resample_id: None,
        customer_id: None,
        color_no: None,
        fabric_name: None,
        fabric_spec: None,
        fabric_width: None,
        gram_weight: None,
        fabric_weight: Decimal::new(100, 0),
        equipment_no: None,
        liquor_ratio: "1:8".to_string(),
        bath_volume: None,
        adjustment_factor: None,
        recipe_detail: None,
        total_dye_cost: None,
        total_auxiliary_cost: None,
        remarks: None,
    };
    let (a, b) = tokio::join!(svc_a.create(mk(), 1), svc_b.create(mk(), 1));
    let a = a.expect("并发建大货处方第一次应成功");
    let b = b.expect("并发建大货处方第二次应成功");
    assert_ne!(
        a.recipe_no, b.recipe_no,
        "并发建大货处方不得取到同一单号，实得两枚同为 {}",
        a.recipe_no
    );
    let rows = production_recipe::Entity::find()
        .all(&*db)
        .await
        .expect("回读大货处方行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建单应各落一行，实得 {} 行",
        rows.len()
    );
}

/// 加料处方并发建单前先种一张已审核大货处方（服务校验关联处方必须 approved）
async fn seed_approved_recipe(db: &DatabaseConnection) -> i32 {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let row = production_recipe::ActiveModel {
        recipe_no: Set("PR-CONTRACT-SEED".to_string()),
        fabric_weight: Set(Decimal::new(100, 0)),
        liquor_ratio: Set("1:8".to_string()),
        status: Set(bingxi_backend::models::status::production_recipe::APPROVED.to_string()),
        is_deleted: Set(false),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子大货处方插入失败");
    row.id
}

#[tokio::test]
async fn addition_concurrent_create_yields_distinct_no() {
    let db = Arc::new(setup_test_db().await);
    let recipe_id = seed_approved_recipe(&db).await;
    let svc_a = ProductionRecipeAdditionService::new(db.clone());
    let svc_b = ProductionRecipeAdditionService::new(db.clone());
    let mk = || CreateProductionRecipeAdditionRequest {
        production_recipe_id: recipe_id,
        work_order_id: None,
        dye_batch_id: None,
        addition_reason: None,
        addition_detail: None,
        total_cost: None,
        remarks: None,
    };
    let (a, b) = tokio::join!(svc_a.create(mk(), 1), svc_b.create(mk(), 1));
    let a = a.expect("并发建加料处方第一次应成功");
    let b = b.expect("并发建加料处方第二次应成功");
    assert_ne!(
        a.addition_no, b.addition_no,
        "并发建加料处方不得取到同一单号，实得两枚同为 {}",
        a.addition_no
    );
    let rows = production_recipe_addition::Entity::find()
        .all(&*db)
        .await
        .expect("回读加料处方行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建单应各落一行，实得 {} 行",
        rows.len()
    );
}

/// 工价并发建单前先种一条工序路线（服务校验 process_route_id 必须存在）
async fn seed_process_route(db: &DatabaseConnection) -> i32 {
    let now: chrono::DateTime<chrono::FixedOffset> =
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap().into();
    let row = process_route::ActiveModel {
        route_code: Set("DOCNO-PROBE-ROUTE".to_string()),
        route_name: Set("契约探针工序".to_string()),
        seq: Set(1),
        process_type: Set("dye".to_string()),
        require_scan: Set(false),
        is_active: Set(true),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子工序路线插入失败");
    row.id
}

#[tokio::test]
async fn wage_rate_concurrent_create_yields_distinct_no() {
    let db = Arc::new(setup_test_db().await);
    let route_id = seed_process_route(&db).await;
    let svc_a = WageRateService::new(db.clone());
    let svc_b = WageRateService::new(db.clone());
    let mk = || CreateWageRateRequest {
        process_route_id: route_id,
        wage_type: None,
        piece_price: Some(Decimal::new(1, 0)),
        time_price: None,
        grade_a_ratio: None,
        grade_b_ratio: None,
        grade_c_ratio: None,
        effective_date: Utc::now().date_naive(),
        expiry_date: None,
        workshop: None,
        remarks: None,
    };
    let (a, b) = tokio::join!(svc_a.create(mk(), 1), svc_b.create(mk(), 1));
    let a = a.expect("并发建工价第一次应成功");
    let b = b.expect("并发建工价第二次应成功");
    assert_ne!(
        a.rate_no, b.rate_no,
        "并发建工价不得取到同一单号，实得两枚同为 {}",
        a.rate_no
    );
    let rows = process_wage_rate::Entity::find()
        .all(&*db)
        .await
        .expect("回读工价行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建单应各落一行，实得 {} 行",
        rows.len()
    );
}

#[tokio::test]
async fn wage_record_concurrent_create_yields_distinct_no() {
    let db = Arc::new(setup_test_db().await);
    let svc_a = WageRecordService::new(db.clone());
    let svc_b = WageRecordService::new(db.clone());
    let mk = || CreateWageRecordRequest {
        period_start: Utc::now().date_naive(),
        period_end: Utc::now().date_naive(),
        workshop: None,
        remarks: None,
    };
    let (a, b) = tokio::join!(svc_a.create(mk(), 1), svc_b.create(mk(), 1));
    let a = a.expect("并发建工资记录第一次应成功");
    let b = b.expect("并发建工资记录第二次应成功");
    assert_ne!(
        a.record_no, b.record_no,
        "并发建工资记录不得取到同一单号，实得两枚同为 {}",
        a.record_no
    );
    let rows = wage_record::Entity::find()
        .all(&*db)
        .await
        .expect("回读工资记录行数失败");
    assert_eq!(
        rows.len(),
        2,
        "两次成功建单应各落一行，实得 {} 行",
        rows.len()
    );
}

/// 真实唯一冲突行为钉，旁路连接先占当日首号且不提交，服务建单撞约束后
/// 必须返回含该单号与已存在语义的专属可外显错误，不得是生成失败请稍后重试
/// 类的拍平文案，且失败后活跃行数不增加（整事务回滚不留半行）。
/// 旁路提交时机由 pg_stat_activity 观测服务侧 wage_record 的 INSERT 进入
/// 等待后才触发，消除竞态交错下的时序不稳定；若服务在旁路可见后才取号
/// （观测超时），则接受成功回落并钉两号不同且不落重复。
#[tokio::test]
async fn wage_record_conflict_returns_docno_exists_not_flattened_retry() {
    let db = setup_test_db().await;
    let bypass = probe_connection().await;

    let taken_no = format!("{}001", date_prefix("WR"));
    let now: chrono::DateTime<chrono::FixedOffset> =
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap().into();
    let btxn = bypass.begin().await.expect("旁路事务开启失败");
    let seed = wage_record::ActiveModel {
        record_no: Set(taken_no.clone()),
        period_start: Set(Utc::now().date_naive()),
        period_end: Set(Utc::now().date_naive()),
        total_workers: Set(0),
        total_step_records: Set(0),
        total_qualified_quantity: Set(Decimal::ZERO),
        total_duration_minutes: Set(0),
        total_amount: Set(Decimal::ZERO),
        // status 与服务写入词表同源（models::status::wage_record_status::DRAFT）
        status: Set(wage_record_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    seed.insert(&btxn).await.expect("旁路占号插入失败");

    let service = WageRecordService::new(Arc::new(db.clone()));
    let create_task = tokio::spawn(async move {
        service
            .create(
                CreateWageRecordRequest {
                    period_start: Utc::now().date_naive(),
                    period_end: Utc::now().date_naive(),
                    workshop: None,
                    remarks: None,
                },
                1,
            )
            .await
    });

    // 轮询服务侧 INSERT 是否已进入等待（被旁路未提交行的唯一索引冲突阻塞），
    // 命中即提交旁路事务，令服务侧在同刻收到 23505
    let mut observed_blocking = false;
    for _ in 0..100 {
        let blocked = bypass
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT COUNT(*) AS cnt FROM pg_stat_activity \
                 WHERE state = 'active' AND query ILIKE '%INSERT INTO \\\"wage_record\\\"%' \
                   AND pid <> pg_backend_pid()"
                    .to_string(),
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);
        if blocked > 0 {
            observed_blocking = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    btxn.commit().await.expect("旁路占号事务提交失败");

    let result = create_task.await.expect("建单任务join失败");

    match result {
        Err(e) => {
            assert!(
                observed_blocking,
                "撞约束报错路径应先观测到服务侧 INSERT 阻塞，观测缺失说明交错异常，错误为 {}",
                e
            );
            let msg = e.to_string();
            assert!(
                msg.contains("已存在") && msg.contains(&taken_no),
                "撞唯一约束应返回含单号 {} 与已存在语义的专属错误，实得 {}",
                taken_no,
                msg
            );
            assert!(
                !msg.contains("请稍后重试"),
                "撞约束错误禁止拍平成生成失败请稍后重试类文案，实得 {}",
                msg
            );
            let live = wage_record::Entity::find()
                .filter(wage_record::Column::IsDeleted.eq(false))
                .all(&db)
                .await
                .expect("回读工资记录活跃行数失败");
            assert_eq!(
                live.len(),
                1,
                "冲突建单失败后只应余旁路一行，服务侧不得留下半行，实得 {} 行",
                live.len()
            );
        }
        Ok(model) => {
            assert_ne!(
                model.record_no, taken_no,
                "未观测到阻塞时服务取号必须已看到旁路行并顺延，绝不允许撞出同号双行，\
                 实得旁路 {} 与服务 {}",
                taken_no, model.record_no
            );
            let live = wage_record::Entity::find()
                .filter(wage_record::Column::IsDeleted.eq(false))
                .all(&db)
                .await
                .expect("回读工资记录活跃行数失败");
            assert_eq!(
                live.len(),
                2,
                "回落成功路径应恰好旁路一行加服务一行，实得 {} 行",
                live.len()
            );
        }
    }

    let remaining = wage_record::Entity::find()
        .filter(wage_record::Column::RecordNo.eq(taken_no.as_str()))
        .all(&db)
        .await
        .expect("清理回读失败");
    for row in remaining {
        wage_record::Entity::delete_by_id(row.id)
            .exec(&db)
            .await
            .expect("旁路探针行清理失败");
    }
}

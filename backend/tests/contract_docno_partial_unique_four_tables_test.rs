//! m0104 四表活跃行部分唯一索引与 cost_collections 全表唯一兜底的活库契约锁
//!
//! 钉数据库层唯一约束真实在场，服务侧取号同临界区改造以它为先决。
//! 正向四件事。一，m0104 迁移源文本形态锁，四条部分唯一索引
//! （WHERE "is_deleted" = false）、四条 fail-visible 重复探测 RAISE EXCEPTION
//! 守卫、down 四条显式 DROP INDEX（不留空 down 且与 up 对称）逐条在场。
//! 二，活库 pg_indexes 回读，四条索引真实存在且索引定义含单号列与
//! is_deleted 谓词（索引形态与读路径谓词同源的正向）。
//! 三，撞约束行为钉，四表各插入一枚哨兵活跃行后同号二次插入必须报
//! SqlErr::UniqueConstraintViolation（重复活跃行落不进去），
//! cost_collections 的 m0063 全表唯一兜底同样钉住。
//! 四，软删复用行为钉，哨兵行置 is_deleted 为 true 后同号活跃行插入必须成功
//! （部分索引谓词与既定复用语义同源的反向钉，防止有人改成全表 UNIQUE）。
//! 读路径谓词同源的反向源码钉，四个读侧文件必须按 IsDeleted.eq(false) 过滤
//! 且单号读口逐处带未删谓词，与部分索引谓词互锁漂移。
//! 调用方为集成测试 crate，仅在 CI 对已跑完迁移链（含 m0104）的 PostgreSQL
//! 执行，活体串行组逐条跑（--test-threads=1）。
//! 入参为 test_common::setup_test_db 注入业务连接（逐用例先清业务表）。
//! 判据结果交给断言本身，无下游。探针行由 setup_test_db 的逐用例 TRUNCATE
//! 回收，软删复用用例自带主键硬删闭环。

mod test_common;

use bingxi_backend::models::{
    cost_collection, process_wage_rate, production_recipe, production_recipe_addition, wage_record,
};
use bingxi_backend::models::status::{
    production_recipe as recipe_status, production_recipe_addition as addition_status,
    wage_rate_status, wage_record_status, wage_type,
};
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, IntoActiveModel,
    QueryFilter, QueryResult, Set, SqlErr, Statement,
};
use test_common::setup_test_db;

const MIGRATION_PATH: &str =
    "migration/src/domain/m0104_add_document_no_partial_unique_four_tables.rs";

const FOUR_INDEXES: [(&str, &str, &str); 4] = [
    (
        "uq_production_recipe_recipe_no_active",
        "production_recipe",
        "recipe_no",
    ),
    (
        "uq_production_recipe_addition_addition_no_active",
        "production_recipe_addition",
        "addition_no",
    ),
    (
        "uq_process_wage_rate_rate_no_active",
        "process_wage_rate",
        "rate_no",
    ),
    ("uq_wage_record_record_no_active", "wage_record", "record_no"),
];

fn migration_src() -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(MIGRATION_PATH);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("读取 m0104 迁移源失败: {e}"))
        .replace('\r', "")
}

fn read_service(path: &str) -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(path);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
        .replace('\r', "")
}

/// 正向一，m0104 源文本形态锁，索引语句、探测守卫、down 回滚逐表钉
#[test]
fn m0104_migration_text_pins_partial_unique_shape() {
    let text = migration_src();
    for (index, table, column) in FOUR_INDEXES {
        assert!(
            text.contains(&format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS \"{}\"\n    ON \"{}\" (\"{}\") WHERE \"is_deleted\" = false;",
                index, table, column
            )),
            "m0104 缺 {} 表 {} 列的活跃行部分唯一索引语句（须带 WHERE is_deleted 谓词）",
            table,
            column
        );
        assert!(
            text.contains(&format!(
                "FROM \"{}\"\n         WHERE \"is_deleted\" = false",
                table
            )),
            "m0104 缺 {} 表建索引前的 fail-visible 重复探测（探测谓词必须与索引谓词同源）",
            table
        );
        assert!(
            text.contains(&format!("DROP INDEX IF EXISTS \"{}\";", index)),
            "m0104 down 必须显式回滚 {}（不留空 down、不只删一半）",
            index
        );
    }
    assert_eq!(
        text.matches("RAISE EXCEPTION").count(),
        4,
        "m0104 应有四条存量重复拒绝消息（命中重复即中止不洗数据），实得 {} 条",
        text.matches("RAISE EXCEPTION").count()
    );
    assert!(
        !text.contains("uq_cost_collections_collection_no"),
        "cost_collections 已由 m0063 全表 UNIQUE 兜底（该表无 is_deleted 列），\
         m0104 不得为它重复建活跃行索引制造第二套口径"
    );
}

/// 读路径与索引谓词同源的双向锁，单号读口必须按未删行过滤
#[test]
fn reader_paths_filter_is_deleted_same_as_index_predicate() {
    for file in [
        "src/services/wage_ops/rate.rs",
        "src/services/wage_ops/record.rs",
        "src/services/production_recipe_ops/recipe_crud.rs",
        "src/services/production_recipe_ops/addition.rs",
    ] {
        let text = read_service(file);
        assert!(
            text.contains("IsDeleted.eq(false)"),
            "{} 的读路径必须按 IsDeleted.eq(false) 过滤，与 m0104 部分索引谓词同源",
            file
        );
    }
    let rate = read_service("src/services/wage_ops/rate.rs");
    assert!(
        rate.contains("process_wage_rate::Column::RateNo.eq(rate_no)")
            && rate.contains("process_wage_rate::Column::IsDeleted.eq(false)"),
        "rate.rs 的 get_by_no 必须同时按单号与未删谓词过滤"
    );
    let record = read_service("src/services/wage_ops/record.rs");
    assert!(
        record.contains("wage_record::Column::RecordNo.eq(record_no)")
            && record.contains("wage_record::Column::IsDeleted.eq(false)"),
        "record.rs 的 get_by_no 必须同时按单号与未删谓词过滤"
    );
}

/// 活库回读，四条部分唯一索引存在且定义含单号列与 is_deleted 谓词
#[tokio::test]
async fn live_db_has_all_four_partial_unique_indexes() {
    let db = setup_test_db().await;
    for (index, table, column) in FOUR_INDEXES {
        let rows = db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT indexdef FROM pg_indexes WHERE schemaname = current_schema AND tablename = $1 AND indexname = $2",
                [table.into(), index.into()],
            ))
            .await
            .unwrap_or_else(|e| panic!("查询 {} 的索引回读失败: {e}", index));
        let defs: Vec<String> = rows
            .iter()
            .map(|r: &QueryResult| {
                r.try_get_by_index::<String>("", "indexdef")
                    .expect("indexdef 解码失败")
            })
            .collect();
        assert_eq!(
            defs.len(),
            1,
            "活库缺唯一索引 {}（{} 表 {} 列），pg_indexes 实得 {} 行；\
             若迁移链未应用 m0104，请确认测试库已跑完本支迁移",
            index,
            table,
            column,
            defs.len()
        );
        let def = defs.first().expect("上一断言已保证恰有一行");
        assert!(
            def.contains("UNIQUE INDEX") && def.contains(column) && def.contains("is_deleted"),
            "索引 {} 定义形态不符（应含 UNIQUE、列名与 is_deleted 谓词），实得 {}",
            index,
            def
        );
    }
}

/// 把插入结果按唯一违例判红，非唯一类错误原样 panic 暴露真实成因
fn expect_unique_violation(result: Result<impl sea_orm::ModelTrait, sea_orm::DbErr>, what: &str) {
    match result {
        Ok(_) => panic!("{} 的重复活跃行竟然落库成功，唯一约束缺失或被绕过", what),
        Err(e) => assert!(
            matches!(e.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))),
            "{} 的重复插入应报唯一约束违例，实得底层错误 {}",
            what,
            e
        ),
    }
}

/// 撞约束行为钉，四表同号第二条活跃行必须撞唯一违例，
/// cost_collections 的 m0063 全表兜底同样钉住
#[tokio::test]
async fn live_db_rejects_duplicate_active_doc_no_on_five_tables() {
    let db = setup_test_db().await;
    let now: chrono::DateTime<chrono::FixedOffset> =
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap().into();
    let today = Utc::now().date_naive();

    let seed = production_recipe::ActiveModel {
        recipe_no: Set("UQPRB-SEED".to_string()),
        fabric_weight: Set(Decimal::new(100, 0)),
        liquor_ratio: Set("1:8".to_string()),
        status: Set(recipe_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("哨兵大货处方插入失败");
    let mut dup = seed.clone().into_active_model();
    dup.id = Default::default();
    expect_unique_violation(dup.insert(&db).await, "production_recipe.recipe_no");

    let seed = production_recipe_addition::ActiveModel {
        addition_no: Set("UQPAB-SEED".to_string()),
        production_recipe_id: Set(999_999),
        status: Set(addition_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("哨兵加料处方插入失败");
    let mut dup = seed.clone().into_active_model();
    dup.id = Default::default();
    expect_unique_violation(dup.insert(&db).await, "production_recipe_addition.addition_no");

    let seed = process_wage_rate::ActiveModel {
        rate_no: Set("UQPWRB-SEED".to_string()),
        process_route_id: Set(999_999),
        route_code: Set("UQ-PROBE".to_string()),
        route_name: Set("唯一约束探针".to_string()),
        wage_type: Set(wage_type::PIECE.to_string()),
        piece_price: Set(Decimal::ONE),
        time_price: Set(Decimal::ZERO),
        grade_a_ratio: Set(Decimal::ONE),
        grade_b_ratio: Set(Decimal::ZERO),
        grade_c_ratio: Set(Decimal::ZERO),
        effective_date: Set(today),
        status: Set(wage_rate_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("哨兵工价插入失败");
    let mut dup = seed.clone().into_active_model();
    dup.id = Default::default();
    expect_unique_violation(dup.insert(&db).await, "process_wage_rate.rate_no");

    let seed = wage_record::ActiveModel {
        record_no: Set("UQWRB-SEED".to_string()),
        period_start: Set(today),
        period_end: Set(today),
        total_workers: Set(0),
        total_step_records: Set(0),
        total_qualified_quantity: Set(Decimal::ZERO),
        total_duration_minutes: Set(0),
        total_amount: Set(Decimal::ZERO),
        status: Set(wage_record_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("哨兵工资记录插入失败");
    let mut dup = seed.clone().into_active_model();
    dup.id = Default::default();
    expect_unique_violation(dup.insert(&db).await, "wage_record.record_no");

    let seed = cost_collection::ActiveModel {
        collection_no: Set("UQCOSTB-SEED".to_string()),
        collection_date: Set(today),
        direct_material: Set(Decimal::ZERO),
        direct_labor: Set(Decimal::ZERO),
        manufacturing_overhead: Set(Decimal::ZERO),
        processing_fee: Set(Decimal::ZERO),
        dyeing_fee: Set(Decimal::ZERO),
        total_cost: Set(Decimal::ZERO),
        status: Set("draft".to_string()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("哨兵成本归集插入失败");
    let mut dup = seed.clone().into_active_model();
    dup.id = Default::default();
    expect_unique_violation(
        dup.insert(&db).await,
        "cost_collections.collection_no（m0063 全表唯一兜底）",
    );
}

/// 软删复用行为钉，占号行置软删后同号活跃行必须可落（谓词同源的反向面）
#[tokio::test]
async fn live_db_allows_number_reuse_after_soft_delete() {
    let db = setup_test_db().await;
    let now: chrono::DateTime<chrono::FixedOffset> =
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap().into();

    let first = wage_record::ActiveModel {
        record_no: Set("UQWR-REUSE".to_string()),
        period_start: Set(Utc::now().date_naive()),
        period_end: Set(Utc::now().date_naive()),
        total_workers: Set(0),
        total_step_records: Set(0),
        total_qualified_quantity: Set(Decimal::ZERO),
        total_duration_minutes: Set(0),
        total_amount: Set(Decimal::ZERO),
        status: Set(wage_record_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("复用钉第一行插入失败");

    let mut blocked = first.clone().into_active_model();
    blocked.id = Default::default();
    expect_unique_violation(blocked.insert(&db).await, "复用钉前提，活跃重复必须被拒");

    let mut soft = first.clone().into_active_model();
    soft.is_deleted = Set(true);
    soft.update(&db).await.expect("复用钉软删失败");

    let mut reused = first.clone().into_active_model();
    reused.id = Default::default();
    let reused = reused
        .insert(&db)
        .await
        .expect("软删后同号活跃行应可复用落库（部分唯一索引谓词语义），失败即谓词漂移");

    wage_record::Entity::delete_by_id(first.id)
        .exec(&db)
        .await
        .expect("复用钉第一行清理失败");
    wage_record::Entity::delete_by_id(reused.id)
        .exec(&db)
        .await
        .expect("复用钉复用行清理失败");
}

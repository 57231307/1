//! 商机赢单金额结转契约锁（连真库 PostgreSQL，`setup_test_db` + 真实回读）。
//!
//! 覆盖 `CrmService::convert_opportunity_to_order` 的赢单金额语义：赢单时
//! `crm_opportunity.estimated_amount`（预估金额）结转进 `actual_amount`（实际金额），
//! 同时保留 `estimated_amount` 原值不抹空。结转方向依据源码：赢单落库只
//! `Set(actual_amount)` 写入估算原值，`estimated_amount` 列保持 Unchanged 不参与
//! UPDATE，故赢单后两列都等于赢单前的估算原值——是「结转」而非「把预估列保留后再
//! 从预估列取不到值」。此锁钉死缺陷回归：赢单不得把已有估算真值抹成 NULL、也不得
//! 把实际金额写成 NULL。
//!
//! 通道：用例经 `test_common::setup_test_db` 连已迁移 PostgreSQL 真跑；表结构唯一
//! 来源 = backend/migration，不自建 DDL、不回退 sqlite。FK 父行口径同
//! `contract_wave7_crm_opp_amount_scope_test`：users id=1（商机 owner/建单人 +
//! trg_crm_opportunity_dept、trg_sales_orders_dept 触发器按 department_id=1 回填）、
//! customers id=1（crm_opportunity.customer_id 与赢单草稿 sales_order.customer_id
//! 真 FK 父行）自种子。

mod test_common;

use bingxi_backend::models::crm_opportunity;
use bingxi_backend::models::status::crm_opportunity as opp_status;
use bingxi_backend::services::crm::cust::CrmService;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend,
    EntityTrait, Statement,
};
use std::sync::Arc;
use test_common::setup_test_db;

fn ts() -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("种子日期非法")
        .and_hms_opt(0, 0, 0)
        .expect("种子时间非法")
        .and_utc()
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

/// 原生执行种子 SQL（口径同 contract_wave15_inspection_order_id_derivation_test::exec）。
async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 已迁移真库 + 本域 FK 父行（users id=1 / customers id=1，ON CONFLICT 兜幂等）。
async fn seeded_db() -> DatabaseConnection {
    let db = setup_test_db().await;
    exec(
        &db,
        r#"INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             (1,'w15_won_operator','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')
           ON CONFLICT (id) DO NOTHING"#,
    )
    .await;
    exec(
        &db,
        r#"INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,
             status,customer_type,owner_id,created_at,updated_at) VALUES
             (1,'CUS-W15-WON','赢单结转客户',0,30,'active','retail',1,
             '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')
           ON CONFLICT (id) DO NOTHING"#,
    )
    .await;
    db
}

/// 播种一条 OPEN 商机：预估金额传入值非空，实际金额如实留空（赢单前本无成交金额）。
async fn seed_open_opportunity(db: &DatabaseConnection, estimated: Decimal) -> i32 {
    let now = ts();
    let opp = crm_opportunity::ActiveModel {
        opportunity_no: Set(format!("OPP-W15-WON-{}", unique_suffix())),
        opportunity_name: Set(format!("赢单结转商机-{}", unique_suffix())),
        customer_id: Set(1),
        lead_id: Set(None),
        opportunity_type: Set(Some("NEW".to_string())),
        opportunity_stage: Set(Some("QUALIFICATION".to_string())),
        win_probability: Set(Some(Decimal::from(50))),
        estimated_amount: Set(Some(estimated)),
        actual_amount: Set(None),
        currency: Set(Some("CNY".to_string())),
        expected_close_date: Set(Some(now.date_naive())),
        actual_close_date: Set(None),
        product_ids: Set(None),
        product_names: Set(None),
        product_desc: Set(None),
        owner_id: Set(1),
        department_id: Set(Some(1)),
        owner_name: Set("销售甲".to_string()),
        last_follow_up_date: Set(None),
        next_follow_up_date: Set(None),
        follow_up_plan: Set(None),
        competitor_names: Set(None),
        competitive_advantage: Set(None),
        opportunity_status: Set(Some(opp_status::OPEN.to_string())),
        won_reason: Set(None),
        lost_reason: Set(None),
        priority: Set(Some("high".to_string())),
        rating: Set(None),
        tags: Set(None),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        created_by: Set(Some(1)),
        updated_by: Set(Some(1)),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种 OPEN 商机失败");
    opp.id
}

/// 从库真回读商机（不是断内存值/接口 2xx，而是断落库后的列值）。
async fn reread_opportunity(db: &DatabaseConnection, id: i32) -> crm_opportunity::Model {
    crm_opportunity::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("回读商机失败")
        .expect("商机必须存在")
}

fn svc(db: &DatabaseConnection) -> CrmService {
    CrmService::new(Arc::new(db.clone()))
}

/// 赢单结转：带非空预估金额的 OPEN 商机走赢单通路后，回读断——
/// 1) 实际金额 = 赢单前预估原值（结转）；
/// 2) 预估金额未被抹成 NULL（仍 = 原值）；
/// 3) 商机确已进入 CLOSED_WON（排除只建草稿单未赢单的假绿）。
#[tokio::test]
async fn won_carries_estimated_amount_to_actual_and_keeps_estimated() {
    let db = seeded_db().await;
    // DECIMAL(15,2) 口径，与回读标度一致，避免 scale 差异引入的假判。
    let estimated = Decimal::new(8_888_800, 2);
    let opp_id = seed_open_opportunity(&db, estimated).await;

    // 前提（真读库确认种子形态）：赢单前预估有值、实际为空。
    let before = reread_opportunity(&db, opp_id).await;
    assert_eq!(
        before.estimated_amount,
        Some(estimated),
        "种子前提：赢单前预估金额必须为非空原值，否则结转断言失去区分力"
    );
    assert_eq!(
        before.actual_amount, None,
        "种子前提：赢单前实际金额必须为空，排除「本来就带实际金额」造成的假绿"
    );

    // 走赢单通路（商机转草稿订单 + 标记赢单）。
    svc(&db)
        .convert_opportunity_to_order(opp_id, 1)
        .await
        .expect("OPEN 商机赢单转订单应成功");

    // 赢单后真读库。
    let after = reread_opportunity(&db, opp_id).await;

    // 断言1（结转方向）：赢单把预估金额结转进实际金额——落库后 actual == 原 estimated。
    assert_eq!(
        after.actual_amount,
        Some(estimated),
        "赢单必须把预估金额结转写入 actual_amount（修复前该列被写成 NULL，属数据破坏）"
    );
    // 断言2（不静默清空）：预估金额作为历史预测基准保留，赢单不得把它抹成 NULL。
    assert_eq!(
        after.estimated_amount,
        Some(estimated),
        "赢单不得把已有的 estimated_amount 真值静默清空为 NULL（源码赢单只写 actual_amount，\
         estimated_amount 保持 Unchanged 不写，故两列都应等于原估算值）"
    );
    // 断言3（确走通赢单）：状态进入 CLOSED_WON，排除「只建单未赢单」的假绿。
    assert_eq!(
        after.opportunity_status.as_deref(),
        Some(opp_status::CLOSED_WON),
        "赢单通路必须把商机状态置为 CLOSED_WON"
    );
}

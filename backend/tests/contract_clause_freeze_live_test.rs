//! 签约条款渲染与固化腿的活库行为锁（切片 B 第二件，全部用例由本次推送触发的云端 CI 兑付）
//!
//! 功能 在真已迁移、按用例清表的 PostgreSQL 上经服务层 API 钉死签约固化腿的行为
//!   按启用模板渲染并固化 快照行存固化当时的正文原文、标题、条款码、顺序、模板版本号与
//!   来源条款项指针；未选模板签约合法（快照为空，详情与打印按“模板前签约”口径）；
//!   草稿与停用模板不可选用；占位符未填满整笔回滚（合同行与快照行都不留）；
//!   模板改版与条款库改文不回溯已固化行（新签用新文，已签行原样）；
//!   值表多余键与同条款码多变体歧义与必填条款条件无来源三类配置错误点名拒绝；
//!   详情腿必填标记按指针单义回查、历史空指针行唯一回落、多义如实给空值；
//!   打印腿摘要文本两口径（固化文本与“模板前签约，条款未固化”）；
//!   数据库侧 chk_cc_body_rendered 与 chk_cc_body_fully_rendered 兜底裸 SQL 归因。
//! 调用方 集成测试 crate，经测试夹具连已跑完整迁移链的 PostgreSQL，跑法
//!   `bash E:/tmp/tools/live_test_lane.sh <车道> contract_clause_freeze_live_test`。
//!   按车道验证分工，本文件本地不跑，编译与用例结论由 CI run 兑付。
//! 入参 无，夹具注入库连接；造模板走模板读写腿服务真接口，造合同走合同创建服务真接口。
//! 传给谁 断言落在 `contract_clauses` 回读行值、服务返回与拒绝消息点名内容、SQLSTATE 与约束名。
//! 存什么 只写本文件自建的探针行，编码带 IT-CTPL-F 私有号段。
//! 存哪里 PostgreSQL `contract_templates`、`clause_library`、`contract_template_clauses`、
//!   `contract_clauses`、`sales_contracts`、`purchase_contracts`。
//!
//! 反空操作自证
//! - 固化只存模板 id 不存正文时，回读断言（正文等于成品文本）必红。
//! - 改版回溯改写已固化行时，改版对照段的正文与版本断言必红。
//! - 占位符未填仍落库时，回滚断言（合同行数 0 与快照行数 0）必红，
//!   或残留 {{ 被库侧 CHECK 当场判红。
//! - 详情腿伪造必填标记时，空指针多义段的 None 断言必红。

mod test_common;

use bingxi_backend::models::contract_clause;
use bingxi_backend::services::contract_clause_service;
use bingxi_backend::services::contract_template_service::{
    ContractTemplateService, CreateTemplateRequest, ReviewClauseRequest, UpsertClauseRequest,
};
use bingxi_backend::services::purchase_contract_service::{
    CreateContractRequest, PurchaseContractService,
};
use bingxi_backend::services::sales_contract_service::{
    CreateSalesContractRequest, SalesContractService,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr,
    EntityTrait, Order, QueryFilter, QueryOrder, RuntimeErr, Set, Statement, Value,
};
use std::collections::HashMap;
use std::sync::Arc;
use test_common::setup_test_db;

const IT: &str = "IT-CTPL-F";

fn tpl_svc(db: &DatabaseConnection) -> ContractTemplateService {
    ContractTemplateService::new(Arc::new(db.clone()))
}

fn pg_bound(sql: String, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn sqlstate_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned()))
            .unwrap_or_else(|| panic!("驱动未上报 SQLSTATE，无法归因: {err}")),
        other => panic!("期望执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

fn constraint_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string()))
            .unwrap_or_else(|| panic!("驱动未上报约束名，无法归因: {err}")),
        other => panic!("期望执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

async fn expect_rejected(db: &DatabaseConnection, stmt: Statement, code: &str, name: &str) {
    let err = match db.execute_raw(stmt).await {
        Ok(_) => panic!("期望被 {name} 拒绝的写入竟然成功（该约束未生效或被放宽）"),
        Err(err) => err,
    };
    assert_eq!(sqlstate_of(&err), code, "SQLSTATE 归因失守: {err}");
    assert_eq!(
        constraint_of(&err),
        name,
        "拒绝应归因到约束 {name}，实得 {err}"
    );
}

async fn seed_customer(db: &DatabaseConnection, code: &str, name: &str) -> i32 {
    let c = bingxi_backend::models::customer::ActiveModel {
        customer_code: Set(code.to_string()),
        customer_name: Set(name.to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    c.id
}

/// 建一枚两条款模板 一枚自带正文（含 {{customer_name}} 占位符）、一枚引用条款库
/// （库文含 {{payment_terms}} 占位符），复核全过并置启用。返回模板 id。
async fn active_two_clause_template(db: &DatabaseConnection, tag: &str) -> i32 {
    // 先播种一枚可用条款库行（库文引用腿要求库行存在且启用）
    db.execute_raw(pg_bound(
        "INSERT INTO clause_library (clause_code, title, body, status, created_by) \
         VALUES ('payment_and_invoice', '付款与账期', '货款按 {{payment_terms}} 结算。', 'active', 9102)"
            .to_string(),
        vec![],
    ))
    .await
    .expect("播种条款库行失败");
    let lib_id = {
        use bingxi_backend::models::clause_library as lib;
        lib::Entity::find()
            .filter(lib::Column::ClauseCode.eq("payment_and_invoice"))
            .one(db)
            .await
            .expect("查库行失败")
            .expect("库行查不到")
            .id
    };
    let svc = tpl_svc(db);
    let created = svc
        .create(
            CreateTemplateRequest {
                template_code: format!("{IT}-{tag}"),
                contract_type: "domestic_sales".to_string(),
                party_side: "seller".to_string(),
                title: format!("渲染固化探针 {tag}"),
                effective_from: None,
                remark: None,
                clauses: Some(vec![
                    UpsertClauseRequest {
                        clause_code: "subject_and_spec".to_string(),
                        title: "标的与规格".to_string(),
                        body: Some("甲方向乙方供应 {{customer_name}} 约定之面料。".to_string()),
                        library_clause_id: None,
                        required: Some(true),
                        sort: 1,
                        applies_when: None,
                    },
                    UpsertClauseRequest {
                        clause_code: "payment_and_invoice".to_string(),
                        title: "付款条款".to_string(),
                        body: None,
                        library_clause_id: Some(lib_id),
                        required: Some(true),
                        sort: 2,
                        applies_when: None,
                    },
                ]),
            },
            9101,
        )
        .await
        .unwrap_or_else(|e| panic!("创建探针模板失败: {e}"));
    let views = svc.list_clause_views(created.id).await.expect("读条款失败");
    for v in views {
        svc.review_clause(
            created.id,
            v.clause.id,
            ReviewClauseRequest {
                legal_review_status: "approved".to_string(),
                legal_review_remark: None,
            },
            9102,
        )
        .await
        .expect("条款复核失败");
    }
    svc.approve(created.id, 9102).await.expect("模板复核失败");
    svc.activate(created.id, 9102).await.expect("置启用失败");
    created.id
}

fn sales_values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

async fn snapshot_rows(
    db: &DatabaseConnection,
    kind: &str,
    id: i32,
) -> Vec<contract_clause::Model> {
    contract_clause::Entity::find()
        .filter(contract_clause::Column::ContractKind.eq(kind))
        .filter(contract_clause::Column::ContractId.eq(id))
        .order_by(contract_clause::Column::Sort, Order::Asc)
        .all(db)
        .await
        .expect("读快照失败")
}

async fn count_contracts(db: &DatabaseConnection, no: &str) -> i64 {
    use sea_orm::PaginatorTrait;
    bingxi_backend::models::sales_contract::Entity::find()
        .filter(bingxi_backend::models::sales_contract::Column::ContractNo.eq(no))
        .count(db)
        .await
        .expect("数合同失败") as i64
}

#[tokio::test]
async fn sales_create_freezes_snapshot_text_version_and_pointer() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "FRZ-1").await;
    let cid = seed_customer(&db, &format!("{IT}-CUS1"), "固化验证客户").await;

    let contract = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-001")),
                contract_name: "固化腿正向合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1000),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(template_id),
                clause_values: Some(sales_values(&[
                    ("customer_name", "固化验证客户"),
                    ("payment_terms", "月结60天"),
                ])),
                contract_market: Some("domestic".to_string()),
                tax_inclusive: Some(true),
            },
            9101,
        )
        .await
        .expect("带模板签约应成功");

    let rows = snapshot_rows(&db, "sales", contract.id).await;
    assert_eq!(rows.len(), 2, "两枚条款应固化两行快照，实得 {}", rows.len());
    assert_eq!(
        rows[0].clause_code, "subject_and_spec",
        "第 1 行按顺序应为标的条款"
    );
    assert_eq!(
        rows[0].body_rendered, "甲方向乙方供应 固化验证客户 约定之面料。",
        "固化正文必须是占位符填齐后的成品原文"
    );
    assert_eq!(
        rows[1].body_rendered, "货款按 月结60天 结算。",
        "库引用条款必须把固化当时的库文填进快照"
    );
    for r in &rows {
        assert_eq!(r.template_id, template_id, "快照必须记来源模板");
        assert_eq!(r.template_version, 1, "快照必须记固化当时的模板版本号");
        assert!(
            r.template_clause_id.is_some(),
            "新固化行必须写来源条款项指针，实得 None（详情腿必填标记就无从单义回查）"
        );
        assert_eq!(r.contract_kind, "sales", "侧别必须与销售合同一致");
    }
    // 详情腿出参 顺序 标题 正文 必填标记 版本齐备且必填标记按指针解析为真
    let detail = contract_clause_service::list_clauses_for_contract(&db, "sales", contract.id)
        .await
        .expect("详情条款数组失败");
    assert_eq!(detail.len(), 2);
    assert_eq!(detail[0]["title"], serde_json::json!("标的与规格"));
    assert_eq!(
        detail[0]["required"],
        serde_json::json!(true),
        "必填标记应解析为真"
    );
    assert_eq!(detail[0]["template_version"], serde_json::json!(1));
    // 打印腿摘要 含两条正文
    let summary = contract_clause_service::clauses_summary_for_print(&db, "sales", contract.id)
        .await
        .expect("打印摘要失败");
    assert!(
        summary.contains("标的与规格") && summary.contains("月结60天"),
        "摘要缺条款：{summary}"
    );
}

#[tokio::test]
async fn missing_placeholder_rolls_back_contract_and_snapshot() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "ROLL-1").await;
    let cid = seed_customer(&db, &format!("{IT}-CUS2"), "回滚验证客户").await;

    let err = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-002")),
                contract_name: "缺占位符值合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(template_id),
                // 只给 customer_name，payment_terms 缺值
                clause_values: Some(sales_values(&[("customer_name", "回滚验证客户")])),
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect_err("占位符未填满必须拒绝落库");
    let msg = err.to_string();
    assert!(
        msg.contains("payment_terms"),
        "拒绝消息应点名缺键 payment_terms，实得「{msg}」"
    );
    assert_eq!(
        count_contracts(&db, &format!("{IT}-SC-002")).await,
        0,
        "渲染拒绝必须整笔回滚，合同行不得残留"
    );
    use sea_orm::PaginatorTrait;
    let snap = contract_clause::Entity::find()
        .count(&db)
        .await
        .expect("数快照失败");
    assert_eq!(snap, 0, "渲染拒绝后条款快照必须一行都不留");
}

#[tokio::test]
async fn draft_and_retired_templates_are_not_selectable() {
    let db = setup_test_db().await;
    let cid = seed_customer(&db, &format!("{IT}-CUS3"), "门验证客户").await;
    let svc = tpl_svc(&db);
    let draft = svc
        .create(
            CreateTemplateRequest {
                template_code: format!("{IT}-GATE-D"),
                contract_type: "domestic_sales".to_string(),
                party_side: "seller".to_string(),
                title: "草稿门探针".to_string(),
                effective_from: None,
                remark: None,
                clauses: Some(vec![UpsertClauseRequest {
                    clause_code: "subject_and_spec".to_string(),
                    title: "标的".to_string(),
                    body: Some("正文无占位符。".to_string()),
                    library_clause_id: None,
                    required: Some(true),
                    sort: 1,
                    applies_when: None,
                }]),
            },
            9101,
        )
        .await
        .expect("建草稿失败");

    let err = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-003")),
                contract_name: "选用草稿合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(draft.id),
                clause_values: None,
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect_err("草稿模板不可被选用签约");
    assert!(
        err.to_string().contains("只有启用中的模板"),
        "拒绝应点名启用门，实得「{err}」"
    );
    assert_eq!(count_contracts(&db, &format!("{IT}-SC-003")).await, 0);

    // 启用再停用 停用版同样不可选
    let views = svc.list_clause_views(draft.id).await.unwrap();
    for v in views {
        svc.review_clause(
            draft.id,
            v.clause.id,
            ReviewClauseRequest {
                legal_review_status: "approved".to_string(),
                legal_review_remark: None,
            },
            9102,
        )
        .await
        .expect("复核条款失败");
    }
    svc.approve(draft.id, 9102).await.expect("复核模板失败");
    svc.activate(draft.id, 9102).await.expect("启用失败");
    svc.retire(draft.id, 9102).await.expect("停用失败");
    let err = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-004")),
                contract_name: "选用停用模板合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(draft.id),
                clause_values: None,
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect_err("停用模板不可被选用签约");
    assert!(
        err.to_string().contains("只有启用中的模板"),
        "停用模板拒绝应点名启用门，实得「{err}」"
    );
}

/// 工单要求的“模板改版不得回溯影响已固化行”活体断言 整段收在同一用例内顺序执行。
#[tokio::test]
async fn template_revision_never_touches_frozen_rows() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "REVX-1").await;
    let cid = seed_customer(&db, &format!("{IT}-CUS4"), "改版回溯客户").await;
    let contract = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-005")),
                contract_name: "改版对照合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(template_id),
                clause_values: Some(sales_values(&[
                    ("customer_name", "改版回溯客户"),
                    ("payment_terms", "月结30天"),
                ])),
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect("v1 签约失败");
    let before = snapshot_rows(&db, "sales", contract.id).await;
    assert_eq!(before.len(), 2);

    // 改版 改新草稿的条款文与标题与顺序 走完复核启用全腿
    let svc = tpl_svc(&db);
    let v2 = svc.revise(template_id, 9103).await.expect("改版失败");
    let v2_clauses = svc.list_clause_views(v2.id).await.unwrap();
    let first = &v2_clauses[0];
    svc.update_clause(
        v2.id,
        first.clause.id,
        UpsertClauseRequest {
            clause_code: "subject_and_spec".to_string(),
            title: "标的与规格（二版改名）".to_string(),
            body: Some("改版后的标的正文 {{customer_name}}。".to_string()),
            library_clause_id: None,
            required: Some(true),
            sort: 5,
            applies_when: None,
        },
        9103,
    )
    .await
    .expect("二版改条款失败");
    for c in svc.list_clause_views(v2.id).await.unwrap() {
        svc.review_clause(
            v2.id,
            c.clause.id,
            ReviewClauseRequest {
                legal_review_status: "approved".to_string(),
                legal_review_remark: None,
            },
            9104,
        )
        .await
        .expect("二版条款复核失败");
    }
    svc.approve(v2.id, 9104).await.expect("二版复核失败");
    svc.retire(template_id, 9104).await.expect("停用一版失败");
    svc.activate(v2.id, 9104).await.expect("启用二版失败");

    // 已固化行必须逐字段原样：正文 标题 顺序 版本号 指针
    let after = snapshot_rows(&db, "sales", contract.id).await;
    assert_eq!(after.len(), before.len(), "改版不得增删已固化行");
    assert_eq!(
        after[0].body_rendered, before[0].body_rendered,
        "改版不得回改固化正文"
    );
    assert_eq!(after[0].title, "标的与规格", "固化标题保持当时值");
    assert_eq!(after[0].sort, before[0].sort, "固化顺序保持当时值");
    assert_eq!(after[0].template_version, 1, "版本指针保持签约当时的一版");
    assert!(
        !after[0].body_rendered.contains("改版后的标的正文"),
        "已固化行被改版内容污染：{}",
        after[0].body_rendered
    );

    // 新签用二版 固化文本与版本随新模板走
    let contract2 = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-006")),
                contract_name: "二版新签合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(2),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(v2.id),
                clause_values: Some(sales_values(&[
                    ("customer_name", "改版回溯客户"),
                    ("payment_terms", "月结30天"),
                ])),
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect("二版新签失败");
    let rows2 = snapshot_rows(&db, "sales", contract2.id).await;
    assert!(
        rows2
            .iter()
            .any(|r| r.body_rendered.contains("改版后的标的正文")),
        "二版新签必须固化新文本"
    );
    assert!(
        rows2.iter().all(|r| r.template_version == 2),
        "二版新签版本应为 2"
    );
}

#[tokio::test]
async fn library_clause_text_change_affects_only_new_signings() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "LIBX-1").await;
    let cid = seed_customer(&db, &format!("{IT}-CUS5"), "库文对照客户").await;
    let mk = |no: &str| CreateSalesContractRequest {
        contract_no: Some(no.to_string()),
        contract_name: "库文对照合同".to_string(),
        customer_id: cid,
        total_amount: Decimal::from(1),
        contract_type: None,
        payment_terms: None,
        delivery_date: None,
        signed_date: None,
        effective_date: None,
        expiry_date: None,
        payment_method: None,
        delivery_location: None,
        remark: None,
        source_order_id: None,
        items: None,
        template_id: Some(template_id),
        clause_values: Some(sales_values(&[
            ("customer_name", "库文对照客户"),
            ("payment_terms", "票到付款"),
        ])),
        contract_market: None,
        tax_inclusive: None,
    };
    let old = SalesContractService::new(Arc::new(db.clone()))
        .create(mk(&format!("{IT}-SC-007")), 9101)
        .await
        .expect("旧签失败");
    // 库文改字（模拟“改一处多处生效”只对新签生效）
    db.execute_raw(pg_bound(
        "UPDATE clause_library SET body = '改后的新库文 {{payment_terms}}。' \
         WHERE clause_code = 'payment_and_invoice'"
            .to_string(),
        vec![],
    ))
    .await
    .expect("改库文失败");
    let new = SalesContractService::new(Arc::new(db.clone()))
        .create(mk(&format!("{IT}-SC-008")), 9101)
        .await
        .expect("新签失败");
    let old_rows = snapshot_rows(&db, "sales", old.id).await;
    let new_rows = snapshot_rows(&db, "sales", new.id).await;
    assert!(
        old_rows
            .iter()
            .any(|r| r.body_rendered == "货款按 票到付款 结算。"),
        "旧签合同快照必须保持固化当时的库文，不得被改库文回溯污染"
    );
    assert!(
        new_rows
            .iter()
            .any(|r| r.body_rendered == "改后的新库文 票到付款。"),
        "新签必须取到改后的库文"
    );
}

#[tokio::test]
async fn config_ambiguous_and_wasteful_inputs_are_named() {
    let db = setup_test_db().await;
    let cid = seed_customer(&db, &format!("{IT}-CUS6"), "歧义验证客户").await;

    // 歧义 同条款码两个变体同时命中（export 与 tax_inclusive 同时成立）
    let svc = tpl_svc(&db);
    let tpl = svc
        .create(
            CreateTemplateRequest {
                template_code: format!("{IT}-AMB-1"),
                contract_type: "export_sales".to_string(),
                party_side: "seller".to_string(),
                title: "歧义变体探针".to_string(),
                effective_from: None,
                remark: None,
                clauses: Some(vec![
                    UpsertClauseRequest {
                        clause_code: "price_and_adjustment".to_string(),
                        title: "价格条款出口版".to_string(),
                        body: Some("出口价格条款。".to_string()),
                        library_clause_id: None,
                        required: Some(false),
                        sort: 1,
                        applies_when: Some("export".to_string()),
                    },
                    UpsertClauseRequest {
                        clause_code: "price_and_adjustment".to_string(),
                        title: "价格条款含税版".to_string(),
                        body: Some("含税价格条款。".to_string()),
                        library_clause_id: None,
                        required: Some(false),
                        sort: 2,
                        applies_when: Some("tax_inclusive".to_string()),
                    },
                ]),
            },
            9101,
        )
        .await
        .expect("建歧义模板失败");
    for c in svc.list_clause_views(tpl.id).await.unwrap() {
        svc.review_clause(
            tpl.id,
            c.clause.id,
            ReviewClauseRequest {
                legal_review_status: "approved".to_string(),
                legal_review_remark: None,
            },
            9102,
        )
        .await
        .expect("复核失败");
    }
    svc.approve(tpl.id, 9102).await.expect("复核失败");
    svc.activate(tpl.id, 9102).await.expect("启用失败");
    let err = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-009")),
                contract_name: "歧义合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(tpl.id),
                clause_values: None,
                // 出口且含税 两个变体同时命中
                contract_market: Some("export".to_string()),
                tax_inclusive: Some(true),
            },
            9101,
        )
        .await
        .expect_err("同条款码多命中必须拒绝");
    assert!(
        err.to_string().contains("只允许出一条"),
        "歧义拒绝应点名，实得「{err}」"
    );

    // 多余键点名（值表与模板错位）
    let template_id = active_two_clause_template(&db, "EXTRA-1").await;
    let err = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-010")),
                contract_name: "多余键合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(template_id),
                clause_values: Some(sales_values(&[
                    ("customer_name", "x"),
                    ("payment_terms", "y"),
                    ("delivery_date_zzz", "多余键"),
                ])),
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect_err("值表多余键必须点名拒绝");
    assert!(
        err.to_string().contains("delivery_date_zzz"),
        "多余键拒绝应点名键本身，实得「{err}」"
    );
}

#[tokio::test]
async fn required_clause_without_condition_source_rejects_and_custom_variant_skips() {
    let db = setup_test_db().await;
    let cid = seed_customer(&db, &format!("{IT}-CUS7"), "条件来源客户").await;
    let svc = tpl_svc(&db);
    // 必填且 applies_when custom_made 合同侧无该来源 签约必须拒绝
    let tpl = svc
        .create(
            CreateTemplateRequest {
                template_code: format!("{IT}-REQ-1"),
                contract_type: "domestic_sales".to_string(),
                party_side: "seller".to_string(),
                title: "必填无来源探针".to_string(),
                effective_from: None,
                remark: None,
                clauses: Some(vec![UpsertClauseRequest {
                    clause_code: "subject_and_spec".to_string(),
                    title: "订做标的条款".to_string(),
                    body: Some("订做标的正文。".to_string()),
                    library_clause_id: None,
                    required: Some(true),
                    sort: 1,
                    applies_when: Some("custom_made".to_string()),
                }]),
            },
            9101,
        )
        .await
        .expect("建模板失败");
    for c in svc.list_clause_views(tpl.id).await.unwrap() {
        svc.review_clause(
            tpl.id,
            c.clause.id,
            ReviewClauseRequest {
                legal_review_status: "approved".to_string(),
                legal_review_remark: None,
            },
            9102,
        )
        .await
        .expect("复核失败");
    }
    svc.approve(tpl.id, 9102).await.expect("复核失败");
    svc.activate(tpl.id, 9102).await.expect("启用失败");
    let err = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-011")),
                contract_name: "必填无来源合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: Some(tpl.id),
                clause_values: None,
                contract_market: Some("domestic".to_string()),
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect_err("必填条款条件无来源必须拒绝");
    assert!(
        err.to_string().contains("没有采集该条件的来源"),
        "拒绝应点名条件来源缺失，实得「{err}」"
    );
}

/// 采购腿同口径钉固 创建成功即固化，侧别 purchase。
#[tokio::test]
async fn purchase_create_freezes_snapshot_with_purchase_kind() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "PUR-1").await;
    // suppliers 为迁移播种参照表，用存量 id 1
    let contract = PurchaseContractService::new(Arc::new(db.clone()))
        .create(
            CreateContractRequest {
                contract_no: Some(format!("{IT}-PC-001")),
                contract_name: "采购固化合同".to_string(),
                supplier_id: 1,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                template_id: Some(template_id),
                clause_values: Some(sales_values(&[
                    ("customer_name", "采购对手方"),
                    ("payment_terms", "货到付款"),
                ])),
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect("采购带模板签约应成功");
    let rows = snapshot_rows(&db, "purchase", contract.id).await;
    assert_eq!(rows.len(), 2, "采购侧固化行数应为 2，实得 {}", rows.len());
    assert!(rows.iter().all(|r| r.contract_kind == "purchase"));
    assert!(rows.iter().all(|r| r.template_version == 1));
    assert!(rows.iter().all(|r| r.template_clause_id.is_some()));
}

/// 历史快照行（无来源指针）的必填标记回落口径 唯一行给值、多义给空值不猜。
#[tokio::test]
async fn legacy_snapshot_rows_resolve_required_with_no_guessing() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "LEG-1").await;
    // 直插一枚无指针快照行（历史形态） 条款码在模板内唯一 回落应给值 true
    db.execute_raw(pg_bound(
        "INSERT INTO contract_clauses (contract_kind, contract_id, clause_code, title, \
         body_rendered, template_id, template_version, sort) \
         VALUES ('sales', 900231, 'subject_and_spec', '标的与规格', '历史固化正文', $1, 1, 1)"
            .to_string(),
        vec![Value::Int(template_id)],
    ))
    .await
    .expect("插历史快照行失败");
    let detail = contract_clause_service::list_clauses_for_contract(&db, "sales", 900_231)
        .await
        .expect("历史行详情失败");
    assert_eq!(detail.len(), 1);
    assert_eq!(
        detail[0]["required"],
        serde_json::json!(true),
        "条款码唯一时应按 (模板,条款码) 回落给出必填标记"
    );

    // 同条款码加一枚并行变体（旁路直插绕开服务位置门） 回落多义 给空值
    db.execute_raw(pg_bound(
        "INSERT INTO contract_template_clauses (template_id, clause_code, title, body, required, sort, applies_when) \
         VALUES ($1, 'subject_and_spec', '标的变体', '变体正文', false, 8, 'export')"
            .to_string(),
        vec![Value::Int(template_id)],
    ))
    .await
    .expect("旁路插变体失败（模板已是 active，绕过服务门直写库面）");
    let detail2 = contract_clause_service::list_clauses_for_contract(&db, "sales", 900_231)
        .await
        .expect("多义历史行详情失败");
    assert_eq!(
        detail2[0]["required"],
        serde_json::Value::Null,
        "回落多义时必填标记必须给空值，绝不猜一个填上"
    );
}

#[tokio::test]
async fn snapshot_body_db_backstops_reject_blank_and_residue() {
    let db = setup_test_db().await;
    let template_id = active_two_clause_template(&db, "BS-1").await;

    // 空串正文 chk_cc_body_rendered
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_clauses (contract_kind, contract_id, clause_code, title, \
             body_rendered, template_id, template_version, sort) \
             VALUES ('sales', 900241, 'subject_and_spec', '标的', '', $1, 1, 1)"
                .to_string(),
            vec![Value::Int(template_id)],
        ),
        "23514",
        "chk_cc_body_rendered",
    )
    .await;

    // 残留未闭合占位符 chk_cc_body_fully_rendered
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_clauses (contract_kind, contract_id, clause_code, title, \
             body_rendered, template_id, template_version, sort) \
             VALUES ('sales', 900242, 'subject_and_spec', '标的', '半成品 {{customer_name}} 正文', $1, 1, 1)"
                .to_string(),
            vec![Value::Int(template_id)],
        ),
        "23514",
        "chk_cc_body_fully_rendered",
    )
    .await;

    // 未选模板签约的合法口径 合同建成 快照为空 打印摘要给“模板前签约”
    let cid = seed_customer(&db, &format!("{IT}-CUS8"), "无前签约客户").await;
    let contract = SalesContractService::new(Arc::new(db.clone()))
        .create(
            CreateSalesContractRequest {
                contract_no: Some(format!("{IT}-SC-012")),
                contract_name: "未选模板合同".to_string(),
                customer_id: cid,
                total_amount: Decimal::from(1),
                contract_type: None,
                payment_terms: None,
                delivery_date: None,
                signed_date: None,
                effective_date: None,
                expiry_date: None,
                payment_method: None,
                delivery_location: None,
                remark: None,
                source_order_id: None,
                items: None,
                template_id: None,
                clause_values: None,
                contract_market: None,
                tax_inclusive: None,
            },
            9101,
        )
        .await
        .expect("未选模板签约应成功");
    assert!(snapshot_rows(&db, "sales", contract.id).await.is_empty());
    let detail = contract_clause_service::list_clauses_for_contract(&db, "sales", contract.id)
        .await
        .expect("详情条款失败");
    assert!(
        detail.is_empty(),
        "模板前签约详情条款数组应为空，不得回填伪造内容"
    );
    let summary = contract_clause_service::clauses_summary_for_print(&db, "sales", contract.id)
        .await
        .expect("摘要失败");
    assert_eq!(
        summary, "模板前签约，条款未固化",
        "打印摘要口径必须与历史合同展示同源"
    );
}

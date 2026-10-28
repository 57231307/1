//! 合同模板读写腿的活库行为锁（切片 B 第一件，全部用例由本次推送触发的云端 CI 兑付）
//!
//! 功能 在真已迁移、按用例清表的 PostgreSQL 上经服务层 API 钉死模板读写腿的行为
//!   全生命周期 新建草稿、法务复核通过、置启用、停用、终态不可复活、同 (contract_type,
//!   party_side) 竞位点名拒绝、改版出新增版本并复位逐条复核、非草稿冻结条款与表头与删除、
//!   被快照引用的草稿拒绝删除、条款输入门（正文与库引用二选一、条款码与适用条件词表、
//!   顺序号界、库引用存在性与张冠李戴、同码同条件并行拒绝）、复核状态机出边、表头三态更新，
//!   以及数据库侧最后防线的 CHECK 与部分唯一索引（裸 SQL 归因到约束名）。
//! 调用方 集成测试 crate，经测试夹具连已跑完整迁移链的 PostgreSQL，跑法
//!   `bash E:/tmp/tools/live_test_lane.sh <车道> contract_template_write_leg_live_test`。
//!   按车道验证分工，本文件本地不跑，编译与用例结论由 CI run 兑付。
//! 入参 无，夹具注入库连接；服务层入参按各请求结构体。
//! 传给谁 断言落在服务返回的行值与拒绝消息的点名内容、以及数据库上报的 SQLSTATE 与约束名。
//! 存什么 只写本文件自建的探针行，模板编码一律带 IT-CTPL-W 私有号段；快照探针合同主键用
//!   9002xx 私有号段。存哪里 PostgreSQL `contract_templates`、`clause_library`、
//!   `contract_template_clauses`、`contract_clauses`。
//!
//! 反空操作自证
//! - 竞位门被放宽（activate 不再点名在位版）时，竞位用例的拒绝消息断言与在位状态断言双红。
//! - 服务出现第二条写 ACTIVE 通道（自动降位）时，旧版状态断言必红（用例显式断旧版仍是 active）。
//! - 改版未复位复核时，v2 条款 pending 断言必红。
//! - 非草稿冻结缺失时，冻结用例的拒绝断言必红。
//! - 裸字面量或旁路写库能绕过状态机吗 数据库 CHECK 与部分唯一索引的归因段直接判红。

mod test_common;

use bingxi_backend::services::contract_template_service::{
    ContractTemplateService, CreateTemplateRequest, ReviewClauseRequest, TemplateListQuery,
    UpdateTemplateHeaderRequest, UpsertClauseRequest,
};
use bingxi_backend::utils::error::AppError;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement, Value,
};
use std::sync::Arc;
use test_common::setup_test_db;

/// 探针模板编码私有号段。
const IT: &str = "IT-CTPL-W";
/// 快照探针的合同主键私有号段值。
const IT_SNAPSHOT_CONTRACT_ID: i32 = 900_201;

fn svc(db: &DatabaseConnection) -> ContractTemplateService {
    ContractTemplateService::new(Arc::new(db.clone()))
}

fn pg_bound(sql: String, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn opt_str(value: Option<&str>) -> Value {
    Value::String(value.map(|v| v.to_string()))
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
    let got_code = sqlstate_of(&err);
    let got_name = constraint_of(&err);
    assert_eq!(
        got_code, code,
        "拒绝成因的 SQLSTATE 应为 {code}，实得 {got_code}；原始错误 {err}"
    );
    assert_eq!(
        got_name, name,
        "拒绝应归因到约束 {name}，实得 {got_name}；原始错误 {err}"
    );
}

/// 服务拒绝的断言助手 要求返回 Err，且错误消息（Display 全文，含分类前缀）包含点名线索。
fn expect_err(err: AppError, needle: &str, what: &str) {
    let msg = err.to_string();
    assert!(
        msg.contains(needle),
        "{what} 的拒绝消息应点名「{needle}」，实得「{msg}」"
    );
}

/// 建一枚最小可用模板（domestic_sales/seller），条款给一枚通用自带正文条款。
async fn seed_template(db: &DatabaseConnection, code: &str) -> i32 {
    let created = svc(db)
        .create(
            CreateTemplateRequest {
                template_code: code.to_string(),
                contract_type: "domestic_sales".to_string(),
                party_side: "seller".to_string(),
                title: format!("{IT} 模板 {code}"),
                effective_from: None,
                remark: Some("活测探针".to_string()),
                clauses: Some(vec![UpsertClauseRequest {
                    clause_code: "subject_and_spec".to_string(),
                    title: "标的与规格".to_string(),
                    body: Some(format!("标的按 {{product}} 交付，探针 {code}。")),
                    library_clause_id: None,
                    required: Some(true),
                    sort: 1,
                    applies_when: None,
                }]),
            },
            9101,
        )
        .await
        .unwrap_or_else(|e| panic!("创建探针模板 {code} 失败: {e}"));
    created.id
}

/// 把条款全部复核通过并记录模板复核，得到一枚可启用的草稿。
async fn make_approvable(db: &DatabaseConnection, template_id: i32) {
    let views = svc(db)
        .list_clause_views(template_id)
        .await
        .expect("读取条款视图失败");
    for view in views {
        svc(db)
            .review_clause(
                template_id,
                view.clause.id,
                ReviewClauseRequest {
                    legal_review_status: "approved".to_string(),
                    legal_review_remark: None,
                },
                9102,
            )
            .await
            .unwrap_or_else(|e| panic!("条款 {} 复核通过失败: {e}", view.clause.id));
    }
    svc(db)
        .approve(template_id, 9102)
        .await
        .unwrap_or_else(|e| panic!("模板 {template_id} 记录复核失败: {e}"));
}

// ---------------------------------------------------------------------------
// 生命周期 复核门、竞位门、终态门
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lifecycle_approve_activate_retire_with_named_rejections() {
    let db = setup_test_db().await;
    let a = seed_template(&db, &format!("{IT}-LIFE-A")).await;
    let b_code = format!("{IT}-LIFE-B");
    let b = seed_template(&db, &b_code).await;

    // 未过复核直接启用 点名复核缺失
    let err = svc(&db)
        .activate(a, 9101)
        .await
        .expect_err("未复核草稿不应启用成功");
    expect_err(err, "尚未记录法务复核通过", "未复核启用");

    make_approvable(&db, a).await;
    let active_a = svc(&db).activate(a, 9101).await.expect("启用 A 失败");
    assert_eq!(active_a.status, "active", "启用后状态应为 active");

    // 同组竞位 拒绝并点名在位版编码与版本，且绝不动在位行
    make_approvable(&db, b).await;
    let err = svc(&db)
        .activate(b, 9101)
        .await
        .expect_err("同组竞位启用应被拒绝");
    let msg = err.to_string();
    assert!(
        msg.contains("只允许一版启用")
            && msg.contains(&format!("{IT}-LIFE-A"))
            && msg.contains("v1"),
        "竞位拒绝应点名在位版编码与版本，实得「{msg}」"
    );
    let still = svc(&db).get_by_id(a).await.expect("回读 A 失败");
    assert_eq!(
        still.status, "active",
        "竞位拒绝不得改动在位版状态（静默降位通道）"
    );

    // 显式两步 先停用 A 再启用 B
    let retired = svc(&db).retire(a, 9101).await.expect("停用 A 失败");
    assert_eq!(retired.status, "retired");
    let active_b = svc(&db)
        .activate(b, 9101)
        .await
        .expect("竞位解除后启用 B 失败");
    assert_eq!(active_b.status, "active");

    // 终态与非法跃迁点名
    let ret_b = svc(&db).retire(b, 9101).await.expect("停用 B 失败");
    assert_eq!(ret_b.status, "retired");
    let err2 = svc(&db)
        .retire(b, 9101)
        .await
        .expect_err("终态重复停用应被拒绝");
    expect_err(err2, "终态", "重复停用");
    let err3 = svc(&db)
        .activate(b, 9101)
        .await
        .expect_err("已停用版本复活应被拒绝");
    expect_err(err3, "启用只能从草稿发起", "停用后复活");
}

// ---------------------------------------------------------------------------
// 改版 新增递增版本、条款复制、复核复位；旧版不动
// ---------------------------------------------------------------------------

#[tokio::test]
async fn revise_bumps_version_and_resets_clause_review_only_on_new_row() {
    let db = setup_test_db().await;
    let rev_code = format!("{IT}-REV-A");
    let a = seed_template(&db, &rev_code).await;
    // 再加一枚条件条款，验证复制覆盖两种适用条件
    svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "export_special_terms".to_string(),
                title: "出口专章".to_string(),
                body: Some("结汇币种 {{currency}}。".to_string()),
                library_clause_id: None,
                required: Some(false),
                sort: 9,
                applies_when: Some("export".to_string()),
            },
            9101,
        )
        .await
        .expect("加条件条款失败");
    make_approvable(&db, a).await;
    svc(&db).activate(a, 9101).await.expect("启用 A 失败");

    let v2 = svc(&db).revise(a, 9103).await.expect("改版失败");
    assert_eq!(v2.version, 2, "改版版本号应递增到 2");
    assert_eq!(v2.status, "draft", "改版新行应为草稿");
    assert_eq!(v2.approved_by, None, "改版新行不得继承复核结论");
    assert_eq!(v2.template_code, rev_code, "改版必须沿用同码");

    let old_clauses = svc(&db).list_clause_views(a).await.expect("读旧版条款失败");
    let new_clauses = svc(&db)
        .list_clause_views(v2.id)
        .await
        .expect("读新版条款失败");
    assert_eq!(
        new_clauses.len(),
        old_clauses.len(),
        "改版应整组复制条款项，旧 {} 新 {}",
        old_clauses.len(),
        new_clauses.len()
    );
    for (old, fresh) in old_clauses.iter().zip(new_clauses.iter()) {
        assert_eq!(
            old.clause.clause_code, fresh.clause.clause_code,
            "复制保条款码"
        );
        assert_eq!(old.clause.sort, fresh.clause.sort, "复制保顺序号");
        assert_eq!(
            old.clause.applies_when, fresh.clause.applies_when,
            "复制保适用条件"
        );
        assert_eq!(old.clause.body, fresh.clause.body, "复制保正文");
        assert_eq!(
            fresh.clause.legal_review_status, "pending",
            "新版条款复核状态必须复位 pending（改版必须重审）"
        );
        assert_eq!(
            fresh.clause.legal_reviewed_by, None,
            "复位必须清空复核人留痕"
        );
    }
    // 旧版整行不动 仍是 active、复核留痕仍在
    let old_row = svc(&db).get_by_id(a).await.expect("回读旧版失败");
    assert_eq!(old_row.status, "active", "改版不得触碰源版本状态");
    assert!(old_row.approved_by.is_some(), "改版不得抹掉旧版复核留痕");
}

// ---------------------------------------------------------------------------
// 非草稿冻结 表头、条款、删除全部点名拒绝
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_draft_freeze_blocks_all_mutation_paths() {
    let db = setup_test_db().await;
    let a = seed_template(&db, &format!("{IT}-FRZ-A")).await;
    let clause_id = svc(&db).list_clause_views(a).await.unwrap()[0].clause.id;
    make_approvable(&db, a).await;
    svc(&db).activate(a, 9101).await.expect("启用失败");

    let err = svc(&db)
        .update_header(
            a,
            UpdateTemplateHeaderRequest {
                title: Some(Some("已启用还想改标题".to_string())),
                effective_from: None,
                remark: None,
            },
            9101,
        )
        .await
        .expect_err("active 模板改表头应被拒绝");
    expect_err(err, "不是草稿状态", "active 改表头");

    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "notice_and_service".to_string(),
                title: "通知与送达".to_string(),
                body: Some("书面通知。".to_string()),
                library_clause_id: None,
                required: Some(false),
                sort: 2,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("active 模板加条款应被拒绝");
    expect_err(err, "条款项只在草稿阶段可维护", "active 加条款");

    let err = svc(&db)
        .delete_clause(a, clause_id, 9101)
        .await
        .expect_err("active 模板删条款应被拒绝");
    expect_err(err, "条款项只在草稿阶段可维护", "active 删条款");

    let err = svc(&db)
        .delete(a, 9101)
        .await
        .expect_err("active 模板删除应被拒绝");
    expect_err(err, "只有未启用的草稿可以删除", "active 删除");
}

// ---------------------------------------------------------------------------
// 删除门 未引用草稿可删；被快照引用的草稿点名拒绝
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_allows_unreferenced_draft_and_names_snapshot_references() {
    let db = setup_test_db().await;
    let a = seed_template(&db, &format!("{IT}-DEL-A")).await;
    let b = seed_template(&db, &format!("{IT}-DEL-B")).await;

    db.execute_raw(pg_bound(
        "INSERT INTO contract_clauses (contract_kind, contract_id, clause_code, title, \
         body_rendered, template_id, template_version, sort) \
         VALUES ('sales', $1, 'subject_and_spec', '标的与规格', '固化正文', $2, 1, 1)"
            .to_string(),
        vec![
            Value::Int(Some(IT_SNAPSHOT_CONTRACT_ID)),
            Value::Int(Some(b)),
        ],
    ))
    .await
    .expect("写快照探针行失败");

    let err = svc(&db)
        .delete(b, 9101)
        .await
        .expect_err("被快照引用的草稿删除应被拒绝");
    expect_err(err, "引用", "被引用删除");

    // 未被引用的草稿正常删除，其条款随模板一并清
    svc(&db).delete(a, 9101).await.expect("删除无引用草稿失败");
    let gone = svc(&db)
        .get_by_id(a)
        .await
        .expect_err("删除后应查不到模板行");
    assert!(
        gone.to_string().contains("不存在"),
        "删除后回读应点名不存在"
    );
}

// ---------------------------------------------------------------------------
// 条款输入门 服务层先点名，数据库兜底后归因
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clause_input_gates_name_every_violation() {
    let db = setup_test_db().await;
    let a = seed_template(&db, &format!("{IT}-GATE-A")).await;

    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "notice_and_service".to_string(),
                title: "两处正文".to_string(),
                body: Some("自带正文".to_string()),
                library_clause_id: Some(1),
                required: None,
                sort: 3,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("正文与库引用都给应被拒绝");
    expect_err(err, "只能给一个", "双正文");

    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "notice_and_service".to_string(),
                title: "没有正文".to_string(),
                body: None,
                library_clause_id: None,
                required: None,
                sort: 3,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("两者都不给应被拒绝");
    expect_err(err, "只能给", "无正文");

    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "not_a_clause_code".to_string(),
                title: "越界条款码".to_string(),
                body: Some("正文".to_string()),
                library_clause_id: None,
                required: None,
                sort: 3,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("词表外条款码应被拒绝");
    expect_err(err, "不在允许范围", "越界条款码");

    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "taxes_and_documents".to_string(),
                title: "顺序号越界".to_string(),
                body: Some("正文".to_string()),
                library_clause_id: None,
                required: None,
                sort: 0,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("顺序号 0 应被拒绝");
    expect_err(err, "顺序号", "顺序号越界");

    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "taxes_and_documents".to_string(),
                title: "引用不存在的库行".to_string(),
                body: None,
                library_clause_id: Some(999_999),
                required: None,
                sort: 4,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("引用不存在库行应被拒绝");
    expect_err(err, "不存在", "悬空库引用");

    // 同码同条件并行 拒绝并点名；不同条件 允许
    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "subject_and_spec".to_string(),
                title: "再来一枚通用标的条款".to_string(),
                body: Some("重复正文".to_string()),
                library_clause_id: None,
                required: None,
                sort: 5,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("同码同条件并行行应被拒绝");
    expect_err(err, "并行两行", "同码同条件并行");

    svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "subject_and_spec".to_string(),
                title: "标的条款出口变体".to_string(),
                body: Some("出口变体正文".to_string()),
                library_clause_id: None,
                required: None,
                sort: 6,
                applies_when: Some("export".to_string()),
            },
            9101,
        )
        .await
        .expect("同码不同适用条件应允许并存");

    // 张冠李戴 库行条款码与引用条款码不一致
    db.execute_raw(pg_bound(
        "INSERT INTO clause_library (clause_code, title, body, created_by) \
         VALUES ('payment_and_invoice', '付款与账期', '按 {{payment_terms}} 结算。', 9104)"
            .to_string(),
        vec![],
    ))
    .await
    .expect("播种条款库探针行失败");
    let lib_id = {
        use bingxi_backend::models::clause_library as lib;
        use sea_orm::{ColumnTrait as _, EntityTrait as _};
        let row = lib::Entity::find()
            .filter(lib::Column::ClauseCode.eq("payment_and_invoice"))
            .one(&db)
            .await
            .expect("查库行失败")
            .expect("库探针行查不到");
        row.id
    };
    let err = svc(&db)
        .add_clause(
            a,
            UpsertClauseRequest {
                clause_code: "delivery_and_risk".to_string(),
                title: "张冠李戴引用".to_string(),
                body: None,
                library_clause_id: Some(lib_id),
                required: None,
                sort: 7,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect_err("库行条款码与引用条款码不一致应被拒绝");
    expect_err(err, "张冠李戴", "库引用条款码错配");
}

// ---------------------------------------------------------------------------
// 复核状态机 只有 pending 出边；改文复位 pending 才允许重审
// ---------------------------------------------------------------------------

#[tokio::test]
async fn clause_review_edges_and_text_edit_reset() {
    let db = setup_test_db().await;
    let a = seed_template(&db, &format!("{IT}-REVW-A")).await;
    let cid = svc(&db).list_clause_views(a).await.unwrap()[0].clause.id;

    let err = svc(&db)
        .review_clause(
            a,
            cid,
            ReviewClauseRequest {
                legal_review_status: "pending".to_string(),
                legal_review_remark: None,
            },
            9105,
        )
        .await
        .expect_err("以 pending 作为复核出口应被拒绝");
    expect_err(err, "approved 或 rejected", "pending 复核出口");

    let ok = svc(&db)
        .review_clause(
            a,
            cid,
            ReviewClauseRequest {
                legal_review_status: "approved".to_string(),
                legal_review_remark: Some("合规".to_string()),
            },
            9105,
        )
        .await
        .expect("pending 转 approved 应成功");
    assert_eq!(ok.legal_reviewed_by, Some(9105), "复核人必须真实留痕");

    let err = svc(&db)
        .review_clause(
            a,
            cid,
            ReviewClauseRequest {
                legal_review_status: "rejected".to_string(),
                legal_review_remark: None,
            },
            9105,
        )
        .await
        .expect_err("已出结论的复核不得被覆盖");
    expect_err(err, "不可覆盖", "复核覆盖");

    // 改文 复位 pending 并清留痕，之后才允许重新出结论
    let updated = svc(&db)
        .update_clause(
            a,
            cid,
            UpsertClauseRequest {
                clause_code: "subject_and_spec".to_string(),
                title: "标的与规格".to_string(),
                body: Some("改过的正文 {{product}}。".to_string()),
                library_clause_id: None,
                required: Some(true),
                sort: 1,
                applies_when: None,
            },
            9101,
        )
        .await
        .expect("草稿条款改文应成功");
    assert_eq!(updated.legal_review_status, "pending", "改文必须复位待复核");
    assert_eq!(updated.legal_reviewed_by, None, "改文必须清空复核留痕");
}

// ---------------------------------------------------------------------------
// 表头三态与词表拒绝与列表过滤
// ---------------------------------------------------------------------------

#[tokio::test]
async fn header_tristate_and_vocab_filters() {
    let db = setup_test_db().await;
    let a = seed_template(&db, &format!("{IT}-HDR-A")).await;

    let err = svc(&db)
        .update_header(
            a,
            UpdateTemplateHeaderRequest {
                title: Some(None),
                effective_from: None,
                remark: None,
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 标题清空应被拒绝");
    expect_err(err, "不能清空", "标题清空");

    let updated = svc(&db)
        .update_header(
            a,
            UpdateTemplateHeaderRequest {
                title: Some(Some("改过的标题".to_string())),
                effective_from: Some(Some(chrono::NaiveDate::from_ymd_opt(2026, 12, 1))),
                remark: Some(None),
            },
            9101,
        )
        .await
        .expect("草稿表头三态更新应成功");
    assert_eq!(updated.title, "改过的标题");
    assert!(updated.effective_from.is_some(), "生效日覆盖应落库");
    assert_eq!(updated.remark, None, "显式 null 应真实清空备注");

    // 词表外筛选值点名拒绝，不静默空结果
    let err = svc(&db)
        .list(TemplateListQuery {
            status: Some("enabled".to_string()),
            page: 1,
            page_size: 10,
            ..Default::default()
        })
        .await
        .expect_err("词表外状态筛选应被拒绝");
    expect_err(err, "不在允许范围", "词表外筛选");

    let (rows, _total) = svc(&db)
        .list(TemplateListQuery {
            template_code: Some(format!("{IT}-HDR-A")),
            status: Some("draft".to_string()),
            page: 1,
            page_size: 10,
            ..Default::default()
        })
        .await
        .expect("词表内筛选应成功");
    assert!(
        rows.iter()
            .any(|r| r.template_code == format!("{IT}-HDR-A")),
        "按编码加状态筛选应命中探针行"
    );

    // 创建口词表拒绝
    let err = svc(&db)
        .create(
            CreateTemplateRequest {
                template_code: format!("{IT}-HDR-B"),
                contract_type: "eleventh_type".to_string(),
                party_side: "seller".to_string(),
                title: "越界类别".to_string(),
                effective_from: None,
                remark: None,
                clauses: None,
            },
            9101,
        )
        .await
        .expect_err("词表外类别创建应被拒绝");
    expect_err(err, "contract_type", "词表外类别");
}

// ---------------------------------------------------------------------------
// 数据库最后防线 裸 SQL 归因 CHECK 与部分唯一索引
// ---------------------------------------------------------------------------

#[tokio::test]
async fn db_backstops_reject_bypass_writes_by_constraint_name() {
    let db = setup_test_db().await;

    // 状态词表 CHECK
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'domestic_sales', 'seller', 1, '词表外状态探针', $2)"
                .to_string(),
            vec![Value::String(Some(format!("{IT}-DB-1"))), opt_str(Some("enabled"))],
        ),
        "23514",
        "chk_contract_templates_status",
    )
    .await;

    // active 必须带 approved_by
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_templates (template_code, contract_type, party_side, version, title, status) \
             VALUES ($1, 'domestic_sales', 'seller', 1, '无复核启用探针', 'active')"
                .to_string(),
            vec![Value::String(Some(format!("{IT}-DB-2")))],
        ),
        "23514",
        "chk_contract_templates_active_approved",
    )
    .await;

    // 同组并排写入两版 active 撞部分唯一索引（先建带复核的两行 draft，再同事务外逐条置 active 的
    // 旁路写法：第二条 update 必撞索引）
    let one = seed_template(&db, &format!("{IT}-DB-3")).await;
    make_approvable(&db, one).await;
    svc(&db).activate(one, 9101).await.expect("启用第一版失败");
    let two = seed_template(&db, &format!("{IT}-DB-4")).await;
    let err = db
        .execute_raw(pg_bound(
            "UPDATE contract_templates SET status = 'active', approved_by = 9106, approved_at = now() \
             WHERE id = $1"
                .to_string(),
            vec![Value::Int(Some(two))],
        ))
        .await
        .err()
        .expect("旁路直写第二版 active 必须被库侧拒绝");
    assert_eq!(
        sqlstate_of(&err),
        "23505",
        "旁路并排启用应撞唯一索引，SQLSTATE 归因失守"
    );
    assert_eq!(
        constraint_of(&err),
        "uq_contract_templates_active_type_side",
        "旁路并排启用应归因到 ACTIVE 部分唯一索引本身"
    );

    // 条款正文与库引用双空 撞 chk_ctc_text_source
    expect_rejected(
        &db,
        pg_bound(
            "INSERT INTO contract_template_clauses (template_id, clause_code, title, sort) \
             VALUES ($1, 'notice_and_service', '两源皆空探针', 2)"
                .to_string(),
            vec![Value::Int(Some(one))],
        ),
        "23514",
        "chk_ctc_text_source",
    )
    .await;
}

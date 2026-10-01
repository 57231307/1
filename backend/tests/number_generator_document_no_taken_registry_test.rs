//! 任务 #144：`is_document_no_taken` 集中注册表（`/document-no/check` 查重白名单）
//! 的 doc_type→真实表真实列 一致性契约测试。
//!
//! 根因防复发：历史上查重白名单只有 7 个类型（handler 内嵌 match），新单据
//! 前端取号后查重直接 400，查重形同虚设。现在白名单收敛到
//! `backend/src/utils/number_generator.rs::is_document_no_taken`，本测试锁定：
//! 1. 任务 #144 要求覆盖的每一类 doc_type（销售 SO/采购 PO/报价 QT/销售合同 SC/
//!    采购合同 PC/委外凭证 OVIS·OVFE·OVRC·OVLS/薪酬 WR·PWR/CRM CUS·OPP·TA/
//!    库存单据类）在注册表中登记，且指向**该单据自己的实体与自己的单号列**
//!    （静态源码层断言臂结构，不放宽）；
//! 2. 实体↔真实表名一致（表名以实体元数据为唯一事实来源，与迁移 DDL 对齐）；
//! 3. 活库行为层（`#[ignore]`，由专用 job 在已迁移 PG 上执行）：注册表登记的每个
//!    doc_type 查重都能在真实表真实列上执行 SELECT（不存在的表/列会直接 DB 报错），
//!    并锁定"插入后查得到、删除后查不到"的正反例；
//! 4. 未知 doc_type 必须返回**可区分错误**（BAD_REQUEST，文案含类型名），
//!    绝不静默返回 Ok(false)（"误判可用"——当年白名单形同虚设的复发形态）。
//!
//! 说明（库存四类）：库存域真实拥有自有单号列的单据表为调拨/调整/盘点三张
//! （inventory_transfers/inventory_adjustment/inventory_counts，模型见
//! `src/models/inventory_*.rs` 的 transfer_no/adjustment_no/count_no）；
//! 库存流水/预占/跌价准备/匹等表无自有单号列，故"库存类"以这三张为准逐一断言，
//! 第四个库存单据若未来引入单号列，必须同步登记注册表并加入本清单。

use bingxi_backend::models::{
    crm_opportunity, customer, customer_transfer_approval, inventory_adjustment, inventory_count,
    inventory_transfer, outsourcing_order, outsourcing_receipt, outsourcing_voucher,
    process_wage_rate, purchase_contract, purchase_order, sales_contract, sales_order,
    sales_quotation, wage_record,
};
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::number_generator::is_document_no_taken;
use sea_orm::{EntityName, EntityTrait};

// =========================================================
// 静态层：注册表源码 = doc_type → 实体::Column 的白名单全集
// =========================================================

fn read_gen() -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("src/utils/number_generator.rs");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
}

/// 取注册表 match 中 `"<doc_type>" =>` 臂的源码片段（到下一个臂键为止），
/// 用于断言"该 doc_type 指向哪个实体/列"，避免跨臂误匹配。
fn registry_arm<'s>(src: &'s str, doc_type: &str) -> &'s str {
    let key = format!("\"{doc_type}\" =>");
    let start = src.find(&key).unwrap_or_else(|| {
        panic!(
            "注册表缺少 doc_type「{doc_type}」：/document-no/check 对它会 400，\
         查重对该单据形同虚设（白名单与实际单据脱节的复发形态）。必须在 \
         utils/number_generator.rs::is_document_no_taken 登记"
        )
    });
    let rest = &src[start + key.len()..];
    let end = rest.find("\" =>").unwrap_or(rest.len());
    &rest[..end]
}

/// 任务 #144 点名的必须登记项：(doc_type, 实体, 单号列表达式)。
/// 委外凭证 OVIS/OVFE/OVRC/OVLS 四类前缀同落 outsourcing_voucher.voucher_no；
/// 薪酬 WR→wage_record.record_no、PWR→process_wage_rate.rate_no；
/// CRM CUS→customer.customer_code、OPP→crm_opportunity.opportunity_no、
/// TA→customer_transfer_approvals.approval_no。
const REQUIRED_REGISTRY: &[(&str, &str)] = &[
    ("sales_order", "sales_order::Column::OrderNo.eq(no)"),
    ("purchase_order", "purchase_order::Column::OrderNo.eq(no)"),
    (
        "sales_quotation",
        "sales_quotation::Column::QuotationNo.eq(no)",
    ),
    (
        "sales_contract",
        "sales_contract::Column::ContractNo.eq(no)",
    ),
    (
        "purchase_contract",
        "purchase_contract::Column::ContractNo.eq(no)",
    ),
    ("customer", "customer::Column::CustomerCode.eq(no)"),
    (
        "outsourcing_voucher",
        "outsourcing_voucher::Column::VoucherNo.eq(no)",
    ),
    ("wage_record", "wage_record::Column::RecordNo.eq(no)"),
    (
        "process_wage_rate",
        "process_wage_rate::Column::RateNo.eq(no)",
    ),
    (
        "crm_opportunity",
        "crm_opportunity::Column::OpportunityNo.eq(no)",
    ),
    (
        "customer_transfer_approval",
        "customer_transfer_approval::Column::ApprovalNo.eq(no)",
    ),
    (
        "inventory_transfer",
        "inventory_transfer::Column::TransferNo.eq(no)",
    ),
    (
        "inventory_adjustment",
        "inventory_adjustment::Column::AdjustmentNo.eq(no)",
    ),
    ("inventory_count", "inventory_count::Column::CountNo.eq(no)"),
    // 委外单据主表（OUT/ORC 前端预生成仍依赖查重，历史根因项，保留断言）
    (
        "outsourcing_order",
        "outsourcing_order::Column::OrderNo.eq(no)",
    ),
    (
        "outsourcing_receipt",
        "outsourcing_receipt::Column::ReceiptNo.eq(no)",
    ),
];

#[test]
fn registry_maps_every_required_doc_type_to_its_own_entity_and_no_column() {
    let src = read_gen();
    for (doc_type, col_expr) in REQUIRED_REGISTRY {
        let arm = registry_arm(&src, doc_type);
        assert!(
            arm.contains("::Entity::find()"),
            "doc_type {doc_type} 的注册臂必须走实体查询（真实表），实际臂内容: {arm}"
        );
        assert!(
            arm.contains(col_expr),
            "doc_type {doc_type} 必须匹配其自有单号列 {col_expr}，防止\
             指错表/错列（查重形同虚设的另一种形态），实际臂内容: {arm}"
        );
    }
}

/// 注册表兜底臂必须是"未知 doc_type → 显式可区分错误"，
/// 禁止任何 `_ => false`/默认放行写法把未登记类型误判为"可用"。
#[test]
fn registry_rejects_unknown_doc_types_instead_of_defaulting_to_available() {
    let src = read_gen();
    assert!(
        src.contains("other =>")
            && src.contains("AppError::bad_request(format!(\"未知的单据类型：{}\", other))"),
        "注册表兜底臂必须对未知 doc_type 返回 bad_request（可区分错误），\
         否则 /document-no/check 会把未登记单据误判为可用"
    );
    assert!(
        !src.contains("_ => false") && !src.contains("other => false"),
        "禁止默认放行臂（未登记 doc_type 静默返回可用 = 查重形同虚设的根因形态）"
    );
}

/// 查重 handler 不得再各自维护 doc_type 映射（双白名单漂移根因）。
#[test]
fn check_handler_delegates_to_central_registry() {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("src/handlers/document_no_handler.rs");
    let handler = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 handler 失败: {e}"));
    assert!(
        handler.contains("is_document_no_taken("),
        "查重 handler 必须委托集中注册表"
    );
    assert!(
        !handler.contains("match q.doc_type") && !handler.contains("match &q.doc_type"),
        "查重 handler 不得内嵌 doc_type match（旧白名单漂移根源）"
    );
}

/// 实体↔真实表名一致（表名以实体元数据为准，与迁移 DDL 逐字对齐；
/// 单号列与模型的 *_no 字段对齐由上面的注册臂断言 + 下面活库行为层共同锁定）。
#[test]
fn entities_map_to_expected_real_tables() {
    assert_eq!(sales_order::Entity.table_name(), "sales_orders");
    assert_eq!(purchase_order::Entity.table_name(), "purchase_orders");
    assert_eq!(sales_quotation::Entity.table_name(), "sales_quotations");
    assert_eq!(sales_contract::Entity.table_name(), "sales_contracts");
    assert_eq!(purchase_contract::Entity.table_name(), "purchase_contracts");
    assert_eq!(customer::Entity.table_name(), "customers");
    assert_eq!(
        outsourcing_voucher::Entity.table_name(),
        "outsourcing_voucher"
    );
    assert_eq!(wage_record::Entity.table_name(), "wage_record");
    assert_eq!(process_wage_rate::Entity.table_name(), "process_wage_rate");
    assert_eq!(crm_opportunity::Entity.table_name(), "crm_opportunity");
    assert_eq!(
        customer_transfer_approval::Entity.table_name(),
        "customer_transfer_approvals"
    );
    assert_eq!(
        inventory_transfer::Entity.table_name(),
        "inventory_transfers"
    );
    assert_eq!(
        inventory_adjustment::Entity.table_name(),
        "inventory_adjustments"
    );
    assert_eq!(inventory_count::Entity.table_name(), "inventory_counts");
    assert_eq!(outsourcing_order::Entity.table_name(), "outsourcing_order");
    assert_eq!(
        outsourcing_receipt::Entity.table_name(),
        "outsourcing_receipt"
    );
}

// =========================================================
// 行为层（无需 PG）：未知 doc_type 的误判防复发
// （该路径在触达任何表之前就返回 Err，用 sqlite::memory: 连接即可真实执行）
// =========================================================

#[tokio::test]
async fn unknown_doc_type_returns_distinguishable_error_not_available() {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");

    let err = is_document_no_taken(&db, "totally_unregistered_doc", "SO20260101001")
        .await
        .expect_err(
            "未知 doc_type 必须显式报错；若返回 Ok(false) 即为\"误判可用\"——查重形同虚设的根因形态",
        );
    match &err {
        AppError::BadRequest(m) => {
            assert!(
                m.contains("totally_unregistered_doc"),
                "错误应携带未知类型名便于定位，实际: {m}"
            );
        }
        other => panic!("期望 AppError::BadRequest（可区分错误），实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BAD_REQUEST");
}

// =========================================================
// 行为层（活库 PG，#[ignore] 由专用 job 执行）：
// 每个登记 doc_type 的 SELECT 必须在真实表真实列上可执行；
// customer 做正反例（插入可查得、硬删除后可复用）。
// =========================================================

#[tokio::test]
#[ignore = "需 TEST_DATABASE_URL 已迁移 PostgreSQL（真实表真实列 SELECT），由专用 job（--run-ignored only）执行"]
async fn live_pg_every_registered_doc_type_queries_real_table_and_column() {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("活库用例必须设置 TEST_DATABASE_URL（指向已跑完迁移的 PostgreSQL）");
    let db = sea_orm::Database::connect(&url).await.unwrap();

    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    for (doc_type, _col_expr) in REQUIRED_REGISTRY {
        let sentinel = format!("NGRCHK-{doc_type}-{nanos}");
        let r = is_document_no_taken(&db, doc_type, &sentinel).await;
        match r {
            Ok(false) => {}
            Ok(true) => panic!("doc_type {doc_type} 对全新哨兵号误报已占用（查错列/脏映射）"),
            Err(e) => panic!(
                "doc_type {doc_type} 查重未能在真实表真实列上执行：{e:?}\
                 （Err(BAD_REQUEST)=注册表缺失；DatabaseError=表/列与迁移 schema 不符）"
            ),
        }
    }
}

#[tokio::test]
#[ignore = "需 TEST_DATABASE_URL 已迁移 PostgreSQL（真实插入/删除 customers），由专用 job（--run-ignored only）执行"]
async fn live_pg_customer_registry_detects_taken_and_freed_no() {
    use bingxi_backend::models::customer;
    use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter, Set};

    let url = std::env::var("TEST_DATABASE_URL")
        .expect("活库用例必须设置 TEST_DATABASE_URL（指向已跑完迁移的 PostgreSQL）");
    let db = sea_orm::Database::connect(&url).await.unwrap();

    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let code = format!("NGRCT-{nanos}");
    let now = chrono::Utc::now();
    let row = customer::ActiveModel {
        customer_code: Set(code.clone()),
        customer_name: Set(format!("注册表探针客户 {code}")),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(100),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("探针客户插入失败");

    let taken = is_document_no_taken(&db, "customer", &code)
        .await
        .expect("customer 查重必须可执行");
    assert!(taken, "插入 {code} 后注册表应查得已占用");

    let free = is_document_no_taken(&db, "customer", &format!("{code}-X"))
        .await
        .expect("customer 查重必须可执行");
    assert!(!free, "邻位号未被占用时应返回 false");

    customer::Entity::delete_by_id(row.id)
        .exec(&db)
        .await
        .expect("探针清理失败");
    let after_delete = is_document_no_taken(&db, "customer", &code)
        .await
        .expect("customer 查重必须可执行");
    assert!(
        !after_delete,
        "硬删除后查重应放行（与 allocate_no 的空洞探测语义闭环）"
    );
}

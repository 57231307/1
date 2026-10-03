//! 契约波次 7 · 活库列形态锁（information_schema 断言，run #4671 W1 交付）
//!
//! 终结的静默失效机制：`ALTER TABLE ... ADD COLUMN IF NOT EXISTS ... NOT NULL/DEFAULT/
//! 数组类型` 在**列已存在**时被 PG 恒 no-op 吞掉——迁移文本写了约束/类型，真实列
//! 却是先建域的裸定义（可空标量）。#4671 实证两族：
//! - customers.owner_id：system/mod.rs:229 裸 `INTEGER` 先生效，finance/mod.rs:109 的
//!   `NOT NULL DEFAULT 0` 恒 no-op ⇒ NULL 行使 `customers_isolation` RLS 判据
//!   （owner_id=… OR owner_id=0）两侧皆 unknown，公海/历史行对所有人不可见
//!   （ci4671-triage §2.3 A1 后果 B），且模型非 Option i32 回读炸（p4/p5/p6 三红）；
//! - 模型 `Option<Vec<String>>/Option<Vec<i32>>`（无 `column_type="Json"` 属性）列
//!   实际为标量 TEXT ⇒ 非 NULL 行解码 ColumnDecode（p3 attachment_urls、p2 crm_lead.tags）。
//!
//! 文本比对型锁测抓不到本族（两边文本各自都"对"），因此本锁只认**活库
//! information_schema / pg_catalog 元数据**：迁移链跑完后逐列断言
//! `data_type / udt_name / is_nullable / column_default`，把"写了但没生效"永久钉死。
//! 真库夹具口径：`mod test_common;` + `setup_test_db()`（缺 TEST_DATABASE_URL 直接
//! panic，无 sqlite 回退；本文件不自建任何 DDL）。
//!
//! 覆盖列清单 = m0070_normalize_customers_owner_id（system 域）+
//! m0071_normalize_array_columns（business 域，12 列）的归一目标。
//! 另含两条行为面正向锁：owner_id 缺省写入取公海默认 0（后果 A 修复面）、
//! crm_lead.tags 数组写入-回读无损（ColumnDecode 族修复面；purchase_receipt
//! 族行为面由其既有用例 `live_receipt_update_null_clears_nullable_and_absent_keeps`
//! 转正后自证——本文件不重复造 FK 前置行，元数据面已由本锁钉死）。

mod test_common;

use bingxi_backend::models::{crm_lead, customer};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement, Value,
};

// ===========================================================================
// 元数据锁
// ===========================================================================

/// 期望列形态：(表, 列, data_type, udt_name, is_nullable, column_default 期望)。
/// default 为 Option：None = 必须无默认（'' 哨兵比较），Some(x) = 逐字相等。
struct ColExpect {
    table: &'static str,
    column: &'static str,
    data_type: &'static str,
    udt_name: &'static str,
    is_nullable: &'static str,
    column_default: Option<&'static str>,
    why: &'static str,
}

/// 数组语义列 = PG 数组类型（information_schema 对数组统一报 data_type='ARRAY'）
const ARRAY: &str = "ARRAY";

const EXPECTED: &[ColExpect] = &[
    // ---- m0070：公海语义列（RLS 判据的 NOT NULL 前提） ----
    ColExpect {
        table: "customers",
        column: "owner_id",
        data_type: "integer",
        udt_name: "int4",
        is_nullable: "NO",
        column_default: Some("0"),
        why: "m0070：可空无默认 ⇒ NULL 行使 RLS 公海/归属判据两侧 unknown（#4671 后果 B）",
    },
    // ---- m0071：附件 URL 数组列（写侧 Set(Vec) 活路径，见 m0071 头注释） ----
    ColExpect {
        table: "purchase_receipt",
        column: "attachment_urls",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：models/purchase_receipt.rs:75 Option<Vec<String>>",
    },
    ColExpect {
        table: "ap_invoice",
        column: "attachment_urls",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：models/ap_invoice.rs:91 Option<Vec<String>>",
    },
    ColExpect {
        table: "ap_payment_request",
        column: "attachment_urls",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：models/ap_payment_request.rs:75 Option<Vec<String>>",
    },
    ColExpect {
        table: "ap_payment",
        column: "attachment_urls",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：models/ap_payment.rs:71 Option<Vec<String>>",
    },
    // ---- m0071：CRM 标签/产品数组列（sales_crm/mod.rs:356 成文 TEXT[] 口径） ----
    ColExpect {
        table: "crm_lead",
        column: "tags",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：models/crm_lead.rs:123；production/mod.rs:281 JSONB ADD 恒 no-op 不构成形态依据",
    },
    ColExpect {
        table: "crm_opportunity",
        column: "tags",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：models/crm_opportunity.rs:110",
    },
    ColExpect {
        table: "crm_opportunity",
        column: "product_ids",
        data_type: ARRAY,
        udt_name: "_int4",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：Option<Vec<i32>> ⇒ integer[]",
    },
    ColExpect {
        table: "crm_opportunity",
        column: "product_names",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：Option<Vec<String>>",
    },
    ColExpect {
        table: "crm_opportunity",
        column: "competitor_names",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：Option<Vec<String>>（暂无写点，列型仍须与模型声明一致）",
    },
    // ---- m0071：收货明细匹级数组列（piece 域） ----
    ColExpect {
        table: "purchase_receipt_item",
        column: "internal_piece_ids",
        data_type: ARRAY,
        udt_name: "_int4",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：Option<Vec<i32>> ⇒ integer[]",
    },
    ColExpect {
        table: "purchase_receipt_item",
        column: "internal_piece_nos",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：Option<Vec<String>>",
    },
    ColExpect {
        table: "purchase_receipt_item",
        column: "supplier_piece_nos",
        data_type: ARRAY,
        udt_name: "_text",
        is_nullable: "YES",
        column_default: None,
        why: "m0071：Option<Vec<String>>",
    },
    // ---- m0073：色卡数量列收紧（SHADOWED DEFAULT 族，终结"写了 NOT NULL 却被
    // ADD COLUMN IF NOT EXISTS 吃掉"的静默失效；模型侧是非 Option i32） ----
    ColExpect {
        table: "color_cards",
        column: "stock_quantity",
        data_type: "integer",
        udt_name: "int4",
        is_nullable: "NO",
        column_default: Some("0"),
        why: "m0073：models/color_card.rs:22 i32 非 Option；NULL 行读取即 ColumnNull",
    },
    ColExpect {
        table: "color_cards",
        column: "issued_quantity",
        data_type: "integer",
        udt_name: "int4",
        is_nullable: "NO",
        column_default: Some("0"),
        why: "m0073：发放门 issued_qty <= stock_quantity 的比较依据，不可为 NULL",
    },
];

struct LiveCol {
    data_type: String,
    udt_name: String,
    is_nullable: String,
    column_default: String,
}

async fn live_column(db: &sea_orm::DatabaseConnection, table: &str, column: &str) -> LiveCol {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT data_type, udt_name, is_nullable, COALESCE(column_default, '') \
               FROM information_schema.columns \
              WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2"
                .to_string(),
            vec![
                Value::String(Some(table.to_string())),
                Value::String(Some(column.to_string())),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("查询 information_schema.columns（{table}.{column}）失败: {e}"));
    // 空结果必须炸：表/列缺失时条件过滤断言会静默空转（"锁不存在=锁通过"假绿形态）
    assert_eq!(
        rows.len(),
        1,
        "{table}.{column} 在活库 public schema 中应恰好存在 1 列（0 列=结构漂移，>1=多 schema 泄漏）"
    );
    let r = &rows[0];
    LiveCol {
        data_type: r
            .try_get_by_index::<String>(0)
            .unwrap_or_else(|e| panic!("{table}.{column} data_type 解码失败: {e}")),
        udt_name: r
            .try_get_by_index::<String>(1)
            .unwrap_or_else(|e| panic!("{table}.{column} udt_name 解码失败: {e}")),
        is_nullable: r
            .try_get_by_index::<String>(2)
            .unwrap_or_else(|e| panic!("{table}.{column} is_nullable 解码失败: {e}")),
        column_default: r
            .try_get_by_index::<String>(3)
            .unwrap_or_else(|e| panic!("{table}.{column} column_default 解码失败: {e}")),
    }
}

/// 全量逐列断言（先聚合再报，一轮看全所有漂移，不炸一藏一）
#[tokio::test]
async fn live_columns_match_normalized_shape_in_information_schema() {
    let db = test_common::setup_test_db().await;
    let mut drifts: Vec<String> = Vec::new();
    for exp in EXPECTED {
        let live = live_column(&db, exp.table, exp.column).await;
        let mut check = |name: &str, got: &str, want: &str| {
            if got != want {
                drifts.push(format!(
                    "{}.{} {name}: 实得 {got:?} 期望 {want:?}（{}）",
                    exp.table, exp.column, exp.why
                ));
            }
        };
        check("data_type", &live.data_type, exp.data_type);
        check("udt_name", &live.udt_name, exp.udt_name);
        check("is_nullable", &live.is_nullable, exp.is_nullable);
        match exp.column_default {
            Some(d) => check("column_default", &live.column_default, d),
            None => check("column_default", &live.column_default, ""),
        }
    }
    assert!(
        drifts.is_empty(),
        "活库列形态与迁移归一目标漂移（ADD COLUMN IF NOT EXISTS 当改型用的回潮形态）:\n{}",
        drifts.join("\n")
    );
}

/// owners 面数据不变量：NOT NULL 生效后全表不得存在 owner_id IS NULL 行
/// （即 RLS `owner_id=… OR owner_id=0` 判据的 unknown 行必须为 0——后果 B 的数据面锁）
#[tokio::test]
async fn no_customer_row_can_have_null_owner_id() {
    let db = test_common::setup_test_db().await;
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*)::text FROM customers WHERE owner_id IS NULL".to_string(),
        ))
        .await
        .expect("统计失败");
    assert_eq!(
        rows[0]
            .try_get_by_index::<String>(0)
            .expect("COUNT 解码失败"),
        "0",
        "customers 不得存在 owner_id IS NULL 行（RLS 公海/历史行静默不可见形态回潮）"
    );
}

// ===========================================================================
// 行为面正向锁
// ===========================================================================

/// 后果 A 修复面：写入路径不设 owner_id（ActiveValue NotSet ⇒ INSERT 省略该列）
/// 必须由库默认取 0（公海）并正常回读——#4671 `Missing value for column 'owner_id'`
/// 三红的正向对照锁。禁止改成夹具补 Set(0)（那只会掩盖默认值缺失的库形态）。
#[tokio::test]
async fn customer_insert_without_owner_id_defaults_to_pool_zero() {
    let db = test_common::setup_test_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().expect("nanos");
    let created = customer::ActiveModel {
        customer_code: Set(format!("W7CSH{suffix}")),
        customer_name: Set(format!("列型锁客户-{suffix}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("wholesale".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        // owner_id 故意不设：验证库默认 0 生效（非夹具口径）
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap_or_else(|e| panic!("缺省 owner_id 的客户写入必须成功（默认 0 生效）: {e}"));
    assert_eq!(
        created.owner_id, 0,
        "缺省写入的 owner_id 必须回读为 0（公海），非 Missing value / NULL"
    );

    let reread = customer::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .expect("按主键回读失败")
        .expect("写入行应存在");
    assert_eq!(reread.owner_id, 0, "二次全列解码同样必须通过");
}

/// ColumnDecode 族修复面：模型 Vec<String> ↔ text[] 真实列写入-回读无损
/// （p2 add_tags / p3 attachment 红同型行为锁；tags 为该族写侧最完整样本——
/// 建、改、回读三态都由本列模型承载）。
#[tokio::test]
async fn crm_lead_tags_roundtrip_as_text_array() {
    let db = test_common::setup_test_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().expect("nanos");
    let created = crm_lead::ActiveModel {
        lead_no: Set(format!("W7CTL{suffix}")),
        lead_source: Set("展会".to_string()),
        contact_name: Set(format!("联系人-{suffix}")),
        owner_id: Set(999901),
        owner_name: Set("列型锁负责人".to_string()),
        tags: Set(Some(vec![
            "W1锁".to_string(),
            "text[]".to_string(),
            "含,逗号与引号\"的标签".to_string(),
        ])),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap_or_else(|e| panic!("crm_lead.tags 数组写入失败（列形态回潮标量 TEXT 即此炸）: {e}"));
    assert_eq!(
        created.tags.as_deref(),
        Some(
            &[
                "W1锁".to_string(),
                "text[]".to_string(),
                "含,逗号与引号\"的标签".to_string()
            ][..]
        ),
        "insert 回读应逐元素无损"
    );

    let reread = crm_lead::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .expect("回读失败")
        .expect("写入行应存在");
    assert_eq!(
        reread.tags, created.tags,
        "tags 二次解码不得报 ColumnDecode（TEXT vs Vec<String> 漂移即炸）"
    );

    let empty = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT array_length(tags, 1) FROM crm_lead WHERE id = $1".to_string(),
            vec![Value::Int(Some(created.id))],
        ))
        .await
        .expect("原生回读失败");
    // PG `array_length()` 返回 INT4（integer），按 Option<i64>（INT8）解码会撞
    // 运行期 ColumnDecode "mismatched types; Rust type Option<i64> (as SQL type
    // INT8) is not compatible with SQL type INT4"（#4672 §A.3 绿转红根因，
    // b7e33b52 引入本段时写错解码类型；被测 tags 往返功能本身无涉）。
    assert_eq!(
        empty[0]
            .try_get_by_index::<Option<i32>>(0)
            .expect("array_length 解码失败"),
        Some(3),
        "库内实际就是 3 元素 PG 数组（非把 JSON 文本塞进标量列的假数组）"
    );
}

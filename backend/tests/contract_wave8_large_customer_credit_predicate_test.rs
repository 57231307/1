//! 契约锁：大客户转单审批判据——**行为级正负例**（分层列 tier 落在高档集合是唯一判据）
//!
//! ## 被测对象与判据出处（三处交叉）
//! - **源码**：`backend/src/services/crm/customer_transfer_approval_service.rs` 的
//!   `check_large_customer`——**唯一判据**是 `lead.converted_customer_id` 指向客户的
//!   `customers.tier` 逐字符落在 `backend/src/constants/customer_tier.rs` 的高档集合
//!   `MAJOR`（经 `customer_tier::is_major` 判定；NULL=未定档由 `Option` 通道归为
//!   非大客户）。函数体通读后确认：信用额度阈值比较、预估金额临时分支、旧
//!   `customer_type == "vip"` 分支均已按终裁删除，金额与渠道**不参与**判定。
//!   判定结果经 `create_approval` 落成 `is_large_customer` 与 `max_level`
//!   （大客户=2 级、普通=1 级），并由 `manager_approve` / `director_approve` 驱动层级流转。
//!   判据的源码形状锁（金额形态零残留、MAJOR/is_major 全 src 唯一出现点）在
//!   `tests/contract_wave8_customer_type_validator_parity_test.rs` 的判据锁一节，
//!   与本文件的行为面互为对撞。
//! - **迁移/DB 目录**：`customers.tier VARCHAR(20)` 可空、无 DEFAULT，值域由真 PG 的
//!   CHECK `chk_customers_tier` 收口（`VIP/GOLD/SILVER/NORMAL` 四档 + NULL）；
//!   渠道列 `customer_type` 的 CHECK 与本列词表互不相交——分层词进不了渠道列、
//!   渠道词进不了本列。
//! - **权威词表**：`backend/src/constants/customer_tier.rs` 的 `ALLOWED`/`MAJOR` 是
//!   全源码唯一出现点；本文件用到的档位 token 一律引该模块常量、不手抄第二套清单，
//!   启动即断"所用值 ∈ ALLOWED"（防词表漂移后测的是库早已拒收的值）。
//!
//! ## 通道（为什么这样接）
//! `check_large_customer` 是**私有 async**，本文件不改可见性、也不写源码扫描冒充行为测，
//! 而是走它的**公开入口** `CustomerTransferApprovalService::create_approval`——与生产
//! HTTP 通道完全同构（handler 即 `new(state.db.clone())` 再直呼 `create_approval`）。
//! 因此**不需要** `AppState`，天然规避 `AppState::default()` 把 service 绑 `Disconnected`
//! 哨兵（sea-orm 对哨兵是 panic 而非 Err，fail-closed 分支永不可达＝假绿）这一坑。
//! 断言的真实判据字段名（`is_large_customer`/`max_level`/`approval_status`/
//! `current_level`/`approval_no`）按响应 DTO 实读，不是猜的键名。
//!
//! ## 边界形态钉法（集合成员制对数值阈值线的等价替换）
//! 判据线不再是"数值阈值 + `>`"，而是**集合成员制**，等强度的边界由四类形态承接，
//! 全部真库取证：
//! - **成员内外判别锁**：同一额度（600000.00）的 GOLD（集内）与 SILVER（集外）判定
//!   必须相反、方向确定——证明判定线真的在 tier 上，而不是恒真/恒假造成的"相等"假绿；
//! - **同档噪声等式锁**：同 tier、不同额度（0 vs 六百万）与不同合法渠道 ⇒ 判定逐比特
//!   相同——证明额度与渠道不参与判据（金额分支回潮即在此红）；
//! - **大小写/空白/未知 token 物理不可达锁**：小写 `vip`/`gold`、带尾随空格的
//!   `"VIP "`/`"GOLD "`、词表外 `DIAMOND`、空串，在真库被 `chk_customers_tier` 拒写
//!   （SQLSTATE 23514 + 约束名归因 + 零落库），且 `is_major` 对这些变体逐字符为假——
//!   失真形态既进不了库、也过不了判据，两头各钉一遍；
//! - **NULL=未定档独立钉**：显式 NULL 可落库且回读为真 NULL（无默认档位造数），
//!   行为上必须判非大客户——NULL 不得被消费成 NORMAL，更不得被猜成高档。
//! `credit_limit` 仍逐值真库回读（DECIMAL(12,2) 无损）：此时它的语义是**噪声变量**，
//! 回读断言保证"同额度对"真同额度，判定差异只可能来自 tier。轻量文件表的 TEXT 亲和
//! 会把 `500000.01` 这类定点值解码失真（本仓踩过的真坑），故所有边界一律走真 PG，
//! 定点数不经浮点、分层值逐字符原样落库回读。
//!
//! ## 为什么必须真库
//! - 判据跨 `crm_lead.converted_customer_id` → `customers.tier` 两张真表（FK 真实关联）；
//! - CHECK 拒写形态（大小写/空白/未知 token）与 SQLSTATE 归因只有真 PG 才有——
//!   自建表不加 CHECK 就能把脏 tier 种进判据，mock 路径可以伪造物理上不可能的形态；
//! - 审批单号走统一取号器（advisory lock + 真实取号），空 schema/假表路径与生产不同。
//!
//! ## 参照表约束与 id 带
//! - `roles`/`departments`/`user_role` 属**密封参照表**（`test_common` 的
//!   `SEALED_REFERENCE_TABLES`）：本文件只引用种子部门 `departments.id=1`，不写这些表。
//! - `users`/`customers`/`crm_lead`/`customer_transfer_approvals` 由夹具逐用例 TRUNCATE；
//!   `users` 不被迁移播种，而 `customers.created_by` 有外键，故先建 users。
//! - 本文件独占 **992xxx** id 带（users 992001..992004、customers 992011..992017、
//!   旁路探针 customers 992031..992037、crm_lead 992021..992027），每段先删后插保幂等；
//!   不占用迁移种子主键、不改种子行、不复用其它锁的 id 带。
//! - `crm_lead`/`customers` 上的部门触发器会按 owner 的 `users.department_id` 回填
//!   `department_id`，故 owner 必须是本文件自建的真实 users 行。
//!
//! ## 实读发现（不掩饰，交回编排方裁定）
//! `check_large_customer` 的入参前置是 `lead.converted_customer_id` 非空，而
//! `create_approval` 先经 `fetch_and_validate_lead` 拒掉 `lead_status == converted` 的
//! 线索；生产转化路径会把这两个字段**同时**写（`lead_status=converted` +
//! `converted_customer_id=Some(id)`）⇒ 走正常链路的已转化线索永远在状态门被拦，
//! **到不了大客户判定**；该判定只在「`converted_customer_id` 非空但 `lead_status`
//! 非 converted」这种存量/旁路形态下才可达。本文件正是用真库直接种这种形态，
//! 对**判据本身**做行为锁；"二级审批在生产上是否还有可达路径"属另一个待裁问题，
//! 本文件不替它背书、也不通过放宽断言把它藏起来。
//!
//! ## 陷阱声明
//! - 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向 sqlite 直接 panic，
//!   ⇒ **本文件所有断言都只有 CI 活库才有结果**（本机仅证明编译与格式，未证明行为）。
//! - 一条 lead 只能有一条 pending 审批，所以每个用例各用自己的 lead、独立建审批，
//!   用例之间不共享行、不依赖顺序；无静态全局状态。
//! - 拒绝路径（经理/总监拒绝、层级门拒绝）都不执行实际转移，断言里同时核
//!   `crm_lead.owner_id` **未变**——防"判错层级但把单转了"这种半态绿。

mod test_common;

use bingxi_backend::constants::customer_tier::{
    ALLOWED as TIER_ALLOWED, GOLD, MAJOR, NORMAL, SILVER, VIP, is_major,
};
use bingxi_backend::constants::customer_type::{ALLOWED as CHANNEL_ALLOWED, OTHER};
use bingxi_backend::models::customer_transfer_approval::{STATUS_PENDING, STATUS_REJECTED};
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    ApproveRequest, CreateTransferApprovalRequest, CustomerTransferApprovalService,
    TransferApprovalDto,
};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

/// `customers.tier` 的 DB CHECK 约束名（旁路越界的归因对象，与迁移/词表模块成文同名）
const CHK_TIER_NAME: &str = "chk_customers_tier";

/// 渠道取值（全部必须是 `customer_type::ALLOWED` 成员；在本判据里渠道是纯噪声变量）
const CHANNEL_RETAIL: &str = "retail";
const CHANNEL_WHOLESALE: &str = "wholesale";
const CHANNEL_DISTRIBUTOR: &str = "distributor";
const CHANNEL_MANUFACTURER: &str = "manufacturer";

const USER_OWNER: i32 = 992_001;
const USER_TARGET: i32 = 992_002;
const USER_MANAGER: i32 = 992_003;
const USER_DIRECTOR: i32 = 992_004;

/// 额度爆表、tier=VIP，但**不被任何 lead 指向**的客户行——证明判据载体必须经
/// `converted_customer_id` 外键，不是"库里存在高档客户就人人二级审批"
const UNLINKED_CUSTOMER: i32 = 992_017;
const DIRTY_PROBE_BASE: i32 = 992_031;
/// 大小写漂移、尾随空白混入、词表外新造档、空串——判据永远不该看到这些形态，
/// 在真库层面它们物理上进不了 `customers.tier`
const DIRTY_TIER_TOKENS: &[&str] = &["vip", "gold", "VIP ", "GOLD ", "DIAMOND", ""];
/// 显式 NULL 探针（最后一个脏位之后一个 id）：未定档是合法形态，必须可落且落真 NULL
const NULL_PROBE_CUSTOMER: i32 = DIRTY_PROBE_BASE + 6;

/// 一个判据场景 = 一条 lead + 它 `converted_customer_id` 指向的 customer（None=未挂客户）。
/// `tier` 是**唯一判据输入**；`credit_scaled`（额度×100，DECIMAL(12,2) 定点）与 `channel`
/// 都是刻意摆放的**噪声变量**：判定必须与它们无关。
struct TierCase {
    lead_id: i32,
    customer_id: Option<i32>,
    tier: Option<&'static str>,
    credit_scaled: i64,
    channel: &'static str,
    expect_large: bool,
    /// 该场景钉的是什么（进失败消息，便于判责）
    why: &'static str,
}

const CASES: &[TierCase] = &[
    TierCase {
        lead_id: 992_021,
        customer_id: Some(992_011),
        tier: Some(VIP),
        credit_scaled: 0,
        channel: CHANNEL_RETAIL,
        expect_large: true,
        why: "tier=VIP + 零额度：必须判大客户——判定若还掺入信用额度，这一例先红",
    },
    TierCase {
        lead_id: 992_022,
        customer_id: Some(992_012),
        tier: Some(VIP),
        credit_scaled: 600_000_000, // 6000000.00
        channel: CHANNEL_MANUFACTURER,
        expect_large: true,
        why: "同 VIP、额度放大到六百万且换合法渠道：判定必须与第一例逐比特相同 \
              ⇒ 额度与渠道都不参与判据（金额分支回潮即在此红）",
    },
    TierCase {
        lead_id: 992_023,
        customer_id: Some(992_013),
        tier: Some(GOLD),
        credit_scaled: 60_000_000, // 600000.00——旧信用额度口径的阈值上方面值，现仅作噪声
        channel: CHANNEL_WHOLESALE,
        expect_large: true,
        why: "tier=GOLD（高档集合第二成员）：判大客户的唯一原因是 GOLD ∈ MAJOR",
    },
    TierCase {
        lead_id: 992_024,
        customer_id: Some(992_014),
        tier: Some(SILVER),
        credit_scaled: 60_000_000, // 与上一例**同额度**
        channel: CHANNEL_DISTRIBUTOR,
        expect_large: false,
        why: "同额度、tier=SILVER（集外低档）：不得判大客户——与 GOLD 例构成\
              「同额度、档不同 ⇒ 判定相反」的判别锁；SILVER 混进高档集合即在此红",
    },
    TierCase {
        lead_id: 992_025,
        customer_id: Some(992_015),
        tier: Some(NORMAL),
        credit_scaled: 600_000_000, // 6000000.00，全表最高实档额度
        channel: CHANNEL_RETAIL,
        expect_large: false,
        why: "tier=NORMAL + 额度远超声称的旧金额阈值：不得判大客户——旧额度判据回潮\
              （或把 NORMAL 当『低档里的默认大客户』）即在此红",
    },
    TierCase {
        lead_id: 992_026,
        customer_id: Some(992_016),
        tier: None,
        credit_scaled: 0,
        channel: OTHER,
        expect_large: false,
        why: "tier=NULL（未定档）：无既有评级依据不许猜档，NULL 既不是 NORMAL 也不是\
              任何高档，必须判非大客户；缺省渠道 other 同样不参与判定",
    },
    TierCase {
        lead_id: 992_027,
        customer_id: None,
        tier: None,
        credit_scaled: 0,
        channel: CHANNEL_RETAIL,
        expect_large: false,
        why: "lead 未关联 converted_customer_id（判据的第一道 if）：判据无载体，必须判\
              非大客户——客户行 992017 存在且 tier=VIP、额度爆表，但**不被该 lead 指向**",
    },
];

// ---------------------------------------------------------------------------
// 夹具与助手（真库裸 SQL 一律 execute_raw / query_one_raw，不经轻量表亲和路径）
// ---------------------------------------------------------------------------

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

fn money(scaled: i64) -> Decimal {
    Decimal::new(scaled, 2)
}

/// 驱动上报的 SQLSTATE（不做 to_string().contains 含混匹配）。
fn sqlstate_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名（PG 专用）：把 23514 归因到本列这一条 CHECK。
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 场景种子：users（含经理/总监，供层级流转用）→ customers（含 tier）→ crm_lead。
/// 全部先删后插保幂等；用例级清理由 `setup_test_db()` 的 TRUNCATE 负责。
async fn seed_scene(db: &DatabaseConnection) {
    let lead_ids = CASES
        .iter()
        .map(|c| c.lead_id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    exec(
        db,
        &format!("DELETE FROM crm_lead WHERE id IN ({lead_ids})"),
    )
    .await;
    let mut cust_ids: Vec<String> = CASES
        .iter()
        .filter_map(|c| c.customer_id.map(|i| i.to_string()))
        .collect();
    cust_ids.push(UNLINKED_CUSTOMER.to_string());
    cust_ids.push(NULL_PROBE_CUSTOMER.to_string());
    cust_ids.extend(
        (DIRTY_PROBE_BASE..DIRTY_PROBE_BASE + DIRTY_TIER_TOKENS.len() as i32)
            .map(|i| i.to_string()),
    );
    exec(
        db,
        &format!("DELETE FROM customers WHERE id IN ({})", cust_ids.join(",")),
    )
    .await;
    exec(
        db,
        &format!(
            "DELETE FROM users WHERE id IN ({USER_OWNER},{USER_TARGET},{USER_MANAGER},{USER_DIRECTOR})"
        ),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             ({USER_OWNER},'lc_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET},'lc_target','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_MANAGER},'lc_manager','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_DIRECTOR},'lc_director','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;

    for c in CASES {
        let Some(cid) = c.customer_id else { continue };
        let values: Vec<sea_orm::Value> = vec![
            cid.into(),
            format!("LC-{:02}", c.lead_id % 100).into(),
            money(c.credit_scaled).into(),
            USER_OWNER.into(),
            USER_OWNER.into(),
            c.channel.into(),
            c.tier.into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
             VALUES ($1,$2,'大客户判据行为锁客户',$3,30,'active',$4,$5,$6,$7,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            values,
        ))
        .await
        .unwrap_or_else(|e| {
            panic!(
                "种客户行失败（id={cid} credit={} tier={:?} channel='{}'）: {e}",
                money(c.credit_scaled),
                c.tier,
                c.channel
            )
        });
    }

    // 额度爆表 + tier=VIP 但不被任何 lead 指向：判据必须只认"线索挂的那个客户"的 tier
    let values: Vec<sea_orm::Value> = vec![
        UNLINKED_CUSTOMER.into(),
        "LC-UNLINKED".into(),
        money(999_999_900).into(),
        USER_OWNER.into(),
        USER_OWNER.into(),
        CHANNEL_RETAIL.into(),
        VIP.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
         VALUES ($1,$2,'未被指向的高档客户',$3,30,'active',$4,$5,$6,$7,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .unwrap_or_else(|e| panic!("种未关联客户行失败 id={UNLINKED_CUSTOMER}: {e}"));

    // 线索刻意种成 `lead_status='new'` + `converted_customer_id` 非空：这是文件头
    // 「实读发现」一节说明的**唯一能让 check_large_customer 可达**的行形态
    // （正常转化链路会同时写 lead_status='converted'，在 create_approval 的状态门就被拒）。
    for c in CASES {
        let values: Vec<sea_orm::Value> = vec![
            c.lead_id.into(),
            format!("LD-{}", c.lead_id).into(),
            lead_status::NEW.into(),
            USER_OWNER.into(),
            c.customer_id.into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,converted_customer_id,created_at,updated_at)
             VALUES ($1,$2,'website',$3,'判据公司','张三','13800000000',$4,'源归属人',$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            values,
        ))
        .await
        .unwrap_or_else(|e| panic!("种线索行失败 lead={} : {e}", c.lead_id));
    }
}

async fn lead_owner(db: &DatabaseConnection, lead_id: i32) -> i32 {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT owner_id FROM crm_lead WHERE id={lead_id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 crm_lead.owner_id(lead={lead_id}) 失败: {e}"));
    row.unwrap_or_else(|| panic!("线索 {lead_id} 不存在（种子未生效？）"))
        .try_get::<i32>("", "owner_id")
        .unwrap_or_else(|e| panic!("lead={lead_id} 的 owner_id 解码失败: {e}"))
}

async fn stored_credit_limit(db: &DatabaseConnection, customer_id: i32) -> Decimal {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT credit_limit FROM customers WHERE id={customer_id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 customers.credit_limit(id={customer_id}) 失败: {e}"));
    row.unwrap_or_else(|| panic!("客户 {customer_id} 不存在（种子未生效？）"))
        .try_get::<Decimal>("", "credit_limit")
        .unwrap_or_else(|e| panic!("客户 {customer_id} 的 credit_limit 解码失败: {e}"))
}

/// tier 回读：真 NULL 塌缩成哨兵文本 `<NULL>`（本列合法值全是字母 token，
/// 哨兵不与任何档位撞名；"未定档落真 NULL"与"落某档位"由此可分辨）。
async fn stored_tier(db: &DatabaseConnection, customer_id: i32) -> String {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT COALESCE(tier, '<NULL>') AS tier FROM customers WHERE id={customer_id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 customers.tier(id={customer_id}) 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("customers 里没有 id={customer_id} 的行（种子未生效？）"));
    r.try_get::<String>("", "tier")
        .unwrap_or_else(|e| panic!("id={customer_id} 的 tier 解码失败: {e}"))
}

async fn count_customer_by_id(db: &DatabaseConnection, id: i32) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT COUNT(*) AS n FROM customers WHERE id={id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("计数 customers(id={id}) 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("计数查询没有返回行（id={id}）"));
    r.try_get::<i64>("", "n")
        .unwrap_or_else(|e| panic!("列 n 解码为 i64 失败: {e}"))
}

/// 带显式 `tier`（或 NULL）的旁路直插——绕开全部应用校验，模拟 CHECK 要拦的写入形态。
async fn insert_customer_tier(
    db: &DatabaseConnection,
    id: i32,
    tier: Option<&str>,
) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("TIER-PRED-{id}").into(),
        USER_OWNER.into(),
        USER_OWNER.into(),
        tier.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
         VALUES ($1,$2,'判据旁路探针',0,30,'active',$3,$4,'other',$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

/// 用例开头的词表自检：本文件用的档位/渠道值必须都在权威词表里，
/// 期望值必须与 MAJOR 成员关系自洽——否则测的就不是收口后的那套口径。
fn assert_seeded_vocabularies_are_authoritative() {
    assert_eq!(
        MAJOR,
        &[VIP, GOLD],
        "高档集合必须恰为分层阶梯最高两档 {{VIP, GOLD}}（业务口径唯一真源，\
         改动需与决策方确认）；实际 MAJOR={MAJOR:?}"
    );
    for token in [SILVER, NORMAL] {
        assert!(
            !MAJOR.contains(&token),
            "SILVER/NORMAL 属阶梯低两档，不得进高档集合（否则二级审批口径整体下移）；\
             '{token}' 混入"
        );
    }
    for c in CASES {
        if let Some(t) = c.tier {
            assert!(
                TIER_ALLOWED.contains(&t),
                "场景种子用的档位 token '{t}' 不在分层词表 ALLOWED={TIER_ALLOWED:?} 内\
                （该值会被 DB CHECK 拒，种子阶段就会红）：场景 lead={}",
                c.lead_id
            );
        }
        assert!(
            CHANNEL_ALLOWED.contains(&c.channel),
            "场景种子用的渠道值 '{}' 不在渠道词表 ALLOWED={CHANNEL_ALLOWED:?} 内",
            c.channel
        );
        let by_membership = c.tier.is_some_and(is_major);
        assert_eq!(
            c.expect_large, by_membership,
            "场景 lead={} 的期望值必须与 MAJOR 成员关系自洽（期望 {}，\
             按 tier={:?} ∈ MAJOR 应为 {by_membership}）——期望抄错先在这里红，\
             而不是去污染行为断言",
            c.lead_id, c.expect_large, c.tier
        );
    }
    // 两套词表语义严格分离：分层 token 不在渠道表内，渠道 token 不在分层/高档集合内
    for t in TIER_ALLOWED {
        assert!(
            !CHANNEL_ALLOWED.contains(t),
            "分层 token '{t}' 混进了渠道词表（两列语义必须严格分离）"
        );
    }
    for t in CHANNEL_ALLOWED {
        assert!(
            !TIER_ALLOWED.contains(t),
            "渠道 token '{t}' 混进了分层词表（混维即口径污染）"
        );
    }
}

fn svc(db: &Arc<DatabaseConnection>) -> CustomerTransferApprovalService {
    CustomerTransferApprovalService::new(db.clone())
}

async fn create(db: &Arc<DatabaseConnection>, lead_id: i32) -> TransferApprovalDto {
    svc(db)
        .create_approval(
            CreateTransferApprovalRequest {
                lead_id,
                to_user_id: USER_TARGET,
                reason: "大客户判据行为锁：申请转移".to_string(),
            },
            USER_OWNER,
            "lc_owner",
        )
        .await
        .unwrap_or_else(|e| panic!("创建转移审批失败（lead={lead_id}）: {e}"))
}

// ---------------------------------------------------------------------------
// 1. 正负例总表：判定值 == 由 tier 是否落在 MAJOR 决定的期望，
//    且额度/渠道/tier 全部真库回读逐值相等（种子失真则先在这里炸）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn large_customer_flag_is_determined_by_tier_major_membership_across_all_cases() {
    assert_seeded_vocabularies_are_authoritative();
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    for c in CASES {
        // 前提复核：档位与额度都是**真进了 DB 再读回来**的值
        if let Some(cid) = c.customer_id {
            let stored_credit = stored_credit_limit(&db, cid).await;
            assert_eq!(
                stored_credit,
                money(c.credit_scaled),
                "客户 {cid} 的 credit_limit 回读应等于种子值 {}（DECIMAL(12,2) 无损）；\
                 实际: {stored_credit}\n（不等=列型/精度漂移，噪声变量的摆放就失去意义）",
                money(c.credit_scaled)
            );
            let stored = stored_tier(&db, cid).await;
            let want = c.tier.unwrap_or("<NULL>");
            assert_eq!(
                stored, want,
                "客户 {cid} 的 tier 存储必须逐字符保真：种子 {:?} 实际回读 {stored:?}\n\
                 （落库值变形=写链/列型上出现归一或截断，判据输入已不是本文件声明的那一档）",
                c.tier
            );
        }

        let dto = create(&db, c.lead_id).await;
        let want_max_level: i32 = if c.expect_large { 2 } else { 1 };

        assert_eq!(
            dto.is_large_customer,
            c.expect_large,
            "lead={} 的大客户判定必须为 {}（场景：{}）\n\
             实际 is_large_customer={}，种子 tier={:?}、credit_limit={}、\
             customer_type='{}'、converted_customer_id={:?}\n\
             这条判错就说明判据里又混进了非分层口径（额度/渠道/其它列）或集合成员漂移。",
            c.lead_id,
            c.expect_large,
            c.why,
            dto.is_large_customer,
            c.tier,
            money(c.credit_scaled),
            c.channel,
            c.customer_id
        );
        assert_eq!(
            dto.max_level, want_max_level,
            "lead={} 的审批级数必须与判定同向（大客户=2 级、普通=1 级；\
             源码 `max_level = if is_large_customer {{ 2 }} else {{ 1 }}`）\n\
             期望 max_level={want_max_level}  实际={}（判定={}）",
            c.lead_id, dto.max_level, dto.is_large_customer
        );
        assert_eq!(
            dto.approval_status, STATUS_PENDING,
            "新建审批单必须是 pending（状态词表来源 models/customer_transfer_approval 的 \
             STATUS_PENDING 常量）；lead={} 实际: {}",
            c.lead_id, dto.approval_status
        );
        assert_eq!(
            dto.current_level, 1,
            "新建审批单从第 1 级（销售经理）开始；lead={} 实际 current_level={}",
            c.lead_id, dto.current_level
        );
        assert!(
            dto.approval_no.starts_with("TA"),
            "审批单号必须由统一取号器产出 TA 前缀（证明走的是生产 service 路径、不是被短路）；\
             lead={} 实际 approval_no={:?}",
            c.lead_id,
            dto.approval_no
        );
        assert_eq!(
            dto.lead_id, c.lead_id,
            "回读判据的响应结构：审批单必须挂回发起的 lead；期望 {} 实际 {}",
            c.lead_id, dto.lead_id
        );
    }
}

// ---------------------------------------------------------------------------
// 2. 判别锁 + 噪声等式锁：档位是判据线，额度/渠道不是
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tier_membership_drives_decision_while_amount_and_channel_do_not() {
    assert_seeded_vocabularies_are_authoritative();
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // 同档 VIP、额度 0 vs 六百万、渠道 retail vs manufacturer ⇒ 判定逐比特相同
    let (a, b) = (
        create(&db, CASES[0].lead_id).await,
        create(&db, CASES[1].lead_id).await,
    );
    assert_eq!(
        (a.is_large_customer, a.max_level),
        (b.is_large_customer, b.max_level),
        "同档 VIP、不同额度（{} vs {}）与不同合法渠道的两条判定必须相等；\n\
         不相等 ⇒ 额度或渠道又被当成判据之一（第二口径回潮）；\n\
         实际零额度侧={:?} / 高额度侧={:?}",
        money(CASES[0].credit_scaled),
        money(CASES[1].credit_scaled),
        (a.is_large_customer, a.max_level),
        (b.is_large_customer, b.max_level)
    );

    // 反向对照：同额度 600000.00，GOLD（集内）vs SILVER（集外）⇒ 判定必须**不同**
    //（旧数值阈值"上下 1 分必须相反"的同构替换：证明上一条相等不是恒真/恒假造成的）
    let in_set = create(&db, CASES[2].lead_id).await;
    let out_set = create(&db, CASES[3].lead_id).await;
    assert_eq!(
        stored_credit_limit(&db, CASES[2].customer_id.unwrap()).await,
        stored_credit_limit(&db, CASES[3].customer_id.unwrap()).await,
        "判别锁前置：GOLD 侧与 SILVER 侧的额度必须真库回读相等，\
         否则下面的判定差异可能来自额度而非档位"
    );
    assert_ne!(
        in_set.is_large_customer,
        out_set.is_large_customer,
        "同额度（{}）、档位分别落在高档集合内外（GOLD vs SILVER）的判定必须相反；\
         相同 ⇒ 判据线根本没在 tier 上（恒真/恒假假绿）",
        money(CASES[2].credit_scaled)
    );
    assert!(
        in_set.is_large_customer && !out_set.is_large_customer,
        "且方向必须是「集内为大客户、集外为非大客户」：\
         实际 GOLD 侧={} SILVER 侧={}",
        in_set.is_large_customer,
        out_set.is_large_customer
    );

    // 高额度 + 低档 / 未定档：判定必须与额度无关（金额分支若在，这两条红）
    let loud_amount_low_tier = create(&db, CASES[4].lead_id).await;
    assert!(
        !loud_amount_low_tier.is_large_customer,
        "tier=NORMAL + 额度 {}（远超终裁前的临时金额阈值）仍必须判非大客户；\
         为真 ⇒ 旧额度判据以别名或常量形式回潮",
        money(CASES[4].credit_scaled)
    );
    let loud_amount_null_tier = create(&db, CASES[5].lead_id).await;
    assert!(
        !loud_amount_null_tier.is_large_customer,
        "tier=NULL（未定档）必须判非大客户，无论其它列摆什么；\
         为真 ⇒ 未定档被兜底猜成了某个档位"
    );
}

// ---------------------------------------------------------------------------
// 3. 失真钉：大小写/空白/未知 token/NULL 四形态各自断言（纯函数面 + 真库面）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dirty_tier_variants_never_reach_the_predicate() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // 纯函数面：高档谓词逐字符敏感——不 trim、不归一大小写，变体与词表外值全为假
    assert!(is_major(VIP) && is_major(GOLD), "集内两档必须命中高档谓词");
    for not_major in [
        "vip", "gold", "VIP ", " GOLD", "normal", "silver", "DIAMOND", "", NORMAL, SILVER,
    ] {
        assert!(
            !is_major(not_major),
            "'{not_major}' 不是高档集合成员（大小写/空白变体与词表外值都不许命中；\
             归一化变体进判据就是把脏值静默吞进二级审批）"
        );
    }

    // 真库面：脏形态物理上进不了 customers.tier——旁路直插被 CHECK 拒，且归因明确、零落库
    for (idx, token) in DIRTY_TIER_TOKENS.iter().enumerate() {
        let id = DIRTY_PROBE_BASE + idx as i32;
        let err = insert_customer_tier(&db, id, Some(token))
            .await
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "旁路直插 tier='{token}' 竟成功——库层兜底失效（小写/尾随空白/未知档/空串 \
                     任何一侧漂移都会让脏值进得了库、进而污染判据输入）"
                )
            });
        assert_eq!(
            sqlstate_of(&err).as_deref(),
            Some("23514"),
            "旁路写 '{token}' 必须撞 CHECK 成 check_violation（SQLSTATE 23514）；\
             只断 is_err 是假绿：期望 23514 实际 SQLSTATE={:?}\n原始错误: {err}",
            sqlstate_of(&err)
        );
        assert_eq!(
            constraint_of(&err).as_deref(),
            Some(CHK_TIER_NAME),
            "23514 必须归因到 {CHK_TIER_NAME} 本约束（只断状态码会把表上别的 CHECK 在挡\
             误当本锁生效）；实际约束名={:?}\n原始错误: {err}",
            constraint_of(&err)
        );
        assert_eq!(
            count_customer_by_id(&db, id).await,
            0,
            "被 CHECK 拒掉的 '{token}' 写入必须零落库（单语句原子失败，不留半行）"
        );
    }

    // NULL 形态对照：未定档是**合法**存储形态，必须可落且落真 NULL（不许被默认值造档）
    insert_customer_tier(&db, NULL_PROBE_CUSTOMER, None)
        .await
        .unwrap_or_else(|e| {
            panic!("显式 tier=NULL（未定档）必须可落——CHECK 的 IS NULL 放行形态: {e}")
        });
    assert_eq!(
        stored_tier(&db, NULL_PROBE_CUSTOMER).await,
        "<NULL>",
        "显式 NULL 探针必须落真 NULL（被默认值/写链兜底改写=未定档形态失真，\
         判据的『NULL 判非大客户』前提随之崩塌）"
    );
}

// ---------------------------------------------------------------------------
// 4. 层级流转：高档进总监层且不转移；集外档不可达总监层；拒绝路径零转移
// ---------------------------------------------------------------------------

#[tokio::test]
async fn major_tier_routes_to_director_level_and_denies_bypass() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // --- 高档（VIP、零额度）：经理通过 → 进第 2 级（总监层），**不得**在此刻完成转移
    let large = create(&db, CASES[0].lead_id).await;
    assert!(
        large.is_large_customer,
        "前置条件失效：lead={}（tier=VIP）应当是大客户",
        CASES[0].lead_id
    );
    let owner_before = lead_owner(&db, CASES[0].lead_id).await;
    let after_manager = svc(&db)
        .manager_approve(
            ApproveRequest {
                approval_id: large.id,
                comment: "经理同意，转总监复核".to_string(),
                approved: true,
            },
            USER_MANAGER,
            "lc_manager",
            &scope_all_ctx(),
        )
        .await
        .unwrap_or_else(|e| panic!("经理审批通过失败（大客户单 {}）: {e}", large.approval_no));
    assert_eq!(
        after_manager.current_level, 2,
        "高档大客户经理通过后必须进入总监审批层；实际 current_level={}",
        after_manager.current_level
    );
    assert_eq!(
        after_manager.approval_status, STATUS_PENDING,
        "大客户在第 2 级仍未终态；实际: {}",
        after_manager.approval_status
    );
    assert_eq!(
        lead_owner(&db, CASES[0].lead_id).await,
        owner_before,
        "大客户仅经理通过时**不得**执行转移（判据走错层级就会把单转掉，形成半态绿）"
    );

    // 总监拒绝 → rejected，且仍不转移
    let after_director = svc(&db)
        .director_approve(
            ApproveRequest {
                approval_id: large.id,
                comment: "总监不同意".to_string(),
                approved: false,
            },
            USER_DIRECTOR,
            "lc_director",
            &scope_all_ctx(),
        )
        .await
        .unwrap_or_else(|e| panic!("总监拒绝审批失败（大客户单 {}）: {e}", large.approval_no));
    assert_eq!(
        after_director.approval_status, STATUS_REJECTED,
        "总监拒绝后审批单终态必须是 rejected（models/customer_transfer_approval 的 \
         STATUS_REJECTED 词表值）；实际: {}",
        after_director.approval_status
    );
    assert_eq!(
        lead_owner(&db, CASES[0].lead_id).await,
        owner_before,
        "任意层级拒绝后不得执行转移；归属人应保持 {owner_before}"
    );

    // --- 集外档（SILVER、额度 600000.00）：不可达总监层（行为锁——旧额度判据会把这条
    //     当成大客户送进第 2 级；判据纠正后总监直呼必须被层级门拒）
    let out_of_set = create(&db, CASES[3].lead_id).await;
    assert_eq!(
        out_of_set.max_level, 1,
        "前置条件失效：lead={}（tier=SILVER，同额度 GOLD 侧为大客户）应是非大客户",
        CASES[3].lead_id
    );
    // `.err()` + 点名 panic：把"返回了 Ok"和"返回了别的错误"分成两种可判读的红，
    // 禁止 `assert!(res.is_err())` 这种把任何错误都当预期错误的假绿形态。
    let err = svc(&db)
        .director_approve(
            ApproveRequest {
                approval_id: out_of_set.id,
                comment: "越级尝试".to_string(),
                approved: true,
            },
            USER_DIRECTOR,
            "lc_director",
            &scope_all_ctx(),
        )
        .await
        .err()
        .unwrap_or_else(|| {
            panic!(
                "非大客户（tier=SILVER、credit_limit={}）的总监审批必须被拒——\
                 若被放行，说明二级审批入口重新按额度计数",
                money(CASES[3].credit_scaled)
            )
        });
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "总监越级审批必须归 VALIDATION_ERROR 族（AppError::validation 层级门；\
         若被拍平成 DATABASE_ERROR/INTERNAL_ERROR 就是族别漂移）；实际 code={} 消息={err}",
        err.error_code()
    );

    // --- 集外档（NORMAL、额度六百万）：经理拒绝 → rejected，不转移
    let low_tier = create(&db, CASES[4].lead_id).await;
    let owner_low_before = lead_owner(&db, CASES[4].lead_id).await;
    let rejected = svc(&db)
        .manager_approve(
            ApproveRequest {
                approval_id: low_tier.id,
                comment: "经理不同意".to_string(),
                approved: false,
            },
            USER_MANAGER,
            "lc_manager",
            &scope_all_ctx(),
        )
        .await
        .unwrap_or_else(|e| panic!("经理拒绝审批失败（普通单 {}）: {e}", low_tier.approval_no));
    assert_eq!(
        rejected.approval_status, STATUS_REJECTED,
        "普通客户经理拒绝后必须是 rejected；实际: {}",
        rejected.approval_status
    );
    assert_eq!(
        lead_owner(&db, CASES[4].lead_id).await,
        owner_low_before,
        "普通客户拒绝路径同样不得执行转移（保持 {owner_low_before}）"
    );
}

/// 审批流用例的范围夹具：All 范围（主管复核可读全域），只锁状态机与层级门，
/// 不在此文件重复验证行级归属；行级归属由归属门用例覆盖。
fn scope_all_ctx() -> bingxi_backend::utils::data_scope::DataScopeContext {
    bingxi_backend::utils::data_scope::DataScopeContext {
        scope: bingxi_backend::utils::data_scope::DataScope::All,
        user_id: 1,
        department_id: None,
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    }
}

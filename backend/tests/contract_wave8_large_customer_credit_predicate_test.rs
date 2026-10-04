//! 契约波次 8 · #259 大客户转单审批判据——**行为级正负例**（信用额度是唯一判据）
//!
//! ## 被测对象与判据出处（三处交叉）
//! - **源码**：`backend/src/services/crm/customer_transfer_approval_service.rs`
//!   `check_large_customer`（:525-539）——**唯一判据**是
//!   `lead.converted_customer_id` 指向的 `customer.credit_limit >
//!   DEFAULT_LARGE_CUSTOMER_CREDIT_THRESHOLD`（:35，`i64 = 500_000`，**模块私有 const**，
//!   本文件只能按源码数值复述，见下方「与编排方前提的差异」）；函数体通读后确认
//!   **不再** 出现任何 `customer_type` 比较（旧 `=="vip"` 分支已在 `9838ef77` 删除）。
//!   判定结果经 `create_approval`（:155-156）落成 `is_large_customer` +
//!   `max_level = if is_large {2} else {1}`，并由 `manager_approve`（:312-354）
//!   / `director_approve`（:372-438）驱动层级流转。
//! - **迁移/DB 目录**：`customers.customer_type` 在库上被 CHECK 锁成渠道五值
//!   （`migration/src/domain/business/m0078_finalize_customer_type_domain.rs:119-120`，
//!   约束名 `chk_customers_customer_type`），分层词 `vip` **物理上进不了本列**
//!   ——由 `tests/contract_wave8_customer_type_domain_live_lock_test.rs` 钉死；本文件
//!   据此把「旧 vip 分支不可达」这一前提也在同一条真库路径上复核一次。
//! - **权威词表**：`backend/src/constants/customer_type.rs:64 ALLOWED`（渠道五值）；
//!   本文件用到的渠道值全部要求是 `ALLOWED` 成员（启动即断，防词表漂移后本文件测的是
//!   一个库早已拒收的值）。
//!
//! ## 通道（为什么这样接）
//! `check_large_customer` 是**私有 async**，本文件不改可见性、也不写源码扫描锁冒充行为测，
//! 而是走它的**公开入口** `CustomerTransferApprovalService::create_approval`
//! （pub，:144）——与生产 HTTP 通道 `handlers/customer_transfer_approval_handler.rs:33`
//! 完全同构：handler 里就是 `CustomerTransferApprovalService::new(state.db.clone())`
//! 再调 `create_approval(req, auth.user_id, &auth.username)`，服务只依赖 `db`。
//! ⇒ 因此**不需要** `AppState`，也就天然规避了 `AppState::default()` 把 40+ service
//! 绑 `Disconnected` 哨兵（sea-orm 对哨兵是 panic 而非 Err，fail-closed 分支永不可达＝假绿）
//! 这一坑；若将来该判据只能通过 HTTP 层触发，按
//! `contract_wave8_crm_read_gate_bypass_test.rs:244-250` 的 `state_from(db)`（`db.clone()`
//! + 真 service）起 Router，禁止 `AppState::default()`。
//! 断言的真实判据字段名是从函数体读出来的：`is_large_customer`（bool）与
//! `max_level`（i32，1=单级、2=需总监二级），不是猜的键名。
//!
//! ## 为什么必须真库
//! - 判据要跨 `crm_lead.converted_customer_id` → `customers.credit_limit` 两张真表；
//!   `credit_limit` 是 **DECIMAL(12,2)** 列（system/m0001:333），与阈值比较发生在
//!   DB 存回的 `Decimal` 上：sqlite 夹具的 TEXT 亲和会把 `500000.01` 变成解码失真
//!   （CI #4669 约 130 例的根因），边界值（恰等阈值/上下 1 分）在假表上根本不可信。
//! - `customers.customer_type` 的 CHECK 只有真 PG 才有——「渠道列永远没有 vip、
//!   旧分支不可达」这条前提在 mock/自建表上是可以被伪造的（自建表不加 CHECK 就能插 vip）。
//! - 审批单号走 `utils/number_generator.rs` 的 advisory lock + 真实取号（:157-167），
//!   空 schema/假表下这条路径与生产不同。
//!
//! ## 参照表约束与 id 带
//! - `roles`/`departments`/`user_role` 属**密封参照表**（`src/services/test_common.rs`
//!   `SEALED_REFERENCE_TABLES`）：本文件只引用种子部门 `departments.id=1`，不写这些表。
//! - `users`/`customers`/`crm_lead`/`customer_transfer_approvals` 是逐用例 TRUNCATE 的
//!   业务表；`users` 不被迁移播种，而 `customers.created_by` 有外键
//!   （`models/customer.rs:123-129`），故先建 users。
//! - 本文件独占 **992xxx** id 带（users 992001/992002、customers 992011..992017、
//!   crm_lead 992021..992027），每段先删后插保幂等；不占用迁移种子主键、不改种子行。
//! - `crm_lead`/`customers` 上的 `trg_*_dept` 触发器会按 owner 的 `users.department_id`
//!   回填 `department_id`，故 owner 必须是本文件自建的真实 users 行。
//!
//! ## 实读发现（不掩饰，交回编排方裁定）
//! `check_large_customer` 的入参前置是 `lead.converted_customer_id` 非空
//! （service :526），而 `create_approval` 在 :151-153 先经
//! `fetch_and_validate_lead` 拒掉 `lead_status == 'converted'` 的线索（:212-218）。
//! 生产转化路径 `services/crm/lead.rs:749-752` 把这两个字段**同时**写
//! （`lead_status=converted` + `converted_customer_id=Some(id)`）
//! ⇒ 走正常链路的已转化线索永远在「已转化为客户，无法转移」处被拦，
//! **到不了大客户判定**；该判定只在「`converted_customer_id` 非空但
//! `lead_status` ≠ converted」这种存量/旁路形态下才可达。
//! 本文件正是用真库直接种这种形态，对**判据本身**做行为锁（判据=信用额度、渠道不参与）；
//! 「二级审批在生产上是否还有可达路径」属另一个待裁问题，本文件不替它背书、
//! 也不通过放宽断言把它藏起来。
//!
//! ## 陷阱声明
//! - 夹具 `setup_test_db()` 缺 `TEST_DATABASE_URL`/指向 sqlite 直接 panic，
//!   ⇒ **本文件所有断言都只有 CI 活库才有结果**（本机仅证明编译与格式，未证明行为）。
//! - 一条 lead 只能有一条 pending 审批（服务 :229-248），所以每个用例各用自己的 lead，
//!   用例之间不共享行、不依赖顺序；无静态全局状态。
//! - 拒绝路径（经理/总监拒绝、层级门拒绝）都不执行实际转移，因此断言里同时核
//!   `crm_lead.owner_id` **未变**——防「判错层级但把单转了」这种半态绿。

mod test_common;

use bingxi_backend::constants::customer_type::{ALLOWED, OTHER};
use bingxi_backend::models::customer_transfer_approval::{STATUS_PENDING, STATUS_REJECTED};
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    ApproveRequest, CreateTransferApprovalRequest, CustomerTransferApprovalService,
    TransferApprovalDto,
};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

/// 源码 `customer_transfer_approval_service.rs:35` 的私有常量数值（**复述**，不是第二套口径：
/// 该 const 无 `pub`，测试无法引用；本文件按源码字面值复述并在用例里以边界三点钉住它的作用线，
/// 若源码改了阈值而不同步本文件，恰等阈值/上下 1 分三例必红——见交付报告的差异说明）
const CREDIT_THRESHOLD: i64 = 500_000;
/// DECIMAL(12,2) 的定点表示：`scaled` = 金额 × 100（不经浮点，杜绝解析失真）
const THRESHOLD_SCALED: i64 = CREDIT_THRESHOLD * 100;

/// 渠道取值（全部必须是 `ALLOWED` 成员，用例开头即断言；不另立词表）
const CHANNEL_RETAIL: &str = "retail";
const CHANNEL_WHOLESALE: &str = "wholesale";
const CHANNEL_DISTRIBUTOR: &str = "distributor";
const CHANNEL_MANUFACTURER: &str = "manufacturer";

const USER_OWNER: i32 = 992_001;
const USER_TARGET: i32 = 992_002;
const USER_MANAGER: i32 = 992_003;
const USER_DIRECTOR: i32 = 992_004;

/// 一个判据场景 = 一条 lead + 它 converted_customer_id 指向的 customer（NULL 表示未关联客户）
struct CreditCase {
    lead_id: i32,
    customer_id: Option<i32>,
    /// 信用额度 ×100（DECIMAL(12,2) 定点）
    credit_scaled: i64,
    channel: &'static str,
    expect_large: bool,
    /// 该场景钉的是什么（进失败消息，便于判责）
    why: &'static str,
}

const CASES: &[CreditCase] = &[
    CreditCase {
        lead_id: 992_021,
        customer_id: Some(992_011),
        credit_scaled: 60_000_000, // 600000.00
        channel: CHANNEL_RETAIL,
        expect_large: true,
        why: "高于阈值 + 渠道 retail：必须判大客户（旧实现要靠 customer_type='vip' 才可达，\
              本列物理上装不下 vip，判据只剩信用额度）",
    },
    CreditCase {
        lead_id: 992_022,
        customer_id: Some(992_012),
        credit_scaled: 60_000_000, // 600000.00，与上一例**同额度**
        channel: CHANNEL_MANUFACTURER,
        expect_large: true,
        why: "同额度、不同合法渠道：判定必须与上一例完全相同 ⇒ 渠道不参与判据（第二口径回潮即在此红）",
    },
    CreditCase {
        lead_id: 992_023,
        customer_id: Some(992_013),
        credit_scaled: 50_000_001, // 500000.01 = 阈值上 1 分
        channel: CHANNEL_DISTRIBUTOR,
        expect_large: true,
        why: "阈值上 1 分（严格 `>` 的下边界外侧）：必须判大客户",
    },
    CreditCase {
        lead_id: 992_024,
        customer_id: Some(992_014),
        credit_scaled: 50_000_000, // 恰等阈值
        channel: CHANNEL_WHOLESALE,
        expect_large: false,
        why: "恰等阈值：源码是 `>` 不是 `>=`，不得判大客户（将判据写成 `>=` 即在此红）",
    },
    CreditCase {
        lead_id: 992_025,
        customer_id: Some(992_015),
        credit_scaled: 49_999_999, // 499999.99 = 阈值下 1 分
        channel: CHANNEL_DISTRIBUTOR,
        expect_large: false,
        why: "阈值下 1 分 + 合法渠道 distributor：不得判大客户（这条就是防 `vip` 分支回潮的\
              行为锁——库里永远没有 vip，靠渠道词翻盘的旧路径必须整体不可达）",
    },
    CreditCase {
        lead_id: 992_026,
        customer_id: Some(992_016),
        credit_scaled: 0,
        channel: OTHER,
        expect_large: false,
        why: "零额度 + 缺省渠道 other：不得判大客户（other 不是任何意义上的大客户标记）",
    },
    CreditCase {
        lead_id: 992_027,
        customer_id: None,
        credit_scaled: 999_999_900, // 9999999.00，额度极高但没有客户行可参照
        channel: CHANNEL_RETAIL,
        expect_large: false,
        why: "lead 未关联 converted_customer_id（源码 :526 的第一道 if）：判据无载体，\
              必须判非大客户——客户行 992017 存在但**不被该 lead 指向**",
    },
];

// ---------------------------------------------------------------------------
// 夹具与助手
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

/// 场景种子：users（含经理/总监，供层级流转用）→ customers → crm_lead。
/// 全部先删后插，用例内自清由 `setup_test_db()` 的 TRUNCATE 负责，这里的 DELETE
/// 只为「同一用例内多次重建」与本地 `cargo test` 共享进程场景保幂等。
async fn seed_scene(db: &DatabaseConnection) {
    let ids = CASES
        .iter()
        .filter_map(|c| c.customer_id.map(|i| i.to_string()))
        .collect::<Vec<_>>()
        .join(",");
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
    if !ids.is_empty() {
        exec(db, &format!("DELETE FROM customers WHERE id IN ({ids})")).await;
    }
    exec(
        db,
        &format!("DELETE FROM users WHERE id IN ({USER_OWNER},{USER_TARGET},{USER_MANAGER},{USER_DIRECTOR})"),
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
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
             VALUES ($1,$2,'大客户判据客户',$3,30,'active',$4,$5,$6,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            values,
        ))
        .await
        .unwrap_or_else(|e| {
            panic!(
                "种客户行失败（id={cid} credit={} channel='{}'）: {e}",
                money(c.credit_scaled),
                c.channel
            )
        });
    }

    // 992017：额度爆表但不被任何 lead 指向——证明「不是有额度就算，必须经 lead 的客户外键」
    let unlinked_id: i32 = 992_017;
    let values: Vec<sea_orm::Value> = vec![
        unlinked_id.into(),
        "LC-UNLINKED".into(),
        money(999_999_900).into(),
        USER_OWNER.into(),
        USER_OWNER.into(),
        CHANNEL_RETAIL.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
         VALUES ($1,$2,'未被指向的大额度客户',$3,30,'active',$4,$5,$6,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .unwrap_or_else(|e| panic!("种未关联客户行失败 id={unlinked_id}: {e}"));

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

/// 用例开头的词表自检：本文件写的渠道值必须都在权威词表里，
/// 否则测的就不是收口后的那套值（漂移会在建客户行时被 CHECK 打死，但消息不可读）。
fn assert_channels_are_in_allowed() {
    for t in [
        CHANNEL_RETAIL,
        CHANNEL_WHOLESALE,
        CHANNEL_DISTRIBUTOR,
        CHANNEL_MANUFACTURER,
        OTHER,
    ] {
        assert!(
            ALLOWED.contains(&t),
            "本文件用的渠道值 '{t}' 不在权威词表 ALLOWED={ALLOWED:?} 内——\
             词表已漂移，请先同步 constants 与本文件"
        );
    }
    for c in CASES {
        assert!(
            ALLOWED.contains(&c.channel),
            "场景种子用的渠道值 '{}' 不在 ALLOWED 内（该值现在会被 DB CHECK 拒，\
             种子阶段就会红）：场景 lead={}",
            c.channel,
            c.lead_id
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
// 1. 正负例总表：判定值 == 由 credit_limit 与阈值决定的期望
// ---------------------------------------------------------------------------

#[tokio::test]
async fn large_customer_flag_is_determined_by_credit_limit_across_all_cases() {
    assert_channels_are_in_allowed();
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    for c in CASES {
        // 前提复核：额度是**真进了 DB 再读回来**的值（DECIMAL(12,2) 无损）
        if let Some(cid) = c.customer_id {
            let stored = stored_credit_limit(&db, cid).await;
            assert_eq!(
                stored,
                money(c.credit_scaled),
                "客户 {cid} 的 credit_limit 回读应等于种子值 {}（DECIMAL(12,2) 无损）；\
                 实际: {stored}\n（不等=列型/精度漂移，本用例后面的阈值比较就失去意义）",
                money(c.credit_scaled)
            );
        }

        let dto = create(&db, c.lead_id).await;
        let want_max_level: i32 = if c.expect_large { 2 } else { 1 };

        assert_eq!(
            dto.is_large_customer,
            c.expect_large,
            "lead={} 的大客户判定必须为 {}（场景：{}）\n\
             实际 is_large_customer={}，种子 credit_limit={}、customer_type='{}'、\
             converted_customer_id={:?}，阈值={}\n\
             这条判错就说明判据里又混进了非信用额度的口径（渠道词/其它列）或阈值比较写反。",
            c.lead_id,
            c.expect_large,
            c.why,
            dto.is_large_customer,
            money(c.credit_scaled),
            c.channel,
            c.customer_id,
            money(THRESHOLD_SCALED)
        );
        assert_eq!(
            dto.max_level, want_max_level,
            "lead={} 的审批级数必须与判定同向（大客户=2 级、普通=1 级；源码 :156 \
             `max_level = if is_large_customer {{ 2 }} else {{ 1 }}`）\n\
             期望 max_level={want_max_level}  实际={}（判定={}）",
            c.lead_id, dto.max_level, dto.is_large_customer
        );
        assert_eq!(
            dto.approval_status, STATUS_PENDING,
            "新建审批单必须是 pending（状态词表来源 models/customer_transfer_approval.rs:15-21）；\
             lead={} 实际: {}",
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
// 2. 判据与渠道无关的**等式**锁：同额度不同合法渠道 ⇒ 判定逐比特相同
// ---------------------------------------------------------------------------

#[tokio::test]
async fn identical_credit_limit_gives_identical_decision_regardless_of_channel() {
    assert_channels_are_in_allowed();
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // CASES[0] 与 CASES[1] 刻意同额度（600000.00）不同渠道（retail / manufacturer）
    let a = create(&db, CASES[0].lead_id).await;
    let b = create(&db, CASES[1].lead_id).await;
    assert_eq!(
        (a.is_large_customer, a.max_level),
        (b.is_large_customer, b.max_level),
        "同额度、不同合法渠道的两条判定必须相等（'{CHANNEL_RETAIL}' 侧 vs \
         '{CHANNEL_MANUFACTURER}' 侧）；\n\
         不相等 ⇒ customer_type 又被当成判据之一（旧 vip 分支的第二口径回潮）；\n\
         实际 retail 侧={:?} / manufacturer 侧={:?}",
        (a.is_large_customer, a.max_level),
        (b.is_large_customer, b.max_level)
    );

    // 反向对照：阈值上下 1 分的两例判定必须**不同**（证明上一条相等不是「恒真/恒假」造成的）
    let just_above = create(&db, CASES[2].lead_id).await; // 500000.01
    let just_below = create(&db, CASES[4].lead_id).await; // 499999.99
    assert_ne!(
        just_above.is_large_customer,
        just_below.is_large_customer,
        "阈值上下各 1 分的判定必须相反（{} vs {}）；相同 ⇒ 阈值这条线根本没在比，\
         上一段的「相等」是假绿",
        money(CASES[2].credit_scaled),
        money(CASES[4].credit_scaled)
    );
    assert!(
        just_above.is_large_customer && !just_below.is_large_customer,
        "且方向必须是「上为大客户、下为非大客户」（源码 :533 `c.credit_limit > threshold`）：\
         实际 above={} below={}",
        just_above.is_large_customer,
        just_below.is_large_customer
    );
}

// ---------------------------------------------------------------------------
// 3. 层级流转：大客户进总监层且不转移；非大客户不可达总监层；拒绝路径零转移
// ---------------------------------------------------------------------------

#[tokio::test]
async fn large_customer_routes_to_director_level_and_denies_bypass() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // --- 大客户：经理通过 → 进第 2 级（总监层），**不得**在此刻完成转移
    let large = create(&db, CASES[0].lead_id).await;
    assert!(
        large.is_large_customer,
        "前置条件失效：lead={} 应当是大客户（credit_limit={} > 阈值 {}）",
        CASES[0].lead_id,
        money(CASES[0].credit_scaled),
        money(THRESHOLD_SCALED)
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
        )
        .await
        .unwrap_or_else(|e| panic!("经理审批通过失败（大客户单 {}）: {e}", large.approval_no));
    assert_eq!(
        after_manager.current_level, 2,
        "大客户经理通过后必须进入总监审批层（源码 :344 `current_level = Set(2)`）；\
         实际 current_level={}",
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
        )
        .await
        .unwrap_or_else(|e| panic!("总监拒绝审批失败（大客户单 {}）: {e}", large.approval_no));
    assert_eq!(
        after_director.approval_status, STATUS_REJECTED,
        "总监拒绝后审批单终态必须是 rejected（models/customer_transfer_approval.rs:19）；\
         实际: {}",
        after_director.approval_status
    );
    assert_eq!(
        lead_owner(&db, CASES[0].lead_id).await,
        owner_before,
        "任意层级拒绝后不得执行转移（服务文件头 :13 成文）；归属人应保持 {owner_before}"
    );

    // --- 非大客户：不可达总监层（行为锁——旧实现只有「渠道= vip」才可能翻进第 2 级）
    let small = create(&db, CASES[4].lead_id).await;
    assert_eq!(
        small.max_level, 1,
        "前置条件失效：lead={} 应是非大客户",
        CASES[4].lead_id
    );
    // `.err()` + 点名 panic：把「返回了 Ok」和「返回了别的错误」分成两种可判读的红，
    // 禁止 `assert!(res.is_err())` 这种把任何错误都当预期错误的假绿形态。
    let err = svc(&db)
        .director_approve(
            ApproveRequest {
                approval_id: small.id,
                comment: "越级尝试".to_string(),
                approved: true,
            },
            USER_DIRECTOR,
            "lc_director",
        )
        .await
        .err()
        .unwrap_or_else(|| {
            panic!(
                "非大客户（渠道 distributor、credit_limit={}，恰在阈值下 1 分）的总监审批必须被拒",
                money(CASES[4].credit_scaled)
            )
        });
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "总监越级审批必须归 VALIDATION_ERROR 族（AppError::validation，源码 :561-565 层级门；\
         若被拍平成 DATABASE_ERROR/INTERNAL_ERROR 就是族别漂移）；实际 code={} 消息={err}",
        err.error_code()
    );

    // --- 非大客户：经理拒绝 → rejected，不转移
    let small2 = create(&db, CASES[3].lead_id).await;
    let owner2_before = lead_owner(&db, CASES[3].lead_id).await;
    let rejected = svc(&db)
        .manager_approve(
            ApproveRequest {
                approval_id: small2.id,
                comment: "经理不同意".to_string(),
                approved: false,
            },
            USER_MANAGER,
            "lc_manager",
        )
        .await
        .unwrap_or_else(|e| panic!("经理拒绝审批失败（普通单 {}）: {e}", small2.approval_no));
    assert_eq!(
        rejected.approval_status, STATUS_REJECTED,
        "普通客户经理拒绝后必须是 rejected；实际: {}",
        rejected.approval_status
    );
    assert_eq!(
        lead_owner(&db, CASES[3].lead_id).await,
        owner2_before,
        "普通客户拒绝路径同样不得执行转移（保持 {owner2_before}）"
    );
}

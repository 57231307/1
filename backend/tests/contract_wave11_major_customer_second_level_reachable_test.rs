//! 大客户转单二级审批**分层判据**行为锁(高档正向可达 + 低档/未定档不触发)
//!
//! ## 锁定的判据(源码出处)
//! `backend/src/services/crm/customer_transfer_approval_service.rs` `check_large_customer`:
//! **唯一判据** = 线索关联客户(`crm_lead.converted_customer_id`,FK 指向 customers)的
//! 分层列 `customers.tier` 落在高档集合 `constants::customer_tier::MAJOR`(与 DB CHECK
//! `chk_customers_tier` 逐值同源)。预估金额、信用额度等其它维度不再参与判定(那是
//! 终裁前的临时口径,本锁同时钉死"金额高但无关联客户 ⇒ 非大客户"防回潮)。
//!
//! ## 判据形态迁移说明(旧锁 ↔ 新锁,判据变更而非掩盖)
//! 本文件旧版钉的是"未转化线索行自身预估金额 > 500000 ⇒ 大客户"的临时金额口径;
//! 用户终裁改以**客户分层列**为大客户唯一权威载体后,判定输入只剩 `customers.tier`,
//! 正/负向线索形态随之从"按金额分档"改为"按所挂客户的分层档位分档"。断言强度未降:
//! 高档集合成员逐一行为验证、集合边界(GOLD 触发/SILVER 不触发)点名钉死、
//! tier NULL 形态独立钉、原"恰等阈值"式边界由**集合成员边界**承接、
//! 二级门拦转移与总监通过才转移的链路断言原样保留。
//!
//! ## 证明形态
//! 走与生产 HTTP 通道完全同构的服务路径(`handlers/customer_transfer_approval_handler.rs`
//! 即 `CustomerTransferApprovalService::new(state.db.clone())` 后直呼同名方法),
//! 对**真实转换链路**逐级断言,而非只断 2xx:
//! - 正向:所挂客户分层为高档集合成员的线索 → `create_approval` 判大客户(max_level=2)→
//!   经理通过后 `current_level=2` 且审批单仍 pending 且 `crm_lead.owner_id` **未变**
//!   (证明转移被二级门拦住)→ 总监通过后审批单 approved 且 `owner_id` 才变更为目标人、
//!   `to_user_name` 回写为真实新用户名。
//! - 负向:分层为高档集合外成员(SILVER/NORMAL)或 NULL(未定档)的线索 → max_level=1;
//!   未挂客户的线索即使预估金额高于旧金额口径的阈值也判非大客户(第二口径防回潮锁);
//!   非大客户单总监层直呼被拒(只断机器码 VALIDATION_ERROR,不断外显文案);
//!   经理通过即单级完成转移、层级保持 1。
//!
//! ## 为什么必须真库
//! 判据跨 `crm_lead` → `customers` 两真表(经 FK `fk_crm_lead_customer` 关联),且种子行
//! 的 `tier` 值必须真实穿过 CHECK `chk_customers_tier`(迁移未注册/未生效时四值种子同样
//! 插得进,但契约锁文件另有活库目录对撞兜底);审批单号走 `utils/number_generator` 真实
//! 取号;转移经 `transfer_lead` 真事务写 `assignment_history`。夹具 `setup_test_db()` 缺
//! `TEST_DATABASE_URL` 或指向 sqlite 直接 panic——本文件所有行为断言只有 CI 活库才有结果。
//!
//! ## 参照表与 id 带
//! - `roles`/`departments`/`user_role` 属密封参照表:只引用种子部门 `departments.id=1`。
//! - 本文件独占 **993xxx** id 带(users 993001..993004、customers 993041..993045、
//!   crm_lead 993021..993026),每用例先删后插保幂等(删序:先线索后客户,FK 子→父);
//!   不占用迁移种子主键、不复用其它锁的 id 带。
//! - 期望的"哪些 token 算高档"一律取运行时 `constants::customer_tier::MAJOR/is_major`,
//!   测试内不手抄第二套成员清单(边界成员点名断言除外,它防两侧一起改错)。

mod test_common;

use bingxi_backend::constants::customer_tier::{self, ALLOWED, GOLD, MAJOR, NORMAL, SILVER, VIP};
use bingxi_backend::models::customer_transfer_approval::{STATUS_APPROVED, STATUS_PENDING};
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    ApproveRequest, CreateTransferApprovalRequest, CustomerTransferApprovalService,
    TransferApprovalDto,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

const USER_OWNER: i32 = 993_001;
const USER_TARGET: i32 = 993_002;
const USER_MANAGER: i32 = 993_003;
const USER_DIRECTOR: i32 = 993_004;

/// 各分层 token 对应的客户行 id(与客户行一一对应)
fn customer_of(token: &str) -> i32 {
    match token {
        VIP => 993_041,
        GOLD => 993_042,
        SILVER => 993_043,
        NORMAL => 993_044,
        other => panic!("词表出现未预置客户行的 token {other:?}(ALLOWED 被单边扩了)"),
    }
}

/// 各分层 token 对应的线索行 id(lead → 上述客户经 converted_customer_id 关联)
fn lead_of(token: &str) -> i32 {
    match token {
        VIP => 993_021,
        GOLD => 993_022,
        SILVER => 993_023,
        NORMAL => 993_024,
        other => panic!("词表出现未预置线索行的 token {other:?}(ALLOWED 被单边扩了)"),
    }
}

/// 客户 tier 为 NULL(未定档)的线索/客户对
const LEAD_TIER_NULL: i32 = 993_025;
const CUSTOMER_TIER_NULL: i32 = 993_045;
/// 未挂客户(converted_customer_id NULL)但预估金额高于旧金额口径阈值的线索——
/// 第二口径防回潮锁:分层判据下必须判非大客户
const LEAD_NO_CUSTOMER_HIGH_AMOUNT: i32 = 993_026;

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 场景种子:users → customers(每个 ALLOWED token 一行 + tier NULL 一行)→
/// crm_lead(逐一经 converted_customer_id 挂到对应客户;另有一条未挂客户的高金额线索)。
/// 线索状态取 'new'(创建审批的状态门只拒 'converted',挂客户行经 FK 真实可建)。
/// 先删后插保幂等,删序 子(线索)→父(客户)。
async fn seed_scene(db: &DatabaseConnection) {
    exec(
        db,
        &format!(
            "DELETE FROM crm_lead WHERE id IN ({},{},{},{},{},{})",
            lead_of(VIP),
            lead_of(GOLD),
            lead_of(SILVER),
            lead_of(NORMAL),
            LEAD_TIER_NULL,
            LEAD_NO_CUSTOMER_HIGH_AMOUNT
        ),
    )
    .await;
    exec(
        db,
        &format!(
            "DELETE FROM customers WHERE id IN ({},{},{},{},{})",
            customer_of(VIP),
            customer_of(GOLD),
            customer_of(SILVER),
            customer_of(NORMAL),
            CUSTOMER_TIER_NULL
        ),
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
             ({USER_OWNER},'mc3_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET},'mc3_target','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_MANAGER},'mc3_manager','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_DIRECTOR},'mc3_director','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;

    // 客户行:每个 ALLOWED token 一行(tier 值必须真实穿过 chk_customers_tier),
    // 外加一行 tier NULL(未定档)。credit_limit 一律 0——判据不再读它,
    // 高额度+低分层必须判非大客户(旧信用额度口径防回潮)。
    let mut tier_cases: Vec<(i32, Option<&str>)> = ALLOWED
        .iter()
        .map(|token| (customer_of(token), Some(*token)))
        .collect();
    tier_cases.push((CUSTOMER_TIER_NULL, None));
    for (customer_id, tier) in tier_cases {
        let values: Vec<sea_orm::Value> = vec![
            customer_id.into(),
            format!("CUST-{customer_id}").into(),
            USER_OWNER.into(),
            USER_OWNER.into(),
            tier.into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,tier,created_at,updated_at)
             VALUES ($1,$2,'分层判据锁客户',0,30,'active',$3,$4,'other',$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            values,
        ))
        .await
        .unwrap_or_else(|e| panic!("种客户行失败 customer={customer_id} tier={tier:?}: {e}"));
    }

    // 线索行:逐一挂对应客户;最后一条不挂客户但预估金额 500001.00
    //(高于旧金额口径阈值——若判据回潮读金额,此条会误判大客户,在这里红)。
    let mut lead_cases: Vec<(i32, i32, Option<&str>)> = ALLOWED
        .iter()
        .map(|token| (lead_of(token), customer_of(token), Some(*token)))
        .collect();
    lead_cases.push((LEAD_TIER_NULL, CUSTOMER_TIER_NULL, None));
    for (lead_id, customer_id, tier) in lead_cases {
        let values: Vec<sea_orm::Value> = vec![
            lead_id.into(),
            format!("LD-{lead_id}").into(),
            lead_status::NEW.into(),
            USER_OWNER.into(),
            customer_id.into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,estimated_amount,converted_customer_id,created_at,updated_at)
             VALUES ($1,$2,'website',$3,'分层判据锁公司','李四','13900000000',$4,'源归属人',NULL,$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            values,
        ))
        .await
        .unwrap_or_else(|e| panic!("种线索行失败 lead={lead_id} tier={tier:?}: {e}"));
    }
    exec(
        db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,estimated_amount,converted_customer_id,created_at,updated_at)
             VALUES ({LEAD_NO_CUSTOMER_HIGH_AMOUNT},'LD-{LEAD_NO_CUSTOMER_HIGH_AMOUNT}','website','{new}','高金额无客户公司','王五','13900000001',{USER_OWNER},'源归属人',500001.00,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            new = lead_status::NEW
        ),
    )
    .await;
}

async fn lead_owner(db: &DatabaseConnection, lead_id: i32) -> i32 {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT owner_id FROM crm_lead WHERE id={lead_id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 crm_lead.owner_id(lead={lead_id}) 失败: {e}"));
    row.unwrap_or_else(|| panic!("线索 {lead_id} 不存在(种子未生效?)"))
        .try_get::<i32>("", "owner_id")
        .unwrap_or_else(|e| panic!("lead={lead_id} 的 owner_id 解码失败: {e}"))
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
                reason: "分层判据锁:申请转移".to_string(),
            },
            USER_OWNER,
            "mc3_owner",
        )
        .await
        .unwrap_or_else(|e| panic!("创建转移审批失败(lead={lead_id}): {e}"))
}

async fn approve(
    db: &Arc<DatabaseConnection>,
    approval_id: i32,
    approved: bool,
    comment: &str,
    as_manager: bool,
) -> TransferApprovalDto {
    let req = ApproveRequest {
        approval_id,
        comment: comment.to_string(),
        approved,
    };
    let result = if as_manager {
        svc(db)
            .manager_approve(req, USER_MANAGER, "mc3_manager")
            .await
    } else {
        svc(db)
            .director_approve(req, USER_DIRECTOR, "mc3_director")
            .await
    };
    result.unwrap_or_else(|e| panic!("审批操作失败(approval={approval_id}): {e}"))
}

/// 高档集合成员边界点名(防两侧一起改错):集合恰为最高两档,
/// GOLD=触发侧边界成员、SILVER=不触发侧边界成员。
#[test]
fn major_tier_set_boundary_membership_is_named() {
    assert_eq!(
        MAJOR,
        &[VIP, GOLD],
        "高档集合必须恰为分层阶梯最高两档 {{VIP, GOLD}}(业务口径唯一真源,\
         改动需与决策方确认);实际 MAJOR={MAJOR:?}"
    );
    assert!(
        customer_tier::is_major(GOLD) && !customer_tier::is_major(SILVER),
        "集合边界点名:GOLD 必须触发、SILVER 必须不触发(二者是相邻档,任何一侧漂移\
         都会让二级审批口径整体偏移)"
    );
}

/// 正向可达锁:高档集合成员的每个 token(VIP、GOLD)——所挂客户分层命中的线索,
/// 二级审批**必须**在生产路径上真实触发;其中 VIP 走全链(拦转移→总监通过才转)。
#[tokio::test]
async fn major_tier_customer_lead_reaches_director_level_and_transfer_executes_only_after_it() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // ① 创建即判大客户:判据唯一由 customers.tier ∈ MAJOR 产出,与金额/额度无关
    for token in MAJOR {
        let approval = create(&db, lead_of(token)).await;
        assert!(
            approval.is_large_customer,
            "所挂客户分层 '{token}'(高档集合成员)的线索必须判大客户——若为 false,\
             说明判据没有只读分层列或 MAJOR 集合漂移(token={token})",
        );
        assert_eq!(
            approval.max_level, 2,
            "大客户审批级数必须为 2(源码 `max_level = if is_large_customer {{ 2 }} else {{ 1 }}`);\
             token='{token}' 实际 max_level={}",
            approval.max_level
        );
        assert_eq!(
            approval.current_level, 1,
            "新建审批单从经理层开始;实际 current_level={}",
            approval.current_level
        );
        assert!(
            approval.approval_no.starts_with("TA"),
            "审批单号必须经统一取号器产出 TA 前缀(证明走生产 service 路径);实际 {:?}",
            approval.approval_no
        );
    }

    // ② 经理通过 → 进入总监层,此刻**不得**发生转移(取 VIP 线索走全链)
    let lead = lead_of(VIP);
    let owner_before = lead_owner(&db, lead).await;
    assert_eq!(
        owner_before, USER_OWNER,
        "种子前置:线索归属人应为 {USER_OWNER}"
    );
    let approval = create(&db, lead).await;
    let after_manager = approve(&db, approval.id, true, "经理同意,转总监复核", true).await;
    assert_eq!(
        after_manager.current_level, 2,
        "大客户经理通过后必须进入总监审批层(源码 `current_level = Set(2)`);实际 current_level={}",
        after_manager.current_level
    );
    assert_eq!(
        after_manager.approval_status, STATUS_PENDING,
        "总监层未批前审批单必须仍 pending;实际 {}",
        after_manager.approval_status
    );
    assert_eq!(
        lead_owner(&db, lead).await,
        owner_before,
        "仅经理通过时不得执行转移——二级审批门若形同虚设(经理层直接转单)在此红"
    );

    // ③ 总监通过 → 审批完成且转移此刻才真实执行
    let after_director = approve(&db, approval.id, true, "总监复核通过", false).await;
    assert_eq!(
        after_director.approval_status, STATUS_APPROVED,
        "总监通过后审批单终态必须是 approved;实际 {}",
        after_director.approval_status
    );
    assert_eq!(
        after_director.director_approver_id,
        Some(USER_DIRECTOR),
        "总监审批人必须回写为实际操作人 {USER_DIRECTOR}"
    );
    assert!(
        after_director.completed_at.is_some(),
        "总监通过后必须落最终完成时间"
    );
    assert_eq!(
        lead_owner(&db, lead).await,
        USER_TARGET,
        "总监通过后线索归属人才应变更为目标人 {USER_TARGET}(转移经 transfer_lead 真实执行)"
    );
    assert_eq!(
        after_director.to_user_name.as_deref(),
        Some("mc3_target"),
        "to_user_name 必须是 transfer_lead 解析出的真实新用户名,不得造名或留空;实际 {:?}",
        after_director.to_user_name
    );
}

/// 负向锁:高档集合外的每个 token(SILVER/NORMAL)、tier NULL(未定档)、
/// 未挂客户的高金额线索,三形态均不触发二级审批;非大客户单总监层直呼被拒;
/// 经理通过即单级完成转移。
#[tokio::test]
async fn non_major_tier_never_enters_second_level() {
    let db = Arc::new(setup_test_db().await);
    seed_scene(&db).await;

    // 集合内低档:每个非 MAJOR 成员(含边界档 SILVER)必须判非大客户
    for token in ALLOWED {
        let approval = create(&db, lead_of(*token)).await;
        assert_eq!(
            (approval.is_large_customer, approval.max_level),
            (
                customer_tier::is_major(token),
                if customer_tier::is_major(token) { 2 } else { 1 }
            ),
            "分层 '{token}' 的判定必须与 MAJOR 集合成员关系逐 token 一致(期望\
             is_major={} 实际 {:?});出现分叉即判据或集合漂移",
            customer_tier::is_major(token),
            (approval.is_large_customer, approval.max_level)
        );
    }

    // tier NULL:未定档不是低档,绝不许被当成 VIP/NORMAL 猜——必须判非大客户
    let tier_null = create(&db, LEAD_TIER_NULL).await;
    assert_eq!(
        (tier_null.is_large_customer, tier_null.max_level),
        (false, 1),
        "客户分层 NULL(未定档)必须判非大客户——把未定档兜底成任何档位都是造数;实际 {:?}",
        (tier_null.is_large_customer, tier_null.max_level)
    );

    // 第二口径防回潮:未挂客户的线索,预估金额高于旧金额阈值也只许判非大客户
    let no_customer = create(&db, LEAD_NO_CUSTOMER_HIGH_AMOUNT).await;
    assert_eq!(
        (no_customer.is_large_customer, no_customer.max_level),
        (false, 1),
        "未挂客户的线索(estimated_amount=500001.00,高于终裁前的临时金额阈值)必须判\
         非大客户——判据只读分层列;若此条为真,说明金额分支回潮形成第二套口径;实际 {:?}",
        (no_customer.is_large_customer, no_customer.max_level)
    );

    // 非大客户单总监层直呼必须被拒(层级门);只断状态族机器码
    let err = svc(&db)
        .director_approve(
            ApproveRequest {
                approval_id: tier_null.id,
                comment: "越级尝试".to_string(),
                approved: true,
            },
            USER_DIRECTOR,
            "mc3_director",
        )
        .await
        .err()
        .unwrap_or_else(|| panic!("非大客户单(current_level=1)的总监审批必须被拒"));
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "层级门拒绝必须归 VALIDATION_ERROR 族;实际 code={}",
        err.error_code()
    );

    // 经理通过即单级完成:审批单 approved 且转移直接执行、层级从未到过 2
    let owner_before = lead_owner(&db, LEAD_TIER_NULL).await;
    let completed = approve(&db, tier_null.id, true, "经理直接完成", true).await;
    assert_eq!(
        completed.current_level, 1,
        "非大客户单经理通过后层级必须保持 1(不进总监层);实际 {}",
        completed.current_level
    );
    assert_eq!(
        completed.approval_status, STATUS_APPROVED,
        "非大客户经理通过即终态 approved;实际 {}",
        completed.approval_status
    );
    assert_eq!(
        owner_before, USER_OWNER,
        "转移前置:线索归属人应保持种子值 {USER_OWNER}"
    );
    assert_eq!(
        lead_owner(&db, LEAD_TIER_NULL).await,
        USER_TARGET,
        "非大客户单经理通过后应立即完成转移(单级即可达终态)"
    );
}

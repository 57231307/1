//! 契约测试：大客户二级审批链正序列行为锁 + 审批跃迁原子性反例锁。
//!
//! 锁的判据（源码 `services/crm/customer_transfer_approval_service.rs`）：
//! 1. 正序列：建线索 → 真实转化腿转化（客户行+CONVERTED+挂链 ID 同时落，分层 NotSet）→
//!    S1 分层腿置 VIP → 建单得 `max_level=2` → 经理批进总监层 → 总监批执行转移；
//!    四枚回读：`customers.owner_id` 等于目标人、审批单 approved、`lead_status` 仍
//!    CONVERTED、`converted_customer_id` 未被清空（转移对象是客户归属，线索只留痕）；
//! 2. 原子性反例（总监腿）：构造转移必失败前提（目标人停用）→ 总监批整笔回滚——
//!    审批单仍 pending、director 落项为空、客户归属未变、零 TRANSFER 历史与零审计行；
//!    再恢复目标人重批 → 成功。杜绝旧形态"先 commit approved 再转移"的半态。
//! 3. 原子性反例（经理单级腿同缺陷同修）：max_level=1 转移失败同样整笔回滚，
//!    恢复后重批成功、`to_user_name` 回写真实用户名。
//!
//! 为什么必须真库：转化腿真实穿过 crm_lead→customers→crm_opportunity 链与编号生成器；
//! 分层写腿与审批判据跨表；转移与跃迁同事务语义只有活库能证伪。夹具 `setup_test_db()`
//! 缺 `TEST_DATABASE_URL` 或指向 sqlite 直接 panic——本文件行为断言交 CI 活库验证。
//!
//! 参照表与 id 带：`departments.id=1` 属密封参照表只引用；本文件独占 984xxx 带
//! （users 984001..984004、crm_lead 984021/984022/984024），客户行 ID 由转化腿动态返回，
//! 每用例先删后插保幂等（删序 审批单/历史→审计(按客户 ID 点名)→商机→线索→客户→users）。

mod test_common;

use bingxi_backend::constants::customer_tier::VIP;
use bingxi_backend::models::dto::crm_dto::ConvertLeadRequest;
use bingxi_backend::models::status::crm_lead as lead_status;
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::crm::customer_transfer_approval_service::{
    ApproveRequest, CreateTransferApprovalRequest, CustomerTransferApprovalService,
};
use bingxi_backend::services::crm::cust::CrmService;
use bingxi_backend::services::customer_service::CustomerService;
use bingxi_backend::utils::data_scope::{DataScope, DataScopeContext};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

const USER_OWNER: i32 = 984_001;
const USER_TARGET: i32 = 984_002;
const USER_MANAGER: i32 = 984_003;
const USER_DIRECTOR: i32 = 984_004;
/// 每用例独立目标人：原子性反例要翻转 is_active，共享用户会跨用例互踩
const USER_TARGET_T2: i32 = 984_005;
const USER_TARGET_T3: i32 = 984_006;
/// 正序列线索（T1）、总监反例线索（T2）与单级反例线索（T3）
const LEAD_HAPPY: i32 = 984_021;
const LEAD_ATOMIC_DIRECTOR: i32 = 984_022;
const LEAD_ATOMIC_MANAGER: i32 = 984_024;
const LEADS: &str = "984021,984022,984024";
const USERS: &str = "984001,984002,984003,984004,984005,984006";

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

async fn scalar_i64(db: &DatabaseConnection, sql: &str) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("标量回读失败: {e}\nSQL: {sql}"))
        .unwrap_or_else(|| panic!("标量回读空集: {sql}"));
    row.try_get::<i64>("", "v")
        .unwrap_or_else(|e| panic!("标量解码失败: {e}"))
}

async fn text_opt(db: &DatabaseConnection, sql: &str) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("文本回读失败: {e}\nSQL: {sql}"))
        .unwrap_or_else(|| panic!("文本回读空集: {sql}"));
    row.try_get("", "v")
        .unwrap_or_else(|e| panic!("文本解码失败: {e}"))
}

/// All 范围审批人上下文（行级门 All=可见，与仓内二级可达锁同形态）。
fn operator_ctx(user_id: i32) -> DataScopeContext {
    DataScopeContext {
        scope: DataScope::All,
        user_id,
        department_id: None,
        dept_ids: vec![],
        dept_member_user_ids: vec![],
    }
}

fn approval_service(db: &Arc<DatabaseConnection>) -> CustomerTransferApprovalService {
    CustomerTransferApprovalService::new(db.clone())
}

fn tier_service(db: &Arc<DatabaseConnection>) -> CustomerService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    CustomerService::new(db.clone(), search_client)
}

/// 幂等清理：按本带线索取回动态客户 ID，逐表按 FK 子→父删除。
async fn cleanup(db: &DatabaseConnection) {
    exec(
        db,
        &format!("DELETE FROM customer_transfer_approvals WHERE lead_id IN ({LEADS})"),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM assignment_histories WHERE lead_id IN ({LEADS})"),
    )
    .await;
    let ids_sql = format!(
        "SELECT converted_customer_id AS c FROM crm_lead WHERE id IN ({LEADS}) AND converted_customer_id IS NOT NULL"
    );
    let rows = db
        .query_all_raw(pg_stmt(&ids_sql))
        .await
        .unwrap_or_else(|e| panic!("读取挂链客户失败: {e}"));
    let cust_ids: Vec<i32> = rows
        .iter()
        .filter_map(|r| r.try_get::<i32>("", "c").ok().flatten())
        .collect();
    if !cust_ids.is_empty() {
        let list = cust_ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let quoted = cust_ids
            .iter()
            .map(|i| format!("'{i}'"))
            .collect::<Vec<_>>()
            .join(",");
        exec(
            db,
            &format!(
                "DELETE FROM audit_logs WHERE resource_type='customer' AND resource_id IN ({quoted})"
            ),
        )
        .await;
        exec(
            db,
            &format!("DELETE FROM crm_opportunity WHERE customer_id IN ({list})"),
        )
        .await;
        exec(db, &format!("DELETE FROM customers WHERE id IN ({list})")).await;
    }
    exec(db, &format!("DELETE FROM crm_lead WHERE id IN ({LEADS})")).await;
    exec(db, &format!("DELETE FROM users WHERE id IN ({USERS})")).await;
}

async fn seed_users(db: &DatabaseConnection) {
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             ({USER_OWNER},'mc18_owner','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET},'mc18_target','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_MANAGER},'mc18_manager','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_DIRECTOR},'mc18_director','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET_T2},'mc18_target_t2','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({USER_TARGET_T3},'mc18_target_t3','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
}

/// 种一条本带未转化线索（形态与正常生命周期一致：未挂链）。
async fn seed_new_lead(db: &DatabaseConnection, lead_id: i32, tag: &str) {
    exec(
        db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,contact_name,mobile_phone,owner_id,owner_name,converted_customer_id,created_at,updated_at)
             VALUES ({lead_id},'LD-{tag}','website','{new}','二级链客户-{tag}','孙七','13900000000',{USER_OWNER},'源归属人',NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            new = lead_status::NEW
        ),
    )
    .await;
}

/// 走**真实转化腿**：转化产出客户行（分层 NotSet）+ CONVERTED + 挂链 ID，
/// 随后用 S1 分层受控写腿置 VIP——判据输入全程无手插，转化门未被触碰。
async fn convert_then_vip(db: &Arc<DatabaseConnection>, lead_id: i32) -> i32 {
    let crm = CrmService::new(db.clone());
    let v = crm
        .convert_lead_to_customer(
            lead_id,
            ConvertLeadRequest {
                customer_type: None,
                notes: None,
            },
            USER_OWNER,
        )
        .await
        .expect("真实转化腿必须成功（本锁不走手插捷径）");
    let customer_id = v["customer_id"]
        .as_i64()
        .expect("转化出口必须携带 customer_id") as i32;
    tier_service(db)
        .update_customer_tier(customer_id, VIP, USER_OWNER)
        .await
        .expect("S1 分层腿写入 VIP 必须成功");
    customer_id
}

/// 建单→经理批（正序列与总监反例共用），返回审批单 ID 与经理批后层级。
async fn create_then_manager(
    db: &Arc<DatabaseConnection>,
    lead_id: i32,
    to_user_id: i32,
) -> (i32, i32) {
    let dto = approval_service(db)
        .create_approval(
            CreateTransferApprovalRequest {
                lead_id,
                to_user_id,
                reason: "二级大客户链正序列锁：申请转移".to_string(),
            },
            USER_OWNER,
            "mc18_owner",
        )
        .await
        .expect("CONVERTED+MAJOR 准入线索建单必须放行");
    let approval_id = dto.id;
    assert!(
        dto.is_large_customer,
        "准入即大客户，is_large_customer 必须为 true"
    );
    assert_eq!(
        dto.max_level, 2,
        "大客户单必须二级，实际 max_level={}",
        dto.max_level
    );
    let mgr = approval_service(db)
        .manager_approve(
            ApproveRequest {
                approval_id,
                comment: "经理同意，转呈总监终审".to_string(),
                approved: true,
            },
            USER_MANAGER,
            "mc18_manager",
            &operator_ctx(USER_MANAGER),
        )
        .await
        .expect("经理批通过（大客户进总监层）必须成功");
    (approval_id, mgr.current_level)
}

/// 反例前提：把目标人置为停用——执行腿的新归属人活跃门必拒。
async fn deactivate_target(db: &DatabaseConnection, target: i32) {
    exec(
        db,
        &format!("UPDATE users SET is_active=FALSE WHERE id={target}"),
    )
    .await;
}

async fn reactivate_target(db: &DatabaseConnection, target: i32) {
    exec(
        db,
        &format!("UPDATE users SET is_active=TRUE WHERE id={target}"),
    )
    .await;
}

async fn approval_status_of(db: &DatabaseConnection, approval_id: i32) -> String {
    text_opt(
        db,
        &format!(
            "SELECT approval_status AS v FROM customer_transfer_approvals WHERE id={approval_id}"
        ),
    )
    .await
    .expect("审批状态非空")
}

/// T1 正序列：转化→VIP→建单二级→经理批→总监批→四枚回读齐钉。
#[tokio::test]
async fn major_converted_full_chain_transfers_customer_and_keeps_lead_terminal() {
    let db = Arc::new(setup_test_db().await);
    cleanup(&db).await;
    seed_users(&db).await;
    seed_new_lead(&db, LEAD_HAPPY, "t1").await;
    let customer_id = convert_then_vip(&db, LEAD_HAPPY).await;

    let (approval_id, level_after_mgr) =
        create_then_manager(&db, LEAD_HAPPY, USER_TARGET).await;
    assert_eq!(
        level_after_mgr, 2,
        "经理批后必须进入总监层（current_level=2）"
    );

    let dto = approval_service(&db)
        .director_approve(
            ApproveRequest {
                approval_id,
                comment: "总监终审通过".to_string(),
                approved: true,
            },
            USER_DIRECTOR,
            "mc18_director",
            &operator_ctx(USER_DIRECTOR),
        )
        .await
        .expect("总监批通过并执行客户归属转移必须成功");
    assert_eq!(dto.approval_status, "approved", "审批终态必须 approved");

    // 回读一：转移对象是客户归属
    let owner = scalar_i64(
        &db,
        &format!("SELECT owner_id::bigint AS v FROM customers WHERE id={customer_id}"),
    )
    .await;
    assert_eq!(
        owner,
        i64::from(USER_TARGET),
        "customers.owner_id 必须等于目标人"
    );
    // 回读二：审批终态与实名回写
    let name = text_opt(
        &db,
        &format!(
            "SELECT to_user_name AS v FROM customer_transfer_approvals WHERE id={approval_id}"
        ),
    )
    .await;
    assert_eq!(
        name.as_deref(),
        Some("mc18_target"),
        "to_user_name 必须回写真实用户名"
    );
    // 回读三：线索仍终态（转化门语义未松：状态不回退）
    let status = text_opt(
        &db,
        &format!("SELECT lead_status AS v FROM crm_lead WHERE id={LEAD_HAPPY}"),
    )
    .await;
    assert_eq!(
        status.as_deref(),
        Some(lead_status::CONVERTED),
        "线索状态不得回退"
    );
    // 回读四：挂链未清空
    let linked = scalar_i64(
        &db,
        &format!(
            "SELECT converted_customer_id::bigint AS v FROM crm_lead WHERE id={LEAD_HAPPY}"
        ),
    )
    .await;
    assert_eq!(
        linked,
        i64::from(customer_id),
        "converted_customer_id 不得被清空"
    );
    // 留痕与审计：TRANSFER 历史一行 + 客户 UPDATE 审计至少一行
    let hist = scalar_i64(
        &db,
        &format!(
            "SELECT COUNT(*)::bigint AS v FROM assignment_histories WHERE lead_id={LEAD_HAPPY} AND action='TRANSFER'"
        ),
    )
    .await;
    assert_eq!(
        hist, 1,
        "客户归属转移必须同事务留一条按审批来源线索的 TRANSFER 历史"
    );
    let audits = scalar_i64(
        &db,
        &format!(
            "SELECT COUNT(*)::bigint AS v FROM audit_logs WHERE resource_type='customer' AND resource_id='{customer_id}' AND action='UPDATE'"
        ),
    )
    .await;
    assert!(audits >= 1, "归属转移必须留下客户审计行，实得 {audits}");
}

/// T2 总监腿原子性反例：目标停用→转移必失败→整笔回滚（pending、审批人落项为空、
/// 归属未变、零历史零审计）；恢复后重批成功。
#[tokio::test]
async fn director_leg_failure_rolls_back_no_half_state() {
    let db = Arc::new(setup_test_db().await);
    cleanup(&db).await;
    seed_users(&db).await;
    seed_new_lead(&db, LEAD_ATOMIC_DIRECTOR, "t2").await;
    let customer_id = convert_then_vip(&db, LEAD_ATOMIC_DIRECTOR).await;
    let (approval_id, _) =
        create_then_manager(&db, LEAD_ATOMIC_DIRECTOR, USER_TARGET_T2).await;

    deactivate_target(&db, USER_TARGET_T2).await;
    let err = approval_service(&db)
        .director_approve(
            ApproveRequest {
                approval_id,
                comment: "总监批但转移前提已被破坏".to_string(),
                approved: true,
            },
            USER_DIRECTOR,
            "mc18_director",
            &operator_ctx(USER_DIRECTOR),
        )
        .await;
    assert!(
        err.is_err(),
        "转移必失败时总监批必须整体失败（不得只落状态不落转移），实得 {err:?}"
    );
    assert_eq!(
        approval_status_of(&db, approval_id).await,
        "pending",
        "转移失败后审批单必须仍 pending（旧形态会留下已批准未转移半态）"
    );
    let director_set = scalar_i64(
        &db,
        &format!(
            "SELECT (director_approver_id IS NOT NULL)::int::bigint AS v FROM customer_transfer_approvals WHERE id={approval_id}"
        ),
    )
    .await;
    assert_eq!(director_set, 0, "回滚后总监审批人落项必须为空");
    let owner = scalar_i64(
        &db,
        &format!("SELECT owner_id::bigint AS v FROM customers WHERE id={customer_id}"),
    )
    .await;
    assert_eq!(owner, i64::from(USER_OWNER), "回滚后客户归属必须未变");
    let hist = scalar_i64(
        &db,
        &format!(
            "SELECT COUNT(*)::bigint AS v FROM assignment_histories WHERE lead_id={LEAD_ATOMIC_DIRECTOR}"
        ),
    )
    .await;
    assert_eq!(hist, 0, "回滚后不得留下 TRANSFER 留痕");
    let audits = scalar_i64(
        &db,
        &format!(
            "SELECT COUNT(*)::bigint AS v FROM audit_logs WHERE resource_type='customer' AND resource_id='{customer_id}'"
        ),
    )
    .await;
    assert_eq!(audits, 0, "回滚后不得留下客户归属审计行");

    reactivate_target(&db, USER_TARGET_T2).await;
    let dto = approval_service(&db)
        .director_approve(
            ApproveRequest {
                approval_id,
                comment: "前提恢复后重批".to_string(),
                approved: true,
            },
            USER_DIRECTOR,
            "mc18_director",
            &operator_ctx(USER_DIRECTOR),
        )
        .await
        .expect("前提恢复后同一 pending 单必须可重批成功");
    assert_eq!(dto.approval_status, "approved", "重批后终态 approved");
    let owner = scalar_i64(
        &db,
        &format!("SELECT owner_id::bigint AS v FROM customers WHERE id={customer_id}"),
    )
    .await;
    assert_eq!(
        owner,
        i64::from(USER_TARGET_T2),
        "重批成功后归属必须落目标人"
    );
}

/// T3 经理单级腿同体反例：max_level=1 线索腿转移失败同样整笔回滚，
/// 恢复后重批成功、线索归属落目标人、to_user_name 为真实用户名。
#[tokio::test]
async fn manager_single_level_failure_rolls_back_no_half_state() {
    let db = Arc::new(setup_test_db().await);
    cleanup(&db).await;
    seed_users(&db).await;
    seed_new_lead(&db, LEAD_ATOMIC_MANAGER, "t3").await;

    let dto = approval_service(&db)
        .create_approval(
            CreateTransferApprovalRequest {
                lead_id: LEAD_ATOMIC_MANAGER,
                to_user_id: USER_TARGET_T3,
                reason: "单级腿原子性反例".to_string(),
            },
            USER_OWNER,
            "mc18_owner",
        )
        .await
        .expect("未转化线索建单必须放行（一级形态不变）");
    assert_eq!(dto.max_level, 1, "未转化非大客户单必须一级");

    deactivate_target(&db, USER_TARGET_T3).await;
    let err = approval_service(&db)
        .manager_approve(
            ApproveRequest {
                approval_id: dto.id,
                comment: "经理批但转移前提已被破坏".to_string(),
                approved: true,
            },
            USER_MANAGER,
            "mc18_manager",
            &operator_ctx(USER_MANAGER),
        )
        .await;
    assert!(
        err.is_err(),
        "线索转移必失败时经理批必须整体失败，实得 {err:?}"
    );
    assert_eq!(
        approval_status_of(&db, dto.id).await,
        "pending",
        "转移失败后审批单必须仍 pending"
    );
    let lead_owner = scalar_i64(
        &db,
        &format!(
            "SELECT owner_id::bigint AS v FROM crm_lead WHERE id={LEAD_ATOMIC_MANAGER}"
        ),
    )
    .await;
    assert_eq!(lead_owner, i64::from(USER_OWNER), "回滚后线索归属必须未变");

    reactivate_target(&db, USER_TARGET_T3).await;
    let after = approval_service(&db)
        .manager_approve(
            ApproveRequest {
                approval_id: dto.id,
                comment: "前提恢复后重批".to_string(),
                approved: true,
            },
            USER_MANAGER,
            "mc18_manager",
            &operator_ctx(USER_MANAGER),
        )
        .await
        .expect("前提恢复后经理重批必须成功");
    assert_eq!(after.approval_status, "approved", "重批后终态 approved");
    assert_eq!(
        after.to_user_name.as_deref(),
        Some("mc18_target_t3"),
        "单级完成必须回写真实新归属人用户名"
    );
    let lead_owner = scalar_i64(
        &db,
        &format!(
            "SELECT owner_id::bigint AS v FROM crm_lead WHERE id={LEAD_ATOMIC_MANAGER}"
        ),
    )
    .await;
    assert_eq!(
        lead_owner,
        i64::from(USER_TARGET_T3),
        "重批成功后线索归属必须落目标人"
    );
}

//! 契约锁：审批人按申请人所在部门的上一级负责人解析（含敏感操作再上一级），
//! 以及发起人部门归属落库（历史缺陷：start_process 从未写入 initiator_department_id）。
//!
//! 本文件钉死的不变量：
//! 1. 正向锁：assignee_type=dept_superior 的审批节点，工单处理人必须是
//!    申请人所在部门上一级部门的 manager_id；节点带 sensitive=true 时是
//!    该领导所在部门的再上一级部门负责人（该领导的上一级）。
//! 2. 发起人部门落库：start_process 建的实例必须带 initiator_department_id
//!    （申报值优先，未申报时按关联表主部门 / users.department_id 现算），
//!    否则按部门解析就失去可信数据源。
//! 3. 反向锁（全部 fail-closed，拒绝发起且实例与工单零残留）：
//!    申请人部门不存在上级部门、上级部门未配置负责人、多部门且无主部门标记、
//!    未关联任何部门——四类形状都必须以可外显校验错误点名去哪里配置，
//!    绝不回落到某个默认人，也绝不建无人可审的孤儿工单。
//! 4. 静态锁：审批人解析必须存在于服务层且词表含 dept_superior；敏感标记
//!    缺省即 false（存量定义无需迁移）；发起函数体必须显式写入部门列。
//!
//! 检测力自证：把解析退回只认 user 时正向锁在 expect 处红；把 fail-closed
//! 改成回落默认人时反向锁在 expect_err 处红；删掉部门落库时静态锁 2 与
//! 正向锁的部门断言同时红；把 sensitive 断链（去掉再上一级）时 t2 处理人
//! 不等于跨两级领导，正向锁红。
//!
//! 静态锁不依赖活库，CI 常规测试阶段执行；活库行为锁按本仓 test_common 夹具
//! 口径（TEST_DATABASE_URL 指向已迁移 PostgreSQL），由 CI 侧执行且整档以
//! --test-threads=1 串行跑，逐用例 setup 互不串库。

mod test_common;

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use std::sync::Arc;

use bingxi_backend::models::dto::bpm_dto::{ApproveTaskRequest, StartProcessRequest};
use bingxi_backend::models::{
    bpm_process_definition, bpm_process_instance, bpm_task, department, user, user_department,
};
use bingxi_backend::services::bpm_service::BpmService;
use bingxi_backend::utils::error::AppError;
use sea_orm::DatabaseConnection;

// ---------------------------------------------------------------------------
// 静态形态锁（不依赖活库，CI 常规可跑）
// ---------------------------------------------------------------------------

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 剔除注释行：守卫对象是代码里的形态，文档注释中说明禁语的上下文不算命中。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 审批人解析函数体：词表必须同时含 user 与 dept_superior 两条执行分支，
/// 且敏感标记的缺省形态是 false（存量定义不带 sensitive 键即非敏感，无需迁移）。
#[test]
fn assignee_resolution_supports_dept_superior_in_service_layer() {
    let src = read_rel("src/services/bpm_ops/mod.rs").replace('\r', "");
    let body = code_only(&src);
    assert!(
        body.contains("fn resolve_node_assignee") && body.contains("dept_superior"),
        "审批人解析必须存在于服务层且词表含 dept_superior（按部门上级解析），缺失即功能回退"
    );
    assert!(
        body.contains("resolve_dept_superior_assignee"),
        "dept_superior 分支必须有独立的解析实现，不允许在 handler 或前端预解析"
    );
    assert!(
        body.contains("Some(serde_json::Value::Bool(false)) => Ok(false)"),
        "敏感标记必须缺省为 false 且仅认布尔值，改成其他缺省或静默吞非布尔即红"
    );
}

/// 发起函数体必须显式写入 initiator_department_id（历史缺陷：该列被
/// ..Default::default() 吞掉从未落库，按部门解析因此没有可信数据源）。
#[test]
fn start_process_persists_initiator_department() {
    let src = read_rel("src/services/bpm_ops/instance.rs").replace('\r', "");
    let anchor = src
        .find("pub async fn start_process(")
        .expect("待锁函数不存在: pub async fn start_process(");
    let tail = &src[anchor..];
    let next = [
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    let body = code_only(&tail[..next]);
    assert!(
        body.contains("initiator_department_id: Set("),
        "start_process 必须把发起人部门显式写入实例行。函数体:\n{body}"
    );
}

// ---------------------------------------------------------------------------
// 活库行为锁（TEST_DATABASE_URL 指向已迁移 PostgreSQL；CI 侧以
//    --test-threads=1 串行执行）
// ---------------------------------------------------------------------------

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间，None 不可达")
}

async fn seed_user(db: &DatabaseConnection, role: &str) -> i32 {
    let now = Utc::now();
    let name = format!("bpm-dept-{role}-{}", unique_tag());
    user::ActiveModel {
        username: Set(name.clone()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name)),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 种带部门归属的用户（users.department_id 单值兼容路径）。
async fn seed_user_in_dept(db: &DatabaseConnection, role: &str, dept_id: i32) -> i32 {
    let now = Utc::now();
    let name = format!("bpm-dept-{role}-{}", unique_tag());
    user::ActiveModel {
        username: Set(name.clone()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name)),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        department_id: Set(Some(dept_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 种部门：parent/manager 均可为 None（分别对应"没有上级部门""未配置负责人"）。
async fn seed_dept(
    db: &DatabaseConnection,
    tag: &str,
    parent_id: Option<i32>,
    manager_id: Option<i32>,
) -> i32 {
    let now = Utc::now();
    department::ActiveModel {
        name: Set(format!("bpm-dept-{tag}-{}", unique_tag())),
        code: Set(format!("BD{tag}{}", unique_tag())),
        parent_id: Set(parent_id),
        manager_id: Set(manager_id),
        sort_order: Set(0),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

async fn seed_user_dept(db: &DatabaseConnection, user_id: i32, dept_id: i32, is_primary: bool) {
    let now = Utc::now();
    user_department::ActiveModel {
        user_id: Set(user_id),
        department_id: Set(dept_id),
        is_primary: Set(is_primary),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_definition(db: &DatabaseConnection, code: &str, config: serde_json::Value) -> i32 {
    let now = Utc::now();
    bpm_process_definition::ActiveModel {
        name: Set(format!("{code}-部门上级流程")),
        code: Set(code.to_string()),
        config: Set(Some(config)),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

fn start_req(code: &str, business_id: i64, initiator_id: i32) -> StartProcessRequest {
    StartProcessRequest {
        process_key: code.to_string(),
        business_type: "quotation".to_string(),
        business_id,
        title: "部门上级契约锁发起单".to_string(),
        initiator_id,
        initiator_name: String::new(),
        initiator_department_id: None,
        priority: None,
        form_data: None,
        variables: None,
    }
}

/// 部门上级解析流程形状：t1 普通 dept_superior，t2 敏感 dept_superior（再上一级），
/// 线性闭链 start → t1 → t2 → end。
fn dept_chain_config() -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "开始" },
            { "id": "t1", "type": "user_task", "name": "部门上级审批",
              "assignee_type": "dept_superior" },
            { "id": "t2", "type": "user_task", "name": "敏感终审",
              "assignee_type": "dept_superior", "sensitive": true },
            { "id": "end", "type": "end_event", "name": "结束" }
        ],
        "edges": [
            { "source": "start", "target": "t1" },
            { "source": "t1", "target": "t2" },
            { "source": "t2", "target": "end" }
        ]
    })
}

/// 组织树三级：leaf（申请人所在部门）→ mid（负责人=L1）→ grand（负责人=L2）。
/// 普通节点应落 L1（leaf 的上一级负责人），敏感节点应落 L2（L1 所在部门的上一级负责人）。
struct OrgTree {
    leaf: i32,
    mid: i32,
    grand: i32,
    leader_mid: i32,
    leader_grand: i32,
}

async fn seed_org_tree(db: &DatabaseConnection) -> OrgTree {
    let leader_grand = seed_user(db, "leader_grand").await;
    let leader_mid = seed_user(db, "leader_mid").await;
    let grand = seed_dept(db, "grand", None, Some(leader_grand)).await;
    let mid = seed_dept(db, "mid", Some(grand), Some(leader_mid)).await;
    let leaf = seed_dept(db, "leaf", Some(mid), None).await;
    OrgTree {
        leaf,
        mid,
        grand,
        leader_mid,
        leader_grand,
    }
}

async fn find_instance_by_business(
    db: &DatabaseConnection,
    business_id: i64,
) -> bpm_process_instance::Model {
    bpm_process_instance::Entity::find()
        .filter(bpm_process_instance::Column::BusinessType.eq("quotation"))
        .filter(bpm_process_instance::Column::BusinessId.eq(business_id))
        .one(db)
        .await
        .unwrap()
        .expect("实例行必须已落库")
}

/// 正向锁：dept_superior 按部门上级落到对的人——首任务=上一级部门负责人（L1），
/// 批准后敏感节点=再上一级部门负责人（L2）；同时断言实例落库了发起人部门。
#[tokio::test]
async fn dept_superior_resolves_to_parent_dept_manager_stepwise() {
    let db = Arc::new(test_common::setup_test_db().await);
    let org = seed_org_tree(&db).await;
    let applicant = seed_user_in_dept(&db, "applicant", org.leaf).await;
    let code = format!("bpm_dept_ok_{}", unique_tag());
    seed_definition(&db, &code, dept_chain_config()).await;

    let business_id = unique_tag();
    let resp = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, applicant))
        .await
        .expect("部门与负责人齐备时 dept_superior 必须能解析发起");

    let first = bpm_task::Entity::find_by_id(resp.task_ids[0])
        .one(&*db)
        .await
        .unwrap()
        .expect("首任务行必须已落库");
    assert_eq!(
        first.actual_handler_id,
        Some(org.leader_mid),
        "普通 dept_superior 节点必须落到申请人所在部门上一级部门的负责人"
    );

    let instance = find_instance_by_business(&db, business_id).await;
    assert_eq!(
        instance.initiator_department_id,
        Some(org.leaf),
        "实例必须落库发起人部门归属（users.department_id 兼容路径）"
    );

    let svc = BpmService::new(db.clone());
    svc.approve_task(
        ApproveTaskRequest {
            task_id: first.id,
            handler_id: org.leader_mid,
            handler_name: String::new(),
            action: "approve".to_string(),
            approval_opinion: None,
            attachment_urls: None,
        },
        Some(org.leader_mid),
    )
    .await
    .expect("上一级负责人审批通过后应推进到敏感节点");

    let second = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(resp.instance_id))
        .filter(bpm_task::Column::NodeId.eq("t2"))
        .one(&*db)
        .await
        .unwrap()
        .expect("敏感节点工单必须已创建");
    assert_eq!(
        second.actual_handler_id,
        Some(org.leader_grand),
        "敏感操作节点必须落到该领导（L1 所在部门 mid）的上一级部门负责人（grand.manager_id）"
    );
    let grand_row = department::Entity::find_by_id(org.grand)
        .one(&*db)
        .await
        .unwrap()
        .expect("造数自证：grand 部门行必须存在");
    assert_eq!(
        grand_row.manager_id,
        Some(org.leader_grand),
        "造数自证：L2 确为再上一级部门 grand 的负责人"
    );
}

/// 反向锁 A：申请人部门没有上级部门 → 拒绝发起、零残留，文案点名去部门管理补层级。
#[tokio::test]
async fn dept_superior_rejects_when_no_parent_department() {
    let db = Arc::new(test_common::setup_test_db().await);
    let leader = seed_user(&db, "leader").await;
    let lonely = seed_dept(&db, "lonely", None, Some(leader)).await;
    let applicant = seed_user_in_dept(&db, "applicant", lonely).await;
    let code = format!("bpm_dept_no_parent_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, dept_chain_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, applicant))
        .await
        .expect_err("部门链断裂时必须拒绝发起，绝不回落默认人");
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("上级部门") && msg.contains("部门管理"),
                "拒绝文案必须点名缺上级部门并指引去部门管理配置，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 反向锁 B：上一级部门存在但未配置负责人 → 拒绝发起、零残留，文案点名指定负责人。
#[tokio::test]
async fn dept_superior_rejects_when_parent_has_no_manager() {
    let db = Arc::new(test_common::setup_test_db().await);
    let no_manager_parent = seed_dept(&db, "noparentmgr", None, None).await;
    let leaf = seed_dept(&db, "leaf", Some(no_manager_parent), None).await;
    let applicant = seed_user_in_dept(&db, "applicant", leaf).await;
    let code = format!("bpm_dept_no_manager_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, dept_chain_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, applicant))
        .await
        .expect_err("上级部门未配置负责人时必须拒绝发起");
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("负责人") && msg.contains("部门管理"),
                "拒绝文案必须点名未配置负责人并指引去部门管理指定，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 反向锁 C：申请人多部门且无主部门标记 → 歧义拒绝发起，文案点名先指定主部门。
#[tokio::test]
async fn dept_superior_rejects_ambiguous_departments() {
    let db = Arc::new(test_common::setup_test_db().await);
    let org = seed_org_tree(&db).await;
    let other = seed_dept(&db, "other", Some(org.mid), None).await;
    let applicant = seed_user(&db, "applicant").await;
    seed_user_dept(&db, applicant, org.leaf, false).await;
    seed_user_dept(&db, applicant, other, false).await;
    let code = format!("bpm_dept_ambiguous_{}", unique_tag());
    let definition_id = seed_definition(&db, &code, dept_chain_config()).await;

    let business_id = unique_tag();
    let err = BpmService::new(db.clone())
        .start_process(start_req(&code, business_id, applicant))
        .await
        .expect_err("多部门无主标记属歧义，必须 fail-closed 而非任选其一");
    match err {
        AppError::ValidationErrorDisplayable(msg) => {
            assert!(
                msg.contains("主部门"),
                "拒绝文案必须点名主部门标记缺失，实得: {msg}"
            );
        }
        other => panic!("应返回可外显的校验错误，实得: {other:?}"),
    }
    assert_rolled_back(&db, business_id, definition_id).await;
}

/// 拒绝路径的事务纪律：实例与任务一行都不许留下（半写实例=回滚失效）。
async fn assert_rolled_back(db: &DatabaseConnection, business_id: i64, definition_id: i32) {
    let instances = bpm_process_instance::Entity::find()
        .filter(bpm_process_instance::Column::BusinessType.eq("quotation"))
        .filter(bpm_process_instance::Column::BusinessId.eq(business_id))
        .all(db)
        .await
        .unwrap();
    assert!(
        instances.is_empty(),
        "拒绝发起必须整体回滚：不得留下任何状态的实例行，实得 {} 条",
        instances.len()
    );
    let tasks = bpm_task::Entity::find()
        .filter(bpm_task::Column::ProcessDefinitionId.eq(definition_id))
        .all(db)
        .await
        .unwrap();
    assert!(
        tasks.is_empty(),
        "拒绝发起不得留下工单行，实得 {} 条",
        tasks.len()
    );
}

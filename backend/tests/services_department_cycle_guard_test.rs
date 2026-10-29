//! 功能：部门父级成环守卫的契约锁，钉住两件事——写侧 update 把 parent_id 指向自身或本部门
//! 后代必须在落库前拒绝，且在库内已存在脏环时判环上溯也必须有限时间返回；读侧取部门可见域
//! 的展开对存量脏环必须每个节点至多访问一次、有限时间返回，而不是无限循环挂死认证链。
//! 调用方：集成测试层（真已迁移 PostgreSQL，TEST_DATABASE_URL；本文件不调 Migrator，
//! 不依赖迁移台账内容，用例需要真库故与既有 contract 族同在 CI 侧执行）。
//! 入参：test_common 的 setup_test_db 连接并清空业务表后，自建一段自有主键的部门行与
//! 两个用户行。departments 属迁移种子参照表、不参与清空，故播种前先按自有主键删除再
//! 重新写入，用毕再删除，保证用例可重复执行且异常形态不在同库存留。
//! 传给谁：DepartmentService 的 update 与 delete、DataPermissionService 的
//! get_user_dept_scope_ids（认证链 get_user_dept_scope_cached 内部走的就是它）。
//! 存什么：判环拒绝的错误变体与文案、拒绝后 parent_id 的原值、放行后 parent_id 的新值、
//! 可见域返回的部门主键集合与返回时限。
//! 存哪里：部门行存 departments 表，用户行存 users 表；断言只读出参与回读，不写新表。
//!
//! 判据（逐条，判红才算锁有效）：
//! 一、update 把 parent_id 写成自身，必须返回可外显的校验错误族，且回读 parent_id 保持原值。
//! 二、update 把 parent_id 写成自己的直接子部门（写成后构成二元环），必须同样拒绝且原值不动。
//! 三、update 把 parent_id 写成自己的孙部门（写成后跨级成环），必须同样拒绝。
//! 四、update 把 parent_id 写成无亲缘关系的其它部门（合法移动），必须放行且新值落库，
//! 防止判环过拒把正常调整层级也拦死。
//! 五、先用实体层直写绕过服务层造出存量互环，再 update 一个无关部门的 parent_id 指向环上
//! 节点，判环上溯必须在有界时间内返回并放行（环不含该部门，本次写入不构成新环），
//! 钉住上溯自带防环终止，库内存量脏环不能把写侧挂死。
//! 六、带子部门的部门删除仍必须被既有行为拒绝，判环改动不得回归删除防线。
//! 七、读侧：把部门岗用户的 users.department_id 指向存量互环节点，取可见域必须在有界
//! 时间内返回，返回集合精确等于环成员加环下叶，即每个节点至多访问一次。修复前该展开是
//! 无 await 让出的忙循环，用例以 multi_thread 运行时让定时器在空闲工作线程到点，
//! 超时即判红；若忙循环先吃光内存则为进程级红，两种形态都如实红，不存在放宽口径。
//!
//! 造数说明：环形态不需要绕过外键，父行在写入时刻都已存在，自引用外键逐步满足；
//! 悬挂外键形态本文件不涉及，因此不需要 SET LOCAL session_replication_role。

mod test_common;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tokio::time::timeout;

use bingxi_backend::handlers::department_handler::UpdateDepartmentRequest;
use bingxi_backend::models::{department, user};
use bingxi_backend::services::data_permission_service::DataPermissionService;
use bingxi_backend::services::department_service::DepartmentService;
use bingxi_backend::utils::error::AppError;

// 自有主键段 9760 至 9771，避开迁移种子部门（id 1 至 5）、未合并树契约夹具占用的
// 9730 至 9752，以及 contract_wave13 占用的部门 947x 与用户 941x。
const DEPT_A: i32 = 9760; // 顶级，判环测试的被移动部门
const DEPT_B: i32 = 9761; // A 的直接子
const DEPT_C: i32 = 9762; // B 的直接子，即 A 的孙
const DEPT_D: i32 = 9763; // 与 A 无亲缘关系的顶级部门，合法移动目标
const DEPT_X: i32 = 9764; // 存量互环甲
const DEPT_Y: i32 = 9765; // 存量互环乙
const DEPT_Z: i32 = 9766; // 无关顶级部门，判环上溯在有脏环时的放行对象
const DEPT_R1: i32 = 9767; // 读侧互环甲，部门岗用户的挂靠部门
const DEPT_R2: i32 = 9768; // 读侧互环乙
const DEPT_LEAF: i32 = 9769; // 挂在 R1 下的正常子节点
const DEPT_DEL_PARENT: i32 = 9770; // 带子删除不回归的父
const DEPT_DEL_KID: i32 = 9771; // DEPT_DEL_PARENT 的子

// 测试用户：USER_OP 为 update/delete 的操作人（无部门），USER_SCOPE 是部门岗（挂 R1）。
const USER_OP: i32 = 9760;
const USER_SCOPE: i32 = 9761;

/// 挂死证伪用的有界时限：修复后展开是毫秒级，给到 10 秒仍超时即判红
const HANG_PROOF_TIMEOUT: Duration = Duration::from_secs(10);

fn fixture_dept_ids() -> Vec<i32> {
    vec![
        DEPT_A,
        DEPT_B,
        DEPT_C,
        DEPT_D,
        DEPT_X,
        DEPT_Y,
        DEPT_Z,
        DEPT_R1,
        DEPT_R2,
        DEPT_LEAF,
        DEPT_DEL_PARENT,
        DEPT_DEL_KID,
    ]
}

async fn insert_dept(db: &sea_orm::DatabaseConnection, id: i32, name: &str, parent: Option<i32>) {
    let now = Utc::now();
    department::ActiveModel {
        id: Set(id),
        name: Set(name.to_string()),
        code: Set(format!("CYCG{id}")),
        parent_id: Set(parent),
        description: Set(Some(format!("成环守卫契约锁{id}"))),
        sort_order: Set(id - 9700),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("测试夹具：播种部门 {id} 失败: {e}"));
}

/// 以实体层直写 parent_id，绕过服务层判环门，构造库内已存在的脏环
async fn force_parent(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    parent: Option<i32>,
) -> department::Model {
    let am = department::ActiveModel {
        id: Set(id),
        parent_id: Set(parent),
        ..Default::default()
    };
    am.update(db)
        .await
        .unwrap_or_else(|e| panic!("测试夹具：直写部门 {id} 的 parent_id 失败: {e}"))
}

async fn seed_user(db: &sea_orm::DatabaseConnection, uid: i32, dept: Option<i32>) {
    let now = Utc::now();
    user::ActiveModel {
        id: Set(uid),
        username: Set(format!("dept_cycle_guard_{uid}")),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        department_id: Set(dept),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("测试夹具：播种用户 {uid} 失败: {e}"));
}

/// 先删自有主键段再重建，departments 参与清空的豁免（参照表）由该先删后插保证可重复
async fn seed(db: &sea_orm::DatabaseConnection) {
    cleanup(db).await;

    insert_dept(db, DEPT_A, "成环守卫甲", None).await;
    insert_dept(db, DEPT_B, "成环守卫乙", Some(DEPT_A)).await;
    insert_dept(db, DEPT_C, "成环守卫丙", Some(DEPT_B)).await;
    insert_dept(db, DEPT_D, "成环守卫丁", None).await;
    insert_dept(db, DEPT_X, "成环守卫环甲", None).await;
    insert_dept(db, DEPT_Y, "成环守卫环乙", None).await;
    insert_dept(db, DEPT_Z, "成环守卫游离", None).await;
    insert_dept(db, DEPT_R1, "成环守卫读环甲", None).await;
    insert_dept(db, DEPT_R2, "成环守卫读环乙", None).await;
    insert_dept(db, DEPT_LEAF, "成环守卫读环叶", Some(DEPT_R1)).await;
    insert_dept(db, DEPT_DEL_PARENT, "成环守卫删除父", None).await;
    insert_dept(db, DEPT_DEL_KID, "成环守卫删除子", Some(DEPT_DEL_PARENT)).await;

    // 存量脏环：X 与 Y 互指、R1 与 R2 互指，父行写入时刻都已存在，外键逐步满足
    force_parent(db, DEPT_X, Some(DEPT_Y)).await;
    force_parent(db, DEPT_Y, Some(DEPT_X)).await;
    force_parent(db, DEPT_R1, Some(DEPT_R2)).await;
    force_parent(db, DEPT_R2, Some(DEPT_R1)).await;

    seed_user(db, USER_OP, None).await;
    seed_user(db, USER_SCOPE, Some(DEPT_R1)).await;
}

async fn cleanup(db: &sea_orm::DatabaseConnection) {
    // 先删用户再删部门：users.department_id 有外键指向部门行
    user::Entity::delete_many()
        .filter(user::Column::Id.is_in(vec![USER_OP, USER_SCOPE]))
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("测试夹具：清理测试用户失败: {e}"));
    department::Entity::delete_many()
        .filter(department::Column::Id.is_in(fixture_dept_ids()))
        .exec(db)
        .await
        .unwrap_or_else(|e| panic!("测试夹具：清理自有部门主键段失败: {e}"));
}

async fn reload_parent(db: &sea_orm::DatabaseConnection, id: i32) -> Option<i32> {
    department::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("测试夹具：回读部门 {id} 失败: {e}"))
        .unwrap_or_else(|| panic!("测试夹具：部门 {id} 应存在"))
        .parent_id
}

/// 只改 parent_id 的最小更新请求：其余字段全部键缺席，保持原值
fn set_parent_req(new_parent: Option<i32>) -> UpdateDepartmentRequest {
    UpdateDepartmentRequest {
        name: None,
        code: None,
        description: None,
        parent_id: Some(new_parent),
        manager_id: None,
        sort_order: None,
        is_active: None,
    }
}

/// 断言拒绝形状必须是可外显校验族且文案只说该做什么，不含内部标识
fn assert_displayable_parent_rejection(err: &AppError) {
    assert!(
        matches!(err, AppError::ValidationErrorDisplayable(m)
            if m.contains("上级部门不能选择本部门或本部门的下级部门")),
        "判环拒绝必须是 ValidationErrorDisplayable 族且文案合规: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 判据一：自引用（parent_id 等于自身）被拒
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_parent_to_self_is_rejected() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let svc = DepartmentService::new(Arc::new(db.clone()));

    let err = svc
        .update(DEPT_A, USER_OP, set_parent_req(Some(DEPT_A)))
        .await
        .expect_err("把上级设成自身必须被拒");
    assert_displayable_parent_rejection(&err);
    assert_eq!(
        reload_parent(&db, DEPT_A).await,
        None,
        "拒绝后 parent_id 必须保持原值，不得先写后滚或写脏"
    );

    cleanup(&db).await;
}

// ---------------------------------------------------------------------------
// 判据二：写成二元环（parent_id 指向直接子部门）被拒
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_parent_to_direct_child_is_rejected() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let svc = DepartmentService::new(Arc::new(db.clone()));

    // A 的父指向其子 B，写成后 A 与 B 互指成环
    let err = svc
        .update(DEPT_A, USER_OP, set_parent_req(Some(DEPT_B)))
        .await
        .expect_err("把上级设成自己的直接子必须被拒");
    assert_displayable_parent_rejection(&err);
    assert_eq!(
        reload_parent(&db, DEPT_A).await,
        None,
        "拒绝后 A 的 parent_id 必须保持原值"
    );
    assert_eq!(
        reload_parent(&db, DEPT_B).await,
        Some(DEPT_A),
        "拒绝不得牵连子部门 B 的既有父级"
    );

    cleanup(&db).await;
}

// ---------------------------------------------------------------------------
// 判据三：跨级后代环（parent_id 指向孙部门）被拒
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_parent_to_grandchild_is_rejected() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let svc = DepartmentService::new(Arc::new(db.clone()));

    // 链 A 下 B 下 C，A 的父指向孙 C，写成后 A 与 B 与 C 跨级成环
    let err = svc
        .update(DEPT_A, USER_OP, set_parent_req(Some(DEPT_C)))
        .await
        .expect_err("把上级设成自己的孙部门必须被拒");
    assert_displayable_parent_rejection(&err);
    assert_eq!(
        reload_parent(&db, DEPT_A).await,
        None,
        "拒绝后 A 的 parent_id 必须保持原值"
    );

    cleanup(&db).await;
}

// ---------------------------------------------------------------------------
// 判据四：合法移动放行（防判环过拒）
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_parent_to_non_descendant_succeeds() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let svc = DepartmentService::new(Arc::new(db.clone()));

    // D 与 A 同为顶级、互不为祖先，把 A 挂到 D 下是合法层级调整
    let updated = svc
        .update(DEPT_A, USER_OP, set_parent_req(Some(DEPT_D)))
        .await
        .expect("合法移动到新上级不得被拒");
    assert_eq!(
        updated.parent_id,
        Some(DEPT_D),
        "返回模型的 parent_id 应为新上级"
    );
    assert_eq!(
        reload_parent(&db, DEPT_A).await,
        Some(DEPT_D),
        "合法移动必须真实落库"
    );

    cleanup(&db).await;
}

// ---------------------------------------------------------------------------
// 判据五：库内已有脏环时判环上溯有界返回并放行
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_under_existing_dirty_cycle_returns_bounded() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let svc = DepartmentService::new(Arc::new(db.clone()));

    // X 与 Y 已是存量互环（seed 内以实体层直写造成），Z 与它们无亲缘关系。
    // 把 Z 的上级指向 X，判环上溯会走进脏环，必须靠已访问集合终止而不是挂死。
    let outcome = timeout(
        HANG_PROOF_TIMEOUT,
        svc.update(DEPT_Z, USER_OP, set_parent_req(Some(DEPT_X))),
    )
    .await
    .expect("判环上溯在存量脏环下挂死（有界时限内未返回）");
    let updated = outcome.expect("环不含本部门时本次写入不构成新环，应放行");
    assert_eq!(updated.parent_id, Some(DEPT_X), "放行后应返回新值");
    assert_eq!(
        reload_parent(&db, DEPT_Z).await,
        Some(DEPT_X),
        "放行必须真实落库"
    );

    cleanup(&db).await;
}

// ---------------------------------------------------------------------------
// 判据六：带子部门删除被拦的既有行为不回归
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_department_with_children_still_rejected() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let svc = DepartmentService::new(Arc::new(db.clone()));

    let err = svc
        .delete(DEPT_DEL_PARENT, USER_OP)
        .await
        .expect_err("存在子部门时删除必须被拒（既有防线）");
    assert!(
        // 业务拒绝族含可外显与不可外显两个变体（出参码同为 BUSINESS_ERROR、状态同为 400），
        // 服务侧改走哪个变体都不该让这条防线失效，故两形态一并判；文案仍须点名"子部门"。
        matches!(
            &err,
            AppError::BusinessError(m) | AppError::BusinessErrorDisplayable(m)
                if m.contains("子部门")
        ),
        "删除防线应维持既有业务错误: {err:?}"
    );
    assert_eq!(
        reload_parent(&db, DEPT_DEL_PARENT).await,
        None,
        "拒绝删除后父部门行必须仍在且层级未被改动"
    );

    cleanup(&db).await;
}

// ---------------------------------------------------------------------------
// 判据七：读侧可见域展开在存量脏环下有界返回且成员不重复
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dept_scope_with_existing_cycle_returns_bounded_distinct_set() {
    let db = test_common::setup_test_db().await;
    seed(&db).await;
    let dps = DataPermissionService::new(Arc::new(db.clone()));

    // USER_SCOPE 的 users.department_id 指向 R1，R1 与 R2 互环，LEAF 挂在 R1 下。
    // 修复前该展开是无 await 让出的忙循环，multi_thread 下定时器在空闲工作线程到点，
    // 超时即判红。parent_id 是单值函数，图中除环外每节点只有一条入路，
    // 集合精确等于可达成员且有限返回，即证明每个节点至多访问一次。
    let ids = timeout(HANG_PROOF_TIMEOUT, dps.get_user_dept_scope_ids(USER_SCOPE))
        .await
        .expect("取部门可见域在存量脏环下挂死（有界时限内未返回）")
        .expect("可见域解析失败");
    assert_eq!(
        ids,
        vec![DEPT_R1, DEPT_R2, DEPT_LEAF],
        "可见域必须精确等于环成员加环下叶（升序，成员不重复也不丢失）"
    );

    cleanup(&db).await;
}

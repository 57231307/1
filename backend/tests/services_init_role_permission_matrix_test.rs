//! 角色权限矩阵落库的**行为级**锁（CI run #4675 E6 根修，真库通道）
//!
//! 缺陷事实（不是推测，逐条可核）：
//! - `ci4675/rs/rs14/backend.log:8429-8432`：`resource=production-orders, action=read,
//!   has_perm=false` → 403。授权定义里明明有 `("production-orders","*")`
//!   （`init_service_ops/permission.rs` 的 production_manager 分组），中间件也支持 `*` 通配
//!   （`middleware/permission.rs::matches_permission`），运行期却判无权限 ⇒ 授权行根本没落库。
//! - `ci4675/rs/rs14/playwright-output.txt:257`：`[globalSetup] 后端现有角色 3 个`。
//!   迁移 `m0001_initial_schema.rs:609-612` 只种 admin/manager/operator，而
//!   `init_service_ops/role.rs::create_default_roles` 旧写法是"admin 已存在即整体早退"，
//!   于是矩阵里 34 个角色码在 roles 表解析不到、其业务授权一条都写不进去
//!   （`if let Some(r) {..}` 无 else 分支，静默跳过）——这就是 403 与
//!   角色权限矩阵大面积只派生出 `/dashboard` 的共同上游根因。
//! - 次生雷：`create_default_role_permissions` 旧幂等守卫是"role_permissions 全表 count>0 就
//!   整体跳过"，存量库重放 init 时同样会吞掉首建。
//!
//! 本文件锁四件事，全部走真库回读（不是只看函数返回 `Ok`）：
//! ① 初始化链跑完后，被判红角色（production_manager / qc_manager）在 `role_permissions`
//!    里**真实存在** production-orders 授权行，且应用外壳码 dashboard 也在场；
//! ② 幂等：连跑两次授权建立子步，行数与内容逐条相等、不新增重复行、也不报错；
//! ③ fail-visible：库里没有业务角色（解析不到角色码）时必须判红，且**一条都不能先落库**
//!    （闸门在写入之前拦住，不许留下"部分角色已授权"的更难查的半截状态）；
//! ④ fail-visible：未登记在 `PERMISSION_RESOURCES` 的资源码必须被闸门点名（授予也是死码授权），
//!    已登记的资源码不得误报。
//!
//! 夹具口径：`test_common::setup_test_db()` 连的是已跑完 `backend/migration` 的 PostgreSQL；
//! `roles` / `role_permissions` 属迁移种子参照表（夹具**不**清空），所以本文件自己把起点收敛回
//! "只有 admin/manager/operator" 的迁移后形态，并在用例结束时留给下一个用例同样的干净起点。
//! 无 `#[ignore]`、不硬编码 sqlite、不建第二套夹具。错误只判类型与缺口点名数据（角色码/资源码），
//! 不锁文案措辞。

mod test_common;

use bingxi_backend::models::{role, role_permission};
use bingxi_backend::services::init_service::{InitError, InitService, PERMISSION_RESOURCES};
use bingxi_backend::services::init_service_ops::permission::{
    RoleResourceGroup, UnregisteredResource, find_unregistered_matrix_resources,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, Statement,
};
use std::sync::Arc;

/// 迁移 `m0001` 自带的三个种子角色：本文件只清自己补建出来的角色，绝不碰这三行
/// （其它用例按 id=1/2/3 依赖它们，见 `contract_wave7_crm_read_vs_write_gate_test.rs:113` 口径）
const MIGRATION_SEED_ROLE_CODES: &[&str] = &["admin", "manager", "operator"];

/// 初始化链要建的管理员账号（用例内自种子、结束时自清理）
const ADMIN_USERNAME: &str = "it_rbac_matrix_admin";
/// 必须过 `utils::password_validator` 的强度策略（大小写 + 数字 + 特殊字符 + ≥8 位）
const ADMIN_PASSWORD: &str = "Init!Matrix2026";

/// 本文件的用例都往同一只真库写 `roles`/`role_permissions`/`users`，
/// 并行执行会互相撞行数断言与唯一键 ⇒ 进程内串行（`roles`/`role_permissions`
/// 是夹具不清空的参照表，这一点是硬约束，不是保守选择）。
static DB_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子/清理 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 把起点收敛回"迁移跑完、init 未跑"的 CI 真实形态：
/// 只留 admin/manager/operator，业务角色与其授权行、夹具账号一律清掉。
/// 顺序 = 授权 → 账号 → 角色（`role_permissions.role_id`、`users.role_id` 都是指向 roles 的外键）。
async fn reset_to_migrated_state(db: &DatabaseConnection) {
    let seeds = MIGRATION_SEED_ROLE_CODES
        .iter()
        .map(|c| format!("'{}'", c))
        .collect::<Vec<String>>()
        .join(",");
    exec(
        db,
        &format!(
            "DELETE FROM role_permissions WHERE role_id IN \
             (SELECT id FROM roles WHERE code NOT IN ({seeds}))"
        ),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM users WHERE username = '{}'", ADMIN_USERNAME),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM roles WHERE code NOT IN ({seeds})"),
    )
    .await;
}

/// 按角色码取 role id；取不到即用例前提不成立（不做静默跳过）
async fn role_id_of(db: &DatabaseConnection, code: &str) -> i32 {
    role::Entity::find()
        .filter(role::Column::Code.eq(code))
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("查询角色 {code} 失败: {e}"))
        .unwrap_or_else(|| panic!("角色 {code} 不存在：初始化链没建出它，用例前提已被破坏"))
        .id
}

/// 回读某角色在某资源上的全部授权（resource_id IS NULL 的角色级行）
async fn role_level_actions(db: &DatabaseConnection, rid: i32, resource: &str) -> Vec<String> {
    role_permission::Entity::find()
        .filter(role_permission::Column::RoleId.eq(rid))
        .filter(role_permission::Column::ResourceType.eq(resource))
        .filter(role_permission::Column::ResourceId.is_null())
        .all(db)
        .await
        .unwrap_or_else(|e| panic!("回读 role_permissions 失败: {e}"))
        .into_iter()
        .filter(|r| r.allowed)
        .map(|r| r.action)
        .collect()
}

/// 全表角色级授权快照（排序后逐条可比），用于幂等断言
async fn role_level_snapshot(db: &DatabaseConnection) -> Vec<(i32, String, String, bool)> {
    let mut rows: Vec<(i32, String, String, bool)> = role_permission::Entity::find()
        .filter(role_permission::Column::ResourceId.is_null())
        .all(db)
        .await
        .unwrap_or_else(|e| panic!("快照 role_permissions 失败: {e}"))
        .into_iter()
        .map(|r| (r.role_id, r.resource_type, r.action, r.allowed))
        .collect();
    rows.sort();
    rows
}

/// 同一 (role_id, resource_type, action) 出现两次即为重复行
fn duplicates(rows: &[(i32, String, String, bool)]) -> Vec<(i32, String, String)> {
    let mut seen: Vec<((i32, String, String), usize)> = Vec::new();
    let mut dup: Vec<(i32, String, String)> = Vec::new();
    for (role_id, resource, action, _allowed) in rows {
        let key = (*role_id, resource.clone(), action.clone());
        match seen.iter_mut().find(|(k, _)| k == &key) {
            Some(entry) => {
                entry.1 += 1;
                if entry.1 == 2 && !dup.contains(&key) {
                    dup.push(key);
                }
            }
            None => seen.push((key, 1)),
        }
    }
    dup
}

/// ① 初始化链跑完后，业务授权必须**真实落库**（E6 的直接反证）
#[tokio::test]
async fn test_initialize_lands_production_order_grants() {
    let _guard = DB_SERIAL.lock().await;
    let db = test_common::setup_test_db().await;
    reset_to_migrated_state(&db).await;

    let svc = InitService::new(Arc::new(db.clone()));
    svc.initialize(ADMIN_USERNAME, ADMIN_PASSWORD)
        .await
        .unwrap_or_else(|e| panic!("初始化链必须成功，实得 {e:?}"));

    // E6 主角：生产岗读生产工单（矩阵授予的是 ("production-orders","*")）
    let pm = role_id_of(&db, "production_manager").await;
    let pm_actions = role_level_actions(&db, pm, "production-orders").await;
    assert!(
        pm_actions.iter().any(|a| a == "*" || a == "read"),
        "production_manager 必须有 production-orders 的角色级授权（E6 的 403 就是这么来的），实得 {pm_actions:?}"
    );

    // 第二个被判红角色（CI 矩阵分区里同样只派生出 /dashboard 的质量岗）：
    // 矩阵给的是 ("production-orders","read") + ("quality-inspections","*")
    let qc = role_id_of(&db, "qc_manager").await;
    let qc_prod = role_level_actions(&db, qc, "production-orders").await;
    assert!(
        qc_prod.iter().any(|a| a == "*" || a == "read"),
        "qc_manager 必须落到 production-orders 读权限，实得 {qc_prod:?}"
    );
    let qc_insp = role_level_actions(&db, qc, "quality-inspections").await;
    assert!(
        qc_insp.iter().any(|a| a == "*" || a == "read"),
        "qc_manager 必须落到 quality-inspections 权限，实得 {qc_insp:?}"
    );

    // 应用外壳码：缺 dashboard:read 的角色登录后会被守卫送到 /403（矩阵口径）
    for rid in [pm, qc] {
        let dash = role_level_actions(&db, rid, "dashboard").await;
        assert!(
            dash.iter().any(|a| a == "*" || a == "read"),
            "角色 {rid} 必须有 dashboard 读权限（外壳码），实得 {dash:?}"
        );
    }

    reset_to_migrated_state(&db).await;
}

/// ② 幂等：连跑两次（再跑第三次）授权建立子步，不报错、不新增行、不产生重复行
#[tokio::test]
async fn test_role_permission_matrix_is_idempotent_on_replay() {
    let _guard = DB_SERIAL.lock().await;
    let db = test_common::setup_test_db().await;
    reset_to_migrated_state(&db).await;

    let arc = Arc::new(db.clone());
    InitService::new(arc.clone())
        .initialize(ADMIN_USERNAME, ADMIN_PASSWORD)
        .await
        .unwrap_or_else(|e| panic!("首跑初始化链必须成功，实得 {e:?}"));

    let first = role_level_snapshot(&db).await;
    assert!(
        !first.is_empty(),
        "首跑后 role_permissions 必须有角色级授权行，否则本用例什么都没验证"
    );
    assert!(
        duplicates(&first).is_empty(),
        "首跑就不得产生重复授权行，重复键 {:?}",
        duplicates(&first)
    );

    // 重放同一子步（运维重放/二次初始化走的就是这一步）：必须 0 新增、0 报错
    InitService::new(arc.clone())
        .create_default_role_permissions()
        .await
        .unwrap_or_else(|e| {
            panic!("幂等重放不得报错（旧 count>0 守卫废除后按条对账），实得 {e:?}")
        });
    let second = role_level_snapshot(&db).await;
    assert_eq!(
        first, second,
        "重放授权建立子步必须逐条等价：行数/内容/allowed 都不许变"
    );

    InitService::new(arc)
        .create_default_role_permissions()
        .await
        .unwrap_or_else(|e| panic!("第三次重放同样不得报错，实得 {e:?}"));
    assert_eq!(
        second,
        role_level_snapshot(&db).await,
        "多次重放仍必须逐条等价"
    );

    reset_to_migrated_state(&db).await;
}

/// ③ fail-visible：库里解析不到角色码时必须判红，且一条都不能先落库
#[tokio::test]
async fn test_unresolvable_role_codes_fail_visible_without_partial_write() {
    let _guard = DB_SERIAL.lock().await;
    let db = test_common::setup_test_db().await;
    // 起点 = 迁移后、业务角色不存在（正是 CI #4675 的实际形态：roles 只有 3 行）
    reset_to_migrated_state(&db).await;

    let before_total = role_permission::Entity::find()
        .count(&db)
        .await
        .unwrap_or_else(|e| panic!("写前计数失败: {e}"));

    let result = InitService::new(Arc::new(db.clone()))
        .create_default_role_permissions()
        .await;

    let err = match result {
        Ok(()) => panic!(
            "角色表缺业务角色时，授权建立必须判红；\
             静默返回成功正是 E6 里那条看起来跑过了、其实什么都没写的路径"
        ),
        Err(e) => e,
    };

    // 只判错误类别 + 缺口点名数据（角色码），不锁文案措辞
    let msg = match &err {
        InitError::DatabaseError(m) => m.clone(),
        other => panic!("期望数据库类错误（授权链断裂属落库问题），实得 {other:?}"),
    };
    for code in ["production_manager", "qc_manager", "dyeing_master"] {
        assert!(
            msg.contains(code),
            "错误体必须点名解析不到的角色码 {code}，实得: {msg}"
        );
    }

    // 闸门必须拦在写入之前：不允许出现"manager/operator 已写、业务角色没写"的半截状态
    let after_total = role_permission::Entity::find()
        .count(&db)
        .await
        .unwrap_or_else(|e| panic!("写后计数失败: {e}"));
    assert_eq!(
        before_total, after_total,
        "判红时不得有任何授权行被写入（应写 vs 实写的对账起点必须保持干净）"
    );

    reset_to_migrated_state(&db).await;
}

/// 未登记资源码的负提交集（真库里建不出"未登记资源"的授权行，故直接驱动闸门纯函数）
const UNREGISTERED_GROUP: &[RoleResourceGroup] = &[(
    "production_manager",
    &[("ghost-resource-not-registered", "read")],
)];

/// 已登记资源码的对照正例：同一道闸门不得误报
const REGISTERED_GROUP: &[RoleResourceGroup] =
    &[("production_manager", &[("production-orders", "*")])];

/// ④ fail-visible：资源码不在 `PERMISSION_RESOURCES` 注册表 ⇒ 必须被点名（授予也是死码授权）
#[test]
fn test_unregistered_matrix_resource_is_reported() {
    // 前提自检：注册表本身在场，且 ghost 码确实没登记、production-orders 确实登记了
    assert!(
        !PERMISSION_RESOURCES.contains(&"ghost-resource-not-registered"),
        "用例前提：ghost 码必须不在注册表里，否则本负例什么都没验证"
    );
    assert!(
        PERMISSION_RESOURCES.contains(&"production-orders"),
        "用例前提：production-orders 必须已登记（E6 就是授的它）"
    );

    let unknown: Vec<UnregisteredResource> =
        find_unregistered_matrix_resources(&[UNREGISTERED_GROUP]);
    assert_eq!(unknown.len(), 1, "未登记资源码必须被点名，实得 {unknown:?}");
    assert_eq!(unknown[0].0, "ghost-resource-not-registered");
    assert!(
        unknown[0].1.contains(&"production_manager".to_string()),
        "点名结果必须带上引用它的角色码，实得 {:?}",
        unknown[0].1
    );

    // 正例：已登记资源码不误报（否则闸门①会把每次正常 init 都判红）
    assert!(
        find_unregistered_matrix_resources(&[REGISTERED_GROUP]).is_empty(),
        "已登记资源码不得被误报为缺口"
    );

    // 全量真实矩阵由用例 ①② 走同一条链路（闸门①在任何写入之前）间接锁死：
    // 那两条能跑通即证明矩阵 123 个资源码全部已登记，未登记一条就会整条链判红。
}

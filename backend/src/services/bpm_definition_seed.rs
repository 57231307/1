//! 出厂内置 BPM 审批流程定义种子落库
//!
//! 报价单/销售订单/采购订单/生产订单的提交审批链路都以 `start_process(process_key=...)` 查找
//! ACTIVE 流程定义为前置门（报价/销售/采购/生产四个提交入口各自以固定编码发起，且都是
//! fail-closed：查不到定义即拒绝提交并回滚业务状态）。系统出厂时该表为空，四个提交审批入口
//! 必然失败——本模块把「出厂即可用」的流程定义以幂等方式补种到位。
//!
//! 幂等口径与内置打印模板种子一致：`INSERT ... ON CONFLICT (code) DO NOTHING`，仅补缺失编码，
//! **绝不覆盖**既有行（用户在流程定义页对节点/审批人的任何调整永不受启动重跑影响）。
//!
//! 节点链形态与业务侧/测试侧既有可用形态同源：`start_event → user_task → end_event`
//! 三节点两条边，user_task 以 `assignee_type=user` 绑定一个具体审批人——审批引擎当前仅支持
//! 指定具体用户（`resolve_node_assignee` 对其余类型显式拒绝），单节点人工审批即出厂默认链路，
//! 上线后可在流程定义页调整。
//!
//! 审批人锚点（不硬编码任何用户）：取「admin 角色下 ID 最小的启用用户」——admin 角色编码取
//! 初始化链路定义的真实角色码（utils::admin_checker::ADMIN_ROLE_CODE），与权限域同源。
//! 角色或启用用户不存在时（系统尚未完成初始化）记 warn 并跳过：这是部署时序前提而非缺陷静默，
//! 完成初始化后（初始化链路内会当场补种一次）或服务重启后启动链会自动补齐，全程显式日志。

use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use tracing::{info, warn};

use crate::models::{bpm_process_definition, role, user};
use crate::utils::admin_checker::ADMIN_ROLE_CODE;
use crate::utils::error::AppError;

/// 流程定义「激活」状态取值：与发起审批时的定义查找过滤值逐字符相同（大写 ACTIVE），
/// 单一词表形态沿用建表默认值 DRAFT / 查找值 ACTIVE 的既有约定，不另立第二套状态常量。
const DEFINITION_STATUS_ACTIVE: &str = "ACTIVE";

/// 出厂内置审批流程定义清单（编码必须与提交审批链路发起侧的 process_key 逐字符相同）。
struct BuiltinApprovalDefinition {
    code: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
}

/// 四个提交审批入口对应的出厂流程定义：编码来源为业务发起侧
/// （报价单提交、销售订单提交、采购订单提交、生产订单提交各自的 start_process 调用），
/// 名称/分类与既有测试种子与业务标题口径一致。
const BUILTIN_APPROVAL_DEFINITIONS: &[BuiltinApprovalDefinition] = &[
    BuiltinApprovalDefinition {
        code: "quotation_approval",
        name: "报价单审批流程",
        category: "sales",
        description: "出厂内置：报价单提交后的单节点人工审批链路，审批人默认绑定 admin 角色首个启用用户，可在流程定义页调整",
    },
    BuiltinApprovalDefinition {
        code: "sales_order_approval",
        name: "销售订单审批流程",
        category: "sales",
        description: "出厂内置：销售订单提交后的单节点人工审批链路，审批人默认绑定 admin 角色首个启用用户，可在流程定义页调整",
    },
    BuiltinApprovalDefinition {
        code: "purchase_order_approval",
        name: "采购订单审批流程",
        category: "purchase",
        description: "出厂内置：采购订单提交后的单节点人工审批链路，审批人默认绑定 admin 角色首个启用用户，可在流程定义页调整",
    },
    BuiltinApprovalDefinition {
        code: "production_order_approval",
        name: "生产订单审批流程",
        category: "production",
        description: "出厂内置：生产订单提交后的单节点人工审批链路，审批人默认绑定 admin 角色首个启用用户，可在流程定义页调整",
    },
];

/// 构建单节点人工审批的流程配置（nodes/edges 唯一生成出口，保证种子形态一致）。
///
/// 入参 `assignee_user_id` 为审批任务绑定的具体用户 ID（users.id 正整数）；
/// 传给审批引擎的节点解析函数（resolve_first_task_node / resolve_node_assignee）消费。
fn build_single_node_approval_config(assignee_user_id: i32) -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "start", "name": "提交审批", "type": "start_event" },
            {
                "id": "approve_task",
                "name": "人工审批",
                "type": "user_task",
                "assignee_type": "user",
                "assignee_value": assignee_user_id,
            },
            { "id": "end", "name": "完成", "type": "end_event" },
        ],
        "edges": [
            { "source": "start", "target": "approve_task" },
            { "source": "approve_task", "target": "end" },
        ],
    })
}

/// 解析出厂审批人锚点：admin 角色（角色码与权限域同源）下 ID 最小的启用用户。
///
/// 返回 `Ok(None)` 表示角色或用户尚不存在（系统未完成初始化），调用方必须显式记日志跳过，
/// 禁止伪造审批人或静默插占位行。数据库错误原样上抛。
async fn resolve_default_approver(
    db: &DatabaseConnection,
) -> Result<Option<user::Model>, AppError> {
    let Some(admin_role) = role::Entity::find()
        .filter(role::Column::Code.eq(ADMIN_ROLE_CODE))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let approver = user::Entity::find()
        .filter(user::Column::RoleId.eq(admin_role.id))
        .filter(user::Column::IsActive.eq(true))
        .order_by_asc(user::Column::Id)
        .limit(1)
        .one(db)
        .await?;
    Ok(approver)
}

/// 幂等落库出厂内置审批流程定义：按 `code` 唯一约束 ON CONFLICT DO NOTHING，只补不覆盖。
///
/// 成功路径打印总数/实写/跳过与所绑定的审批人；写库失败上抛 [`AppError`]（不吞错），
/// 由启动链决定降级口径。调用方：完整模式启动链（service_bootstrap）与系统初始化完成处
/// （init 初始化链路），两处同函数保证 fresh 部署与存量部署都真实收敛。
pub async fn seed_builtin_bpm_definitions(db: &DatabaseConnection) -> Result<(), AppError> {
    let Some(approver) = resolve_default_approver(db).await? else {
        warn!(
            admin_role_code = ADMIN_ROLE_CODE,
            "BPM 出厂流程定义种子跳过：admin 角色或其启用用户尚不存在（系统未完成初始化），\
             初始化完成时将当场补种，或于下次服务启动自动重试"
        );
        return Ok(());
    };

    let config = build_single_node_approval_config(approver.id);
    let now = chrono::Utc::now();
    let total = BUILTIN_APPROVAL_DEFINITIONS.len();
    let codes: Vec<String> = BUILTIN_APPROVAL_DEFINITIONS
        .iter()
        .map(|def| def.code.to_string())
        .collect();

    // 先查已在册编码，只为缺失者构造插入行——实写数与跳过数必须是查出来的事实，
    // 不能取自写库返回值（sea-orm 的 InsertManyResult 不携带受影响行数，拿它计数会得到错数）。
    let existing: Vec<String> = bpm_process_definition::Entity::find()
        .filter(bpm_process_definition::Column::Code.is_in(codes.clone()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.code)
        .collect();
    let missing: Vec<&BuiltinApprovalDefinition> = BUILTIN_APPROVAL_DEFINITIONS
        .iter()
        .filter(|def| !existing.iter().any(|row| row.as_str() == def.code))
        .collect();

    if missing.is_empty() {
        info!(
            total,
            inserted = 0usize,
            skipped = total,
            assignee_user_id = approver.id,
            "BPM 出厂审批流程定义种子无需写入（四枚编码均已在册，既有定义与用户调整不覆盖）"
        );
        return Ok(());
    }

    let models: Vec<bpm_process_definition::ActiveModel> = missing
        .iter()
        .map(|def| bpm_process_definition::ActiveModel {
            id: Default::default(),
            name: Set(def.name.to_string()),
            code: Set(def.code.to_string()),
            description: Set(Some(def.description.to_string())),
            category: Set(Some(def.category.to_string())),
            version: Set(Some("v1".to_string())),
            config: Set(Some(config.clone())),
            status: Set(DEFINITION_STATUS_ACTIVE.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect();
    let to_write = models.len();

    bpm_process_definition::Entity::insert_many(models)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(bpm_process_definition::Column::Code)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await
        .map_err(|e| AppError::database(format!("BPM 出厂审批流程定义种子写库失败: {e}")))?;

    // 写后复核 出厂清单里的每一枚编码都必须真实在册。ON CONFLICT DO NOTHING 在并发下可能一行也没写进去，
    // 也可能别的进程刚插好——两种都算达成，所以复核按集合而不是按自写计数判。
    let after: Vec<String> = bpm_process_definition::Entity::find()
        .filter(bpm_process_definition::Column::Code.is_in(codes))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.code)
        .collect();
    let still_missing: Vec<&str> = BUILTIN_APPROVAL_DEFINITIONS
        .iter()
        .map(|def| def.code)
        .filter(|code| !after.iter().any(|row| row.as_str() == *code))
        .collect();
    if !still_missing.is_empty() {
        return Err(AppError::database(format!(
            "BPM 出厂审批流程定义种子复核未通过，仍缺编码 {:?}",
            still_missing
        )));
    }

    info!(
        total,
        inserted = to_write,
        skipped = existing.len(),
        assignee_user_id = approver.id,
        "BPM 出厂审批流程定义种子完成（幂等：仅补 code 缺失者，既有定义不覆盖）"
    );
    if !existing.is_empty() {
        info!(
            skipped = existing.len(),
            codes = ?existing,
            "部分出厂流程定义因同编码已存在而跳过（保留既有定义与用户调整，未覆盖）"
        );
    }
    Ok(())
}

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::data_permission;
use crate::models::field_permission;
use crate::models::role::{self, Entity as RoleEntity};
use crate::models::role_permission::{self, Entity as RolePermissionEntity};
// V15 P0-S06：权限变更审计
use crate::models::permission_change_audit;
use crate::utils::admin_checker;
use crate::utils::data_scope::DataScope;
use crate::utils::error::AppError;
// V15 P0-S07：权限变更时失效缓存
use crate::middleware::permission::invalidate_permission_cache;
use serde::{Deserialize, Serialize};

/// 角色权限详情
#[derive(Debug, Serialize, Deserialize)]
pub struct RolePermissionDetail {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 角色详情
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleDetail {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub permissions: Option<String>,
    pub is_system: bool,
    /// 行级数据范围（词表唯一来源 `utils/data_scope::DataScope`：all/dept/self，
    /// 落库列 roles.data_scope，由 PATCH /roles/{id}/data-scope 写入）
    pub data_scope: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub permission_list: Option<Vec<RolePermissionDetail>>,
}

/// 创建角色请求
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 更新角色请求（V15 P1-14.12-E：移除 code 字段，禁止修改 role.code 防提权）
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 分配权限请求
#[derive(Debug, Deserialize)]
pub struct AssignPermissionRequest {
    pub role_id: i32,
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
}

/// 批量整批替换（PUT /roles/{id}/permissions）中的单条期望授予。
/// 角色级语义：本结构刻意不含 resource_id —— 批量端点只维护
/// `resource_id IS NULL` 的角色级授予行，记录级授权仍走单条 POST。
#[derive(Debug, Deserialize, Serialize)]
pub struct BatchPermissionEntry {
    pub resource_type: String,
    pub action: String,
    pub allowed: bool,
}

/// 批量整批替换的执行结果（供 handler 写一条批量审计 + 返回最终角色级授予集）。
pub struct ReplacePermissionsResult {
    /// 事务内最终角色级授予行全集（审计 after，同时作为接口出参）
    pub after: Vec<RolePermissionDetail>,
    /// before 快照的 JSON 形态（service 已序列化，handler 直接放入泛化审计，避免二次序列化）
    pub before_snapshot: serde_json::Value,
    /// after 快照的 JSON 形态（同上，兼作批量审计 new_value 与接口出参快照）
    pub after_snapshot: serde_json::Value,
    /// 新增行数
    pub inserted: u64,
    /// 仅 allowed 翻转的更新行数
    pub updated: u64,
    /// 删除行数（期望集未包含的既有角色级行）
    pub deleted: u64,
}

/// 角色权限服务
pub struct RolePermissionService {
    db: Arc<DatabaseConnection>,
}

/// `permission_change_audits.old_value/new_value` 的列宽（VARCHAR(200)，建表 DDL 在
/// 迁移 finance 域）。这两列是**字符串摘要列**，不是快照容器：批量整批替换的
/// before/after 全量 JSON 快照由 audit_logs 的 JSONB 快照列
/// （before_snapshot/after_snapshot/old_value/new_value，DDL 同为 finance 域迁移）
/// 承载，专表行只落按列宽截断的可读摘要。PostgreSQL VARCHAR(n) 按**字符**计长，
/// 因此截断判据用 `chars().count()`，不用字节数。
const PCA_AUDIT_VALUE_COL_CHARS: usize = 200;

/// `role_permissions.resource_type` 列宽（VARCHAR(100)，建表 DDL 在迁移 system 域
/// m0005）。批量端点在入库前按此宽度显式校验，超长直接 400 点名字段，
/// 不允许落到数据库层再变裸 500。
const ROLE_PERMISSION_RESOURCE_TYPE_MAX_CHARS: usize = 100;

/// `role_permissions.action` 列宽（VARCHAR(50)，同上建表 DDL）。
/// permission_code 是派生列（`<module>.<resource>.<action>`，VARCHAR(255)）：
/// module 取值要么是三~十字符的词表常量、要么回退为 resource_type 本身，
/// 故在 resource_type≤100、action≤50 的前置校验成立时派生串最长约 252 字符，
/// 不会超 255，无需再单独校验。
const ROLE_PERMISSION_ACTION_MAX_CHARS: usize = 50;

/// 生成批量授予集的审计摘要（写入 VARCHAR(200) 列前的收敛形态）：
/// `共 N 项：resource:action=allow，…`。逐项累积，预留截断标记的宽度；
/// 集合大到列宽装不下时按项截断，并**必须**留一条 warn 点名角色、列名、
/// 总项数与列出项数（截断可见，不静默丢字）。全量内容不丢：同事务的
/// audit_logs JSONB 快照（handler 侧 AuditEvent）始终带完整 before/after。
/// 兜底再做一次按字符硬截断，从构造上保证返回值恒 ≤ 列宽，杜绝该列再产生 22001。
fn grant_set_audit_summary(rows: &[RolePermissionDetail], column: &str, role_id: i32) -> String {
    // 截断标记 "...(+K 项未列)" 的最坏长度预留（K 按 10 位数字算），保证拼装结果不越界
    const TRUNCATION_RESERVE_CHARS: usize = 20;
    let total = rows.len();
    let head = format!("共 {total} 项：");
    let head_chars = head.chars().count();
    let budget = PCA_AUDIT_VALUE_COL_CHARS.saturating_sub(TRUNCATION_RESERVE_CHARS);

    let mut body = String::new();
    let mut body_chars = 0usize;
    let mut shown = 0usize;
    for row in rows {
        let item = format!(
            "{}:{}={}",
            row.resource_type,
            row.action,
            if row.allowed { "allow" } else { "deny" }
        );
        let item_chars = item.chars().count();
        let sep_chars = usize::from(!body.is_empty());
        if head_chars + body_chars + sep_chars + item_chars > budget {
            break;
        }
        if !body.is_empty() {
            body.push('，');
        }
        body.push_str(&item);
        body_chars += sep_chars + item_chars;
        shown += 1;
    }

    let summary = if shown < total {
        tracing::warn!(
            role_id,
            column,
            total,
            shown,
            col_max_chars = PCA_AUDIT_VALUE_COL_CHARS,
            "权限变更审计摘要按列宽截断（截断可见）：完整授予集合见 audit_logs 的 JSONB 快照列"
        );
        format!("{head}{body}...(+{} 项未列)", total - shown)
    } else {
        format!("{head}{body}")
    };

    // 硬兜底：任何路径下都不返回超过列宽的串（理论上到不了这里，到达即说明上面预算有误）
    if summary.chars().count() > PCA_AUDIT_VALUE_COL_CHARS {
        tracing::warn!(
            role_id,
            column,
            "权限变更审计摘要拼装超出列宽预算，执行按字符硬截断（不应发生，出现需排查预算逻辑）"
        );
        let kept: String = summary
            .chars()
            .take(PCA_AUDIT_VALUE_COL_CHARS - 1)
            .collect();
        return format!("{kept}…");
    }
    summary
}

impl RolePermissionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取角色列表
    pub async fn list_roles(&self) -> Result<Vec<RoleDetail>, AppError> {
        let roles = RoleEntity::find()
            .order_by(role::Column::Code, Order::Asc)
            .all(&*self.db)
            .await?;

        let role_details: Vec<RoleDetail> = roles
            .into_iter()
            .map(|role| RoleDetail {
                id: role.id,
                name: role.name,
                code: role.code,
                description: role.description,
                permissions: role.permissions,
                is_system: role.is_system,
                data_scope: role.data_scope,
                created_at: role.created_at,
                updated_at: role.updated_at,
                permission_list: None,
            })
            .collect();

        Ok(role_details)
    }

    /// 获取角色详情（包含权限列表）
    pub async fn get_role_detail(&self, role_id: i32) -> Result<RoleDetail, AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        // 获取角色权限列表
        let permissions = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .order_by(role_permission::Column::ResourceType, Order::Asc)
            .all(&*self.db)
            .await?;

        let permission_list: Vec<RolePermissionDetail> = permissions
            .into_iter()
            .map(|perm| RolePermissionDetail {
                id: perm.id,
                role_id: perm.role_id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
                created_at: perm.created_at,
                updated_at: perm.updated_at,
            })
            .collect();

        Ok(RoleDetail {
            id: role.id,
            name: role.name,
            code: role.code,
            description: role.description,
            permissions: role.permissions,
            is_system: role.is_system,
            data_scope: role.data_scope,
            created_at: role.created_at,
            updated_at: role.updated_at,
            permission_list: Some(permission_list),
        })
    }

    /// 创建角色
    pub async fn create_role(&self, request: CreateRoleRequest) -> Result<RoleDetail, AppError> {
        // 缺陷 14.1-D 修复：角色编码命名规范校验（仅允许小写字母、数字、下划线，3-50 字符）
        if request.code.len() < 3 || request.code.len() > 50 {
            return Err(AppError::business("角色编码长度必须在 3-50 字符之间"));
        }
        if !request
            .code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(AppError::business("角色编码仅允许小写字母、数字和下划线"));
        }
        // 缺陷 14.5-C 修复：is_system=true 时 code 必须为 admin（应用层约束）
        if request.is_system.unwrap_or(false) && request.code != "admin" {
            return Err(AppError::business(
                "系统角色（is_system=true）的编码必须为 admin",
            ));
        }

        // 检查角色编码是否已存在
        let existing = RoleEntity::find()
            .filter(role::Column::Code.eq(&request.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business("角色编码已存在"));
        }

        let role = role::ActiveModel {
            id: Default::default(),
            name: sea_orm::ActiveValue::Set(request.name),
            code: sea_orm::ActiveValue::Set(request.code),
            description: sea_orm::ActiveValue::Set(request.description),
            permissions: sea_orm::ActiveValue::NotSet,
            is_system: sea_orm::ActiveValue::Set(request.is_system.unwrap_or(false)),
            // V15 P0-S01：新角色默认 data_scope=self（最小权限原则）
            data_scope: sea_orm::ActiveValue::Set("self".to_string()),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };

        let role_entity = role.insert(&*self.db).await?;

        self.get_role_detail(role_entity.id).await
    }

    /// 更新角色（V15 P1-14.12-E：禁止修改 role.code，防止 admin 提权攻击）
    pub async fn update_role(
        &self,
        role_id: i32,
        request: UpdateRoleRequest,
        user_id: i32,
    ) -> Result<RoleDetail, AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        // 系统角色不允许修改
        if role.is_system {
            return Err(AppError::business("系统角色不允许修改"));
        }

        // V15 P1-14.12-E：role.code 不可修改（原 code 更新与重复检查逻辑已删除）

        let mut role_update: role::ActiveModel = role.into();
        if let Some(name) = request.name {
            role_update.name = sea_orm::ActiveValue::Set(name);
        }
        // V15 P1-14.12-E：禁止修改 role.code
        if let Some(description) = request.description {
            role_update.description = sea_orm::ActiveValue::Set(Some(description));
        }
        if let Some(is_system) = request.is_system {
            role_update.is_system = sea_orm::ActiveValue::Set(is_system);
        }
        role_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        let role_entity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            role_update,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        self.get_role_detail(role_entity.id).await
    }

    /// 更新角色行级数据范围（PATCH /roles/{id}/data-scope，roles.data_scope 唯一写入口）。
    ///
    /// 词表校验走 `DataScope` 往返（`utils/data_scope.rs`，all/dept/self 三值小写）：
    /// `parse_scope` 对未知值静默回退 Self_，因此判据是**往返后逐字符相等**
    /// （`parse_scope(s).as_str() == s`）——只接受三枚规范小写值，未知值显式 400，
    /// 不沿用解析器的静默回退（兜底即掩盖错误）。
    /// DEPT_AND_BELOW/CUSTOM 本轮刻意不开放：列宽 VARCHAR(10) 存不下、且无对应执行器。
    /// 系统角色不允许修改（与 `update_role`/`validate_assignable_role` 同口径）。
    pub async fn update_role_data_scope(
        &self,
        role_id: i32,
        data_scope: &str,
        user_id: i32,
    ) -> Result<RoleDetail, AppError> {
        let parsed = DataScope::parse_scope(data_scope);
        if parsed.as_str() != data_scope {
            tracing::warn!(
                role_id,
                input = data_scope,
                "data_scope 取值不在词表（all/dept/self 小写）内，拒绝写入"
            );
            return Err(AppError::validation_displayable(format!(
                "data_scope 取值非法：仅支持 all/dept/self（收到 {data_scope}）"
            )));
        }

        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        if role.is_system {
            return Err(AppError::business("系统角色不允许修改"));
        }

        let mut role_update: role::ActiveModel = role.into();
        role_update.data_scope = sea_orm::ActiveValue::Set(parsed.as_str().to_string());
        role_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        let role_entity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            role_update,
            Some(user_id),
        )
        .await?;

        self.get_role_detail(role_entity.id).await
    }

    /// 删除角色
    ///
    /// 引用口径与 DB 约束逐条对齐（CI `DELETE /roles/40` 撞
    /// `fk_data_permissions_role` 被 `From<DbErr>` 裸映射成 500 之真因）：
    /// - `users.role_id`（`fk_users_role`）= 外部主体引用 ⇒ 仍有绑定用户即整单拒绝，
    ///   绝不代为改动用户数据；条数属查询所得实体值，只进日志不外显。
    /// - `role_permissions`（`fk_role_permissions_role`）/ `data_permissions`
    ///   （`fk_data_permissions_role`）/ `field_permissions`（`fk_field_permissions_role`）
    ///   = 该角色自身的授权配置行（角色聚合内从属数据）。原实现只清 role_permissions，
    ///   且 `delete_data_permission` 是软删（`is_enabled=false`，行仍在）⇒ 配过数据权限的
    ///   角色永久删不掉并回 500。现与角色删除同事务物理清除三类配置行，逐表条数入日志。
    /// - 预校验与清理之后仍可能被并发写入命中 FK ⇒ 统一按 `DB_RELATION` 降级为业务错误，
    ///   失败信封不外泄 500（同族先例 `services/crm/lead.rs::delete_lead`）。
    /// 严禁把 DDL 改成 `ON DELETE CASCADE`：那会静默删掉引用方数据。
    pub async fn delete_role(&self, role_id: i32, user_id: i32) -> Result<(), AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        // 系统角色不允许删除
        if role.is_system {
            return Err(AppError::business("系统角色不允许删除"));
        }

        // 检查是否有用户关联此角色
        let user_count = crate::models::user::Entity::find()
            .filter(crate::models::user::Column::RoleId.eq(role_id))
            .count(&*self.db)
            .await?;

        if user_count > 0 {
            tracing::warn!(
                "角色 {} 删除被拒：仍有 {} 个用户绑定该角色（操作人 {}）",
                role_id,
                user_count,
                user_id
            );
            return Err(AppError::business_displayable(
                "该角色仍有用户在使用，不可删除，请先调整这些用户的角色",
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 删除角色权限
        let removed_role_permissions = RolePermissionEntity::delete_many()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .exec(&txn)
            .await?;

        // 删除该角色的数据权限与字段权限配置行（含软删残留行）——
        // 这两张表以 FK 引用 roles，不清理则角色永久无法删除
        let removed_data_permissions = data_permission::Entity::delete_many()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .exec(&txn)
            .await?;
        let removed_field_permissions = field_permission::Entity::delete_many()
            .filter(field_permission::Column::RoleId.eq(role_id))
            .exec(&txn)
            .await?;
        tracing::info!(
            "角色 {} 删除：随角色清除其授权配置行 role_permissions={}、data_permissions={}、field_permissions={}（操作人 {}）",
            role_id,
            removed_role_permissions.rows_affected,
            removed_data_permissions.rows_affected,
            removed_field_permissions.rows_affected,
            user_id
        );

        // 删除角色（P0 8-3 修复：补审计日志）
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let result = crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            RoleEntity,
            _,
        >(&txn, "role", role_id, Some(user_id))
        .await;

        // 竞态兜底：预校验通过后仍可能被并发写入的引用行命中 FK，
        // 此时降级为与预校验同口径的业务错误（整事务回滚，不留半删状态）
        if let Err(e) = result {
            if matches!(
                e,
                AppError::DatabaseError(ref msg) if msg == crate::utils::messages::err_msg::DB_RELATION
            ) {
                tracing::warn!(
                    "角色 {} 删除命中外键约束（并发新增引用行，操作人 {}）：整事务回滚并降级为业务错误",
                    role_id,
                    user_id
                );
                return Err(AppError::business_displayable(
                    "该角色正被其他数据引用，删除已取消，请稍后重试或先处理关联数据",
                ));
            }
            return Err(e);
        }

        // 提交事务
        txn.commit().await?;
        tracing::info!("角色 {} 删除成功（操作人 {}）", role_id, user_id);

        Ok(())
    }

    // ===== assign_permission 私有 helpers（D08 拆分）=====

    /// 校验角色存在且非系统角色（系统角色不允许改权限）
    async fn validate_assignable_role(&self, role_id: i32) -> Result<(), AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;
        if role.is_system {
            return Err(AppError::business("系统角色不允许修改权限".to_string()));
        }
        Ok(())
    }

    /// V15 P1-14.3-D：SoD 职责分离校验 — 禁止同一角色同时持有 create 与 approve 权限
    /// 审计计划 14.3.1 要求"创建采购 + 审批采购"和"创建销售 + 审批销售"冲突分离。；本函数在 assign_permission 时拦截违反 SoD 的授权：新增 `create` 时，若角色已有 `approve`（或 `*`）则拒绝；新增 `approve` 时，若角色已有 `create`（或 `*`）则拒绝；新增 `*` 时，若角色已有 `create` 或 `approve` 则拒绝；admin 角色（is_system=true）在 validate_assignable_role 已被拦截，不会走到此处。
    async fn validate_sod_create_approve(
        &self,
        request: &AssignPermissionRequest,
    ) -> Result<(), AppError> {
        // 仅当 allowed=true 时才需 SoD 校验（allowed=false 的拒绝型权限不构成冲突）
        if !request.allowed {
            return Ok(());
        }
        let new_action = request.action.as_str();
        // 判断新权限是否涉及 create/approve 语义
        let new_has_create = new_action == "create" || new_action == "*";
        let new_has_approve = new_action == "approve" || new_action == "*";
        if !new_has_create && !new_has_approve {
            return Ok(());
        }

        // 查询该角色同 resource_type 的所有已授权权限
        let existing = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(request.role_id))
            .filter(role_permission::Column::ResourceType.eq(&request.resource_type))
            .filter(role_permission::Column::Allowed.eq(true))
            .all(&*self.db)
            .await?;

        let existing_has_create = existing
            .iter()
            .any(|p| p.action == "create" || p.action == "*");
        let existing_has_approve = existing
            .iter()
            .any(|p| p.action == "approve" || p.action == "*");

        // 冲突检测：create 与 approve 不可共存
        if new_has_create && existing_has_approve {
            return Err(AppError::business(format!(
                "SoD 职责分离冲突：角色 {} 已持有 {}:{} 的 approve 权限，不可再授予 create",
                request.role_id, request.resource_type, request.action
            )));
        }
        if new_has_approve && existing_has_create {
            return Err(AppError::business(format!(
                "SoD 职责分离冲突：角色 {} 已持有 {}:{} 的 create 权限，不可再授予 approve",
                request.role_id, request.resource_type, request.action
            )));
        }
        // 新增 * 通配符时，若已有 create 或 approve 也拒绝（避免通过通配符绕过 SoD）
        if new_action == "*" && (existing_has_create || existing_has_approve) {
            return Err(AppError::business(format!(
                "SoD 职责分离冲突：角色 {} 已持有 {} 的 create/approve 权限，不可授予 * 通配符",
                request.role_id, request.resource_type
            )));
        }
        Ok(())
    }

    /// 按角色+资源+动作查询已存在的权限记录
    async fn find_existing_permission(
        &self,
        request: &AssignPermissionRequest,
    ) -> Result<Option<role_permission::Model>, AppError> {
        let mut query = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(request.role_id))
            .filter(role_permission::Column::ResourceType.eq(&request.resource_type))
            .filter(role_permission::Column::Action.eq(&request.action));
        if let Some(resource_id) = request.resource_id {
            query = query.filter(role_permission::Column::ResourceId.eq(resource_id));
        } else {
            query = query.filter(role_permission::Column::ResourceId.is_null());
        }
        Ok(query.one(&*self.db).await?)
    }

    /// 更新已存在的权限记录（含审计日志 + 缓存失效）
    async fn update_existing_permission(
        &self,
        perm: role_permission::Model,
        request: &AssignPermissionRequest,
        user_id: i32,
    ) -> Result<role_permission::Model, AppError> {
        let old_allowed = perm.allowed;
        let mut perm_update: role_permission::ActiveModel = perm.into();
        perm_update.allowed = sea_orm::ActiveValue::Set(request.allowed);
        perm_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        let perm_entity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            perm_update,
            Some(user_id),
        )
        .await?;
        invalidate_permission_cache(request.role_id);
        self.write_permission_audit(
            "role_permission_assign",
            user_id,
            request.role_id,
            &request.resource_type,
            &request.action,
            Some(old_allowed.to_string()),
            Some(request.allowed.to_string()),
        )
        .await;
        Ok(perm_entity)
    }

    /// 创建新权限记录（含审计日志 + 缓存失效）
    /// B12-P2-1：自动生成 permission_code（三段式：<模块>.<资源>.<操作>）
    async fn create_new_permission(
        &self,
        request: &AssignPermissionRequest,
        user_id: i32,
    ) -> Result<role_permission::Model, AppError> {
        // B12-P2-1：生成 permission_code
        let permission_code =
            Self::generate_permission_code(&request.resource_type, &request.action);

        let permission = role_permission::ActiveModel {
            id: Default::default(),
            role_id: sea_orm::ActiveValue::Set(request.role_id),
            resource_type: sea_orm::ActiveValue::Set(request.resource_type.clone()),
            resource_id: sea_orm::ActiveValue::Set(request.resource_id),
            action: sea_orm::ActiveValue::Set(request.action.clone()),
            allowed: sea_orm::ActiveValue::Set(request.allowed),
            permission_code: sea_orm::ActiveValue::Set(Some(permission_code)),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };
        let perm_entity = permission.insert(&*self.db).await?;
        invalidate_permission_cache(perm_entity.role_id);
        self.write_permission_audit(
            "role_permission_assign",
            user_id,
            perm_entity.role_id,
            &perm_entity.resource_type,
            &perm_entity.action,
            None,
            Some(perm_entity.allowed.to_string()),
        )
        .await;
        Ok(perm_entity)
    }

    /// B12-P2-1：生成 permission_code（三段式：<模块>.<资源>.<操作>）
    fn generate_permission_code(resource_type: &str, action: &str) -> String {
        let module = Self::resolve_module_from_resource(resource_type);
        format!("{}.{}.{}", module, resource_type, action)
    }

    /// B12-P2-1：根据资源类型解析模块名
    fn resolve_module_from_resource(resource_type: &str) -> String {
        match resource_type {
            // IAM 模块
            "users" | "roles" | "departments" | "permissions" | "field-permissions" => {
                "iam".to_string()
            }
            // 目录模块
            "products" | "categories" | "warehouses" | "boms" => "catalog".to_string(),
            // 销售模块
            "orders" | "fabric-orders" | "customers" | "customer-credits" | "sales-contracts"
            | "sales-prices" | "sales-returns" | "quotations" => "sales".to_string(),
            // 采购模块
            "purchase-orders"
            | "purchase-receipts"
            | "purchase-returns"
            | "purchase-contracts"
            | "purchase-prices"
            | "suppliers"
            | "supplier-evaluations" => "purchase".to_string(),
            // 库存模块
            "inventory" | "stock" | "transfers" | "adjustments" | "reservations" | "counts"
            | "batches" | "stock-alerts" | "piece-split" => "inventory".to_string(),
            // 生产模块
            "production-orders"
            | "dye-batches"
            | "dye-recipes"
            | "dye-batch-rework"
            | "dye-batch-quality"
            | "flow-cards"
            | "process-routes"
            | "outsourcing-orders"
            | "outsourcing-receipts"
            | "outsourcing-vouchers"
            | "mrp"
            | "capacity"
            | "scheduling" => "production".to_string(),
            // 质量模块
            "quality-inspections"
            | "quality-issues"
            | "quality-standards"
            | "fabric-inspections"
            | "fabric-defects" => "quality".to_string(),
            // 财务模块
            "vouchers" | "subjects" | "fixed-assets" | "budgets" | "cost-collections" | "ar"
            | "ap" | "gl" | "fund-management" | "fund-transfers" | "currencies"
            | "exchange-rates" | "ar-reconciliations" | "wages" => "finance".to_string(),
            // CRM 模块
            "crm-leads" | "crm-opportunities" | "crm-customers" | "five-dimension"
            | "sales-analysis" => "crm".to_string(),
            // HR 模块
            "employees" | "wage-rates" | "wage-records" => "hr".to_string(),
            // 物流模块
            "logistics" | "ship-orders" | "incoterms" => "logistics".to_string(),
            // 系统模块
            "audit-logs" | "slow-queries" | "system-config" | "print-templates" | "data-import"
            | "dashboard" => "system".to_string(),
            // AI 模块
            "ai-forecast" | "ai-inventory-opt" | "ai-anomaly" | "ai-recommendation"
            | "ai-recipe-opt" | "ai-quality-pred" | "ai-process-opt" | "ai-summary" => {
                "ai".to_string()
            }
            // 默认：使用 resource_type 作为模块名
            _ => resource_type.to_string(),
        }
    }

    /// 将权限 Model 转换为 RolePermissionDetail
    fn build_permission_detail(model: role_permission::Model) -> RolePermissionDetail {
        RolePermissionDetail {
            id: model.id,
            role_id: model.role_id,
            resource_type: model.resource_type,
            resource_id: model.resource_id,
            action: model.action,
            allowed: model.allowed,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }

    /// 分配权限：校验角色→SoD校验→查已有→更新或新建→审计+缓存失效
    pub async fn assign_permission(
        &self,
        request: AssignPermissionRequest,
        user_id: i32,
    ) -> Result<RolePermissionDetail, AppError> {
        self.validate_assignable_role(request.role_id).await?;
        // V15 P1-14.3-D：SoD 职责分离校验（create 与 approve 不可共存）
        self.validate_sod_create_approve(&request).await?;
        let existing = self.find_existing_permission(&request).await?;
        let perm_entity = if let Some(perm) = existing {
            self.update_existing_permission(perm, &request, user_id)
                .await?
        } else {
            self.create_new_permission(&request, user_id).await?
        };
        Ok(Self::build_permission_detail(perm_entity))
    }

    /// SoD 终集合校验（批量整批替换专用，纯函数）：在**写入后的最终授予集**上复算
    /// create/approve 互斥，判据与逐条路径 `validate_sod_create_approve` 同源——
    /// 同一 resource_type 下（仅 allowed=true 行参与判定）：
    /// - 字面 `create` 与字面 `approve` 不可共存；
    /// - `*` 通配与字面 `create`/`approve` 不可共存（防通配符绕过）；
    /// - 单独一枚 `*` 不算冲突（与逐条规则"新增 * 时库里无 create/approve 则放行"等价，
    ///   故通配符不计入 create/approve 本身，只看是否另有字面行）。
    ///
    /// 为什么逐条校验挡不住并发：`validate_sod_create_approve` 读的是**请求各自时刻**
    /// 的 DB 快照，两个并发请求（一个授 create、一个授 approve）能双双通过再双双落库。
    /// 批量路径把校验挪到"事务内写完后的终集合"上，配合 roles 行 `lock_exclusive`
    /// 串行化同角色的并发替换，终集合要么整体成立、要么整体回滚，不再存在中间态。
    fn find_sod_conflict_in_final_set(allowed_rows: &[role_permission::Model]) -> Option<String> {
        let mut by_resource: HashMap<&str, (bool, bool, bool)> = HashMap::new();
        for row in allowed_rows {
            let entry = by_resource
                .entry(row.resource_type.as_str())
                .or_insert((false, false, false));
            match row.action.as_str() {
                "create" => entry.0 = true,
                "approve" => entry.1 = true,
                "*" => entry.2 = true,
                _ => {}
            }
        }
        by_resource
            .iter()
            .find_map(|(resource, (has_create, has_approve, has_wild))| {
                if (*has_create && *has_approve) || (*has_wild && (*has_create || *has_approve)) {
                    Some((*resource).to_string())
                } else {
                    None
                }
            })
    }

    /// 批量整批替换角色的**角色级**授予（PUT /roles/{id}/permissions）。
    ///
    /// 语义与边界：
    /// - body 是期望的完整角色级授予集；`resource_id IS NULL` 的角色级行是替换对象，
    ///   记录级行（`resource_id = Some(..)`）不在本端点作用面——既不覆盖也不删除，
    ///   仍由单条 POST/DELETE 维护（与 `matches_permission` 的记录级授权语义配套：
    ///   记录级行本就不能放行整集合请求，混进替换集只会造成删错行/顶错 id）。
    /// - 全流程单事务：锁角色行（`lock_exclusive` 串行化同角色并发写）→ 读现状 →
    ///   diff 写（新增/仅 allowed 翻转的更新/期望集外的删除）→ **终集合 SoD 复算** →
    ///   一条批量审计（before/after 各一份全量快照）→ 提交；任一环节 Err 即整体回滚。
    /// - 审计从"逐条 2 行 permission_change_audit × N"收敛为整批 1 行，泛化 audit_logs
    ///   由 handler 的单个 AuditEvent（before/after 快照）承载。
    /// - 缓存失效只在提交成功后（与单条路径同口径）。
    pub async fn replace_role_permissions(
        &self,
        role_id: i32,
        entries: Vec<BatchPermissionEntry>,
        user_id: i32,
    ) -> Result<ReplacePermissionsResult, AppError> {
        // 入参去重闸门：同一 (resource_type, action) 出现多次属调用方错误，
        // 显式拒绝而非静默 last-wins（静默合并会掩盖前端 diff bug）。
        let mut desired: HashMap<(String, String), bool> = HashMap::with_capacity(entries.len());
        let mut duplicated: Vec<String> = Vec::new();
        for entry in entries {
            if entry.resource_type.trim().is_empty() || entry.action.trim().is_empty() {
                return Err(AppError::validation_displayable(
                    "批量授予项的 resource_type/action 不能为空",
                ));
            }
            // 入库前置宽度校验：与 role_permissions 列宽逐字对齐（见上面两个常量），
            // 超长直接 400 点名字段与实收字符数，不让它走到 SQL 层炸成裸 500。
            let resource_chars = entry.resource_type.chars().count();
            if resource_chars > ROLE_PERMISSION_RESOURCE_TYPE_MAX_CHARS {
                return Err(AppError::validation_displayable(format!(
                    "批量授予项超长：resource_type「{}」实收 {resource_chars} 字符，超过列宽上限 {ROLE_PERMISSION_RESOURCE_TYPE_MAX_CHARS}",
                    entry.resource_type
                )));
            }
            let action_chars = entry.action.chars().count();
            if action_chars > ROLE_PERMISSION_ACTION_MAX_CHARS {
                return Err(AppError::validation_displayable(format!(
                    "批量授予项超长：action「{}」实收 {action_chars} 字符，超过列宽上限 {ROLE_PERMISSION_ACTION_MAX_CHARS}",
                    entry.action
                )));
            }
            let key = (entry.resource_type.clone(), entry.action.clone());
            if desired.insert(key.clone(), entry.allowed).is_some() {
                duplicated.push(format!("{}:{}", key.0, key.1));
            }
        }
        if !duplicated.is_empty() {
            return Err(AppError::validation_displayable(format!(
                "批量授予集合存在重复项：{}",
                duplicated.join(", ")
            )));
        }

        let txn = (*self.db).begin().await?;

        // 锁角色行：与并发替换/其它 lock_exclusive 写点互斥；角色不存在 404。
        let role = RoleEntity::find_by_id(role_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;
        if role.is_system {
            return Err(AppError::business_displayable(
                "系统角色不允许修改权限，请改用自定义角色授权".to_string(),
            ));
        }

        // 现状（角色级行；(resource_type, action) → allowed）
        let existing_rows = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::ResourceId.is_null())
            .all(&txn)
            .await?;
        let existing_map: HashMap<(String, String), role_permission::Model> = existing_rows
            .into_iter()
            .map(|row| ((row.resource_type.clone(), row.action.clone()), row))
            .collect();
        let mut before: Vec<RolePermissionDetail> = existing_map
            .values()
            .map(|row| Self::build_permission_detail(row.clone()))
            .collect();
        // 快照按 (resource_type, action) 稳定排序，审计 diff 与人工比对不受哈希序影响
        before.sort_by(|a, b| (&a.resource_type, &a.action).cmp(&(&b.resource_type, &b.action)));

        let now = chrono::Utc::now();
        let mut inserted = 0u64;
        let mut updated = 0u64;
        let mut deleted = 0u64;

        for ((resource_type, action), allowed) in &desired {
            match existing_map.get(&(resource_type.clone(), action.clone())) {
                Some(row) if row.allowed == *allowed => {}
                Some(row) => {
                    let mut active: role_permission::ActiveModel = row.clone().into();
                    active.allowed = sea_orm::ActiveValue::Set(*allowed);
                    active.updated_at = sea_orm::ActiveValue::Set(now);
                    active.update(&txn).await?;
                    updated += 1;
                }
                None => {
                    let permission_code = Self::generate_permission_code(resource_type, action);
                    role_permission::ActiveModel {
                        id: Default::default(),
                        role_id: sea_orm::ActiveValue::Set(role_id),
                        resource_type: sea_orm::ActiveValue::Set(resource_type.clone()),
                        resource_id: sea_orm::ActiveValue::Set(None),
                        action: sea_orm::ActiveValue::Set(action.clone()),
                        allowed: sea_orm::ActiveValue::Set(*allowed),
                        permission_code: sea_orm::ActiveValue::Set(Some(permission_code)),
                        created_at: sea_orm::ActiveValue::Set(now),
                        updated_at: sea_orm::ActiveValue::Set(now),
                    }
                    .insert(&txn)
                    .await?;
                    inserted += 1;
                }
            }
        }
        for ((resource_type, action), row) in &existing_map {
            if !desired.contains_key(&(resource_type.clone(), action.clone())) {
                RolePermissionEntity::delete_many()
                    .filter(role_permission::Column::Id.eq(row.id))
                    .exec(&txn)
                    .await?;
                deleted += 1;
            }
        }

        // SoD 在写入后的最终集合上复算（含记录级 allowed=true 行，与逐条路径
        // "按 resource_type 看全部已授权行"的口径同源）。违规则 Err 上抛，
        // 事务随 drop 回滚，不留下半截授权。
        let final_allowed_rows = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::Allowed.eq(true))
            .all(&txn)
            .await?;
        if let Some(resource) = Self::find_sod_conflict_in_final_set(&final_allowed_rows) {
            tracing::warn!(
                role_id,
                resource_type = %resource,
                operator_id = user_id,
                "批量整批替换命中 SoD 终集合冲突：create/approve 共存，整事务回滚"
            );
            return Err(AppError::business(format!(
                "SoD 职责分离冲突：角色 {role_id} 在最终授予集合上对资源 {resource} 同时持有 create 与 approve，写入已整体回滚"
            )));
        }

        // 最终角色级集合（after 快照 + 接口出参）
        let after_rows = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::ResourceId.is_null())
            .order_by(role_permission::Column::ResourceType, Order::Asc)
            .order_by(role_permission::Column::Action, Order::Asc)
            .all(&txn)
            .await?;
        let after: Vec<RolePermissionDetail> = after_rows
            .into_iter()
            .map(Self::build_permission_detail)
            .collect();

        // 全量 before/after 快照序列化：随 ReplacePermissionsResult 返回给 handler，
        // 由 AuditEvent 写入 audit_logs 的 JSONB 快照列（宽度无上限）。
        let before_json = serde_json::to_value(&before)
            .map_err(|e| AppError::internal(format!("序列化批量审计 before 快照失败: {e}")))?;
        let after_json = serde_json::to_value(&after)
            .map_err(|e| AppError::internal(format!("序列化批量审计 after 快照失败: {e}")))?;
        // 专表一行批量审计：old_value/new_value 是 VARCHAR(200) 字符串列，
        // 只落按列宽收敛的摘要（截断可见：grant_set_audit_summary 内部对超限集合
        // 记 warn 点名角色与列），全量内容不丢——上面两枚 JSONB 快照始终完整。
        permission_change_audit::ActiveModel {
            id: Default::default(),
            change_type: sea_orm::ActiveValue::Set("role_permission_batch_replace".to_string()),
            operator_id: sea_orm::ActiveValue::Set(user_id),
            role_id: sea_orm::ActiveValue::Set(Some(role_id)),
            user_id: sea_orm::ActiveValue::Set(None),
            resource_type: sea_orm::ActiveValue::Set(None),
            action: sea_orm::ActiveValue::Set(None),
            old_value: sea_orm::ActiveValue::Set(Some(grant_set_audit_summary(
                &before,
                "old_value",
                role_id,
            ))),
            new_value: sea_orm::ActiveValue::Set(Some(grant_set_audit_summary(
                &after,
                "new_value",
                role_id,
            ))),
            changed_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            client_ip: sea_orm::ActiveValue::Set(None),
            remark: sea_orm::ActiveValue::Set(Some(format!(
                "批量整批替换角色级授予：新增 {inserted}、更新 {updated}、删除 {deleted}"
            ))),
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        invalidate_permission_cache(role_id);
        tracing::info!(
            role_id,
            inserted,
            updated,
            deleted,
            operator_id = user_id,
            "角色级授予批量整批替换完成（SoD 已按终集合复算）"
        );

        Ok(ReplacePermissionsResult {
            after,
            before_snapshot: before_json,
            after_snapshot: after_json,
            inserted,
            updated,
            deleted,
        })
    }

    /// 移除权限
    pub async fn remove_permission(
        &self,
        permission_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let permission = RolePermissionEntity::find_by_id(permission_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("权限 {} 未找到", permission_id)))?;

        // 检查是否为系统角色的权限
        let role = RoleEntity::find_by_id(permission.role_id)
            .one(&*self.db)
            .await?;

        if let Some(r) = role {
            if r.is_system {
                return Err(AppError::business("系统角色的权限不允许删除".to_string()));
            }
        }

        // V15 P0-S06：保存被删除权限的信息用于审计日志（删除后无法再获取）
        let audit_role_id = permission.role_id;
        let audit_resource_type = permission.resource_type.clone();
        let audit_action = permission.action.clone();
        let audit_old_allowed = permission.allowed;

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let result = crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            RolePermissionEntity,
            _,
        >(&*self.db, "role_permission", permission_id, Some(user_id))
        .await;

        // V15 P0-S07：权限删除后失效该角色的权限缓存
        if result.is_ok() {
            invalidate_permission_cache(audit_role_id);

            // V15 P0-S06：写入权限删除审计日志（best-effort，失败不阻塞主流程）
            self.write_permission_audit(
                "role_permission_remove",
                user_id,
                audit_role_id,
                &audit_resource_type,
                &audit_action,
                Some(audit_old_allowed.to_string()),
                None,
            )
            .await;
        }

        result
    }

    /// V15 P0-S06 新增：写入权限变更审计日志（best-effort，失败仅记录 warn 不阻塞主流程）
    /// 记录角色权限的分配/移除变更，用于合规审查和安全追溯。；审计日志写入失败不影响业务主流程，仅记录 warn 日志。；# 参数；`change_type`：变更类型（role_permission_assign / role_permission_remove）；`operator_id`：操作人 ID；`role_id`：受影响角色 ID；`resource_type`：资源类型；`action`：操作权限码；`old_value`：旧值（如旧 allowed）；`new_value`：新值（如新 allowed）
    async fn write_permission_audit(
        &self,
        change_type: &str,
        operator_id: i32,
        role_id: i32,
        resource_type: &str,
        action: &str,
        old_value: Option<String>,
        new_value: Option<String>,
    ) {
        let audit = permission_change_audit::ActiveModel {
            id: Default::default(),
            change_type: sea_orm::ActiveValue::Set(change_type.to_string()),
            operator_id: sea_orm::ActiveValue::Set(operator_id),
            role_id: sea_orm::ActiveValue::Set(Some(role_id)),
            user_id: sea_orm::ActiveValue::Set(None),
            resource_type: sea_orm::ActiveValue::Set(Some(resource_type.to_string())),
            action: sea_orm::ActiveValue::Set(Some(action.to_string())),
            old_value: sea_orm::ActiveValue::Set(old_value),
            new_value: sea_orm::ActiveValue::Set(new_value),
            changed_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            client_ip: sea_orm::ActiveValue::Set(None),
            remark: sea_orm::ActiveValue::Set(None),
        };
        if let Err(e) = audit.insert(&*self.db).await {
            tracing::warn!(
                target: "security_audit",
                event = "PERMISSION_AUDIT_WRITE_FAILED",
                change_type = change_type,
                role_id = role_id,
                operator_id = operator_id,
                error = %e,
                "权限变更审计日志写入失败（best-effort，不阻塞主流程）"
            );
        }
    }

    /// 检查角色是否为管理员角色（带缓存）
    async fn is_admin_role(&self, role_id: i32) -> Result<bool, AppError> {
        Ok(admin_checker::is_admin_role(&self.db, role_id).await)
    }

    /// 获取角色的全部**角色级**权限行（GET /roles/{id}/permissions）。
    ///
    /// 角色级授予语义（本轮修正的原缺陷）：只返回 `resource_id IS NULL` 的角色级行，
    /// 且 allowed 两桶都返回（授予行 + 显式拒绝行，靠 `allowed` 字段区分）。
    /// 原实现不过滤 resource_id，前端 RoleTab 以 `${resource_type}::${action}` 为键
    /// 做后写覆盖去重：同键的记录级行会顶掉角色级行的关联 id，取消勾选时
    /// DELETE /roles/permissions/{id} 删错行（删到记录级行、角色级行残留）。
    ///
    /// 边界与消费方影响：
    /// - 记录级授权行（`resource_id = Some(..)`）**不再从本端点返回**：它们在权限
    ///   中间件 `matches_permission` 里照常生效（仅放行同实例），照常由单条
    ///   POST /roles/{id}/permissions 写入、GET /roles/{id} 详情的 permission_list 全量可见；
    ///   本端点服务的是"角色 × 资源 × 动作"勾选矩阵，记录级行不属于该矩阵的键空间。
    /// - 出参行的 `resource_id` 恒为 null；`id` 即角色级关联行主键，是前端 DELETE
    ///   唯一应操作的 id，不再有记录级行混入顶替。
    pub async fn get_role_permissions(
        &self,
        role_id: i32,
    ) -> Result<Vec<RolePermissionDetail>, AppError> {
        let permissions = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::ResourceId.is_null())
            .order_by(role_permission::Column::ResourceType, Order::Asc)
            .order_by(role_permission::Column::Action, Order::Asc)
            .all(&*self.db)
            .await?;

        let permission_list: Vec<RolePermissionDetail> = permissions
            .into_iter()
            .map(|perm| RolePermissionDetail {
                id: perm.id,
                role_id: perm.role_id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
                created_at: perm.created_at,
                updated_at: perm.updated_at,
            })
            .collect();

        Ok(permission_list)
    }

    /// 检查权限
    pub async fn check_permission(
        &self,
        role_id: i32,
        resource_type: &str,
        action: &str,
        resource_id: Option<i32>,
    ) -> Result<bool, AppError> {
        // Admin 角色绕过所有权限检查（从数据库查询角色编码，而非硬编码 ID）
        if self.is_admin_role(role_id).await? {
            return Ok(true);
        }

        // 查询角色权限
        let mut query = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::ResourceType.eq(resource_type))
            .filter(role_permission::Column::Action.eq(action));

        // 如果有具体的资源 ID，优先匹配
        if let Some(rid) = resource_id {
            query = query.filter(
                role_permission::Column::ResourceId
                    .eq(rid)
                    .or(role_permission::Column::ResourceId.is_null()),
            );
        }

        let permissions = query
            .order_by(role_permission::Column::ResourceId, Order::Desc)
            .all(&*self.db)
            .await?;

        // 返回第一个匹配的权限结果
        if let Some(perm) = permissions.into_iter().next() {
            Ok(perm.allowed)
        } else {
            // 默认拒绝
            Ok(false)
        }
    }
}

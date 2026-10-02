//! CRM 写侧越权门（用户 2026-10-02 裁定**方案 A**：读可 All，写须 owner 或显式
//! 「管理员代操作」权限键 + 留痕）。
//!
//! ## 为什么要单独立一个门
//! `utils/data_scope::check_resource_owner` 的 `DataScope::All` 分支"始终通过"，
//! 而它同时被**读**（详情/360 视图）和**写**（更新/删除/合并）两条链路复用。
//! "能看全库"被顺带当成"能改任何人的行"，水平越权就是这么长出来的：e2e
//! `flow/34-horizontal-privilege` 与 Rust `contract_wave6_crm_pool_owner_test`
//! 一直按两种相反口径各自钉死，永远互相打红。方案 A 把两者分开：
//! - 读侧：`check_resource_owner` 语义不变（All 可读全域，审计/报表/主管复核需要）；
//! - 写侧：本人行恒可写；跨 owner 写必须持有显式权限键
//!   `resource_type="crm"` / `action="cross_owner_write"`（见 [`CROSS_OWNER_WRITE_KEY`]）。
//!   `RolePermissionService::check_permission` 对 admin 角色内置放行（`is_admin_role`），
//!   所以默认形态＝"只有超管能代操作，其他 all 范围角色须被显式授予该键"，
//!   比原来"凡 data_scope=all 都能改任何人"严格，且不夺走超管的运维能力。
//!
//! ## 留痕
//! 代操作写成功放行时打结构化 warn 日志（actor/role/resource/owner），且各写入口
//! 本身都经 `AuditLogService::update_with_audit` 落审计行（actor=操作人），可按
//! trace_id 回溯。拒绝出参仍是固定脱敏常量（权限文案永久脱敏是用户 2026-10-01 硬令），
//! 具体原因只进日志。

use crate::middleware::auth_context::AuthContext;
use crate::services::role_permission_service::RolePermissionService;
use crate::utils::data_scope::{DataScope, DataScopeContext};
use crate::utils::error::AppError;
use std::sync::Arc;

/// 「管理员代操作」权限键（与 role_permissions 表的 resource_type/action 成对）。
/// 不在迁移里给任何角色播种：默认只有 admin（check_permission 内置放行）能用，
/// 其他 all 范围角色须由运维在权限管理界面显式授予。
pub const CROSS_OWNER_WRITE_KEY: (&str, &str) = ("crm", "cross_owner_write");

/// 跨 owner 写准入：本人行直接放行；All 范围查权限键；Dept/Self 走原归属判定
/// （部门内代管是 dept 的本职，不需要额外键）。
///
/// `Err(permission_denied)` 时出参为固定脱敏常量 + FORBIDDEN，调用方不得重包装成 500。
pub async fn ensure_cross_owner_write_allowed(
    state_db: Arc<sea_orm::DatabaseConnection>,
    auth: &AuthContext,
    ctx: &DataScopeContext,
    resource_owner_id: Option<i32>,
    resource_dept_id: Option<i32>,
    resource_label: &str,
) -> Result<(), AppError> {
    // 本人行：任何 scope 都可写，无需查键（也最常见的路径，避免多一次 DB 往返）
    if resource_owner_id.is_some_and(|owner| owner == ctx.user_id) {
        return Ok(());
    }

    if ctx.scope != DataScope::All {
        // Dept / Self：沿用原归属判定语义（可见部门 / 仅本人），不引入新键
        if crate::utils::data_scope::check_resource_write_owner(ctx, resource_owner_id, resource_dept_id, false)
        {
            return Ok(());
        }
        return Err(AppError::permission_denied(format!(
            "无权操作 {resource_label}（数据范围限制）"
        )));
    }

    // All 范围跨 owner 写：必须持有显式代操作键
    let role_id = auth.role_id.ok_or_else(|| {
        // 角色未加载＝无法证明授权，按最小权限拒绝（fail-closed，不静默放行）
        tracing::warn!(
            actor = auth.user_id,
            resource = resource_label,
            resource_owner = ?resource_owner_id,
            "代操作写被拒：角色未加载，无法校验跨 owner 写权限键"
        );
        AppError::permission_denied(format!("无权操作 {resource_label}（数据范围限制）"))
    })?;

    let granted = RolePermissionService::new(state_db)
        .check_permission(
            role_id,
            CROSS_OWNER_WRITE_KEY.0,
            CROSS_OWNER_WRITE_KEY.1,
            None,
        )
        .await?;

    if !granted {
        tracing::warn!(
            actor = auth.user_id,
            role = role_id,
            resource = resource_label,
            resource_owner = ?resource_owner_id,
            key = CROSS_OWNER_WRITE_KEY.0,
            action = CROSS_OWNER_WRITE_KEY.1,
            "代操作写被拒：All 范围但未持有跨 owner 写权限键"
        );
        return Err(AppError::permission_denied(format!(
            "无权操作 {resource_label}（数据范围限制）"
        )));
    }

    // 放行即留痕：真正发生了一次"代他人操作"，日志与审计行（update_with_audit）双轨可查
    tracing::info!(
        actor = auth.user_id,
        role = role_id,
        resource = resource_label,
        resource_owner = ?resource_owner_id,
        "代操作写放行（方案 A：All 范围 + 显式 cross_owner_write 键）"
    );
    Ok(())
}

use crate::models::role;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
// P0-D03（Batch 488）：Redis 分布式缓存接入（find_by_id 读穿透 + 写失效）
use crate::utils::redis_cache::{
    cache_key, redis_cache_del, redis_cache_get_json, redis_cache_set_json, DEFAULT_CACHE_TTL_SECS,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RoleService {
    db: Arc<DatabaseConnection>,
}

impl RoleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 根据 ID 查找角色
    /// P0-D03（Batch 488）：接入 Redis 分布式缓存（5 分钟 TTL）；读穿透：先查 Redis，未命中查 DB 后回填 Redis；写失效：update/delete 时清除对应 key
    pub async fn find_by_id(&self, id: i32) -> Result<role::Model, AppError> {
        // P0-D03：先查 Redis 缓存
        let cache_key_str = cache_key("role", id);
        if let Some(cached) = redis_cache_get_json::<role::Model>(&cache_key_str).await {
            return Ok(cached);
        }

        // 缓存未命中 → 查询 DB
        let role = role::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 ID {} 不存在", id)))?;

        // 回填 Redis 缓存（5 分钟 TTL）
        redis_cache_set_json(&cache_key_str, &role, DEFAULT_CACHE_TTL_SECS).await;

        Ok(role)
    }

    /// 根据编码查找角色
    pub async fn find_by_code(&self, code: &str) -> Result<role::Model, AppError> {
        role::Entity::find()
            .filter(role::Column::Code.eq(code))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色编码 {} 不存在", code)))
    }

    /// 创建角色（V15 P0-S01：新增 data_scope 参数（all/dept/self），默认 self）
    pub async fn create_role(
        &self,
        name: String,
        code: String,
        description: Option<String>,
        permissions: Option<String>,
        is_system: bool,
        data_scope: Option<String>,
    ) -> Result<role::Model, AppError> {
        // V15 P2 14.5-C：is_system 只能用于 admin 系统角色，防止普通角色被标记为系统角色
        if is_system && code != "admin" {
            return Err(AppError::validation(
                "仅 admin 角色可标记为系统角色（is_system=true）",
            ));
        }
        let active_role = role::ActiveModel {
            id: Default::default(),
            name: Set(name),
            code: Set(code),
            description: Set(description),
            permissions: Set(permissions),
            is_system: Set(is_system),
            // V15 P0-S01：数据范围，未指定时默认 self（最小权限原则）
            data_scope: Set(data_scope.unwrap_or_else(|| "self".to_string())),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_role.insert(&*self.db).await
    }

    /// 更新角色信息（V15 P1-14.12-E：role.code 不可修改，移除 code 参数防提权）
    /// 批次 86 v2 复审 P2-1 修复：find + 状态门 + update 移入单一事务 + lock_exclusive 串行化
    pub async fn update_role(
        &self,
        role_id: i32,
        name: Option<String>,
        description: Option<String>,
        permissions: Option<String>,
        is_system: Option<bool>,
    ) -> Result<role::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let role_model = role::Entity::find_by_id(role_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 ID {} 不存在", role_id)))?;

        // 系统角色不允许修改
        if role_model.is_system {
            return Err(AppError::business("系统角色不允许修改"));
        }

        let mut role_active: role::ActiveModel = role_model.into();

        if let Some(name) = name {
            role_active.name = Set(name);
        }
        // V15 P1-14.12-E：禁止修改 role.code，防止 admin 将其他角色 code 改为 "admin" 提权
        if let Some(description) = description {
            role_active.description = Set(Some(description));
        }
        if let Some(permissions) = permissions {
            role_active.permissions = Set(Some(permissions));
        }
        if let Some(is_system) = is_system {
            role_active.is_system = Set(is_system);
        }

        role_active.updated_at = Set(Utc::now());
        let result = role_active.update(&txn).await?;
        txn.commit().await?;

        // P0-D03：失效角色缓存（角色信息已更新）
        redis_cache_del(&cache_key("role", role_id)).await;

        Ok(result)
    }

    /// 删除角色（批次 86 v2 复审 P2-2 修复：find + 状态门 + delete 移入单一事务 + lock_exclusive 串行化）
    ///
    /// 引用口径与 `RolePermissionService::delete_role` 完全一致（同族先例
    /// `services/crm/lead.rs::delete_lead`）：`users.role_id` 属外部主体引用 ⇒ 有绑定即拒；
    /// `role_permissions`/`data_permissions`/`field_permissions` 是角色自身的授权配置行 ⇒
    /// 同事务物理清除（`data_permissions` 的软删残留行同样会以 FK 永久阻塞删除，
    /// CI #4669 `DELETE /roles/40` 的 500 即此）。禁止 `ON DELETE CASCADE` 静默删引用方数据。
    pub async fn delete_role(&self, role_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let role_model = role::Entity::find_by_id(role_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 ID {} 不存在", role_id)))?;

        // 系统角色不允许删除
        if role_model.is_system {
            return Err(AppError::business("系统角色不允许删除"));
        }

        // 外部主体引用预校验：仍有用户绑定该角色（users.role_id → fk_users_role）即拒绝，
        // 不代为改动用户数据；条数只进日志不外显
        let bound_users = crate::models::user::Entity::find()
            .filter(crate::models::user::Column::RoleId.eq(role_id))
            .count(&txn)
            .await?;
        if bound_users > 0 {
            tracing::warn!("角色 {} 删除被拒：仍有 {} 个用户绑定该角色", role_id, bound_users);
            return Err(AppError::business_displayable(
                "该角色仍有用户在使用，不可删除，请先调整这些用户的角色",
            ));
        }

        // 角色自身授权配置行随角色一并清除（三张表均以 FK 引用 roles）
        let removed_role_permissions = crate::models::role_permission::Entity::delete_many()
                .filter(crate::models::role_permission::Column::RoleId.eq(role_id))
                .exec(&txn)
                .await?;
        let removed_data_permissions = crate::models::data_permission::Entity::delete_many()
            .filter(crate::models::data_permission::Column::RoleId.eq(role_id))
            .exec(&txn)
            .await?;
        let removed_field_permissions = crate::models::field_permission::Entity::delete_many()
            .filter(crate::models::field_permission::Column::RoleId.eq(role_id))
            .exec(&txn)
            .await?;
        tracing::info!(
            "角色 {} 删除：随角色清除授权配置行 role_permissions={}、data_permissions={}、field_permissions={}",
            role_id,
            removed_role_permissions.rows_affected,
            removed_data_permissions.rows_affected,
            removed_field_permissions.rows_affected
        );

        let role_active: role::ActiveModel = role_model.into();
        let result = role_active.delete(&txn).await.map_err(AppError::from);
        if let Err(e) = result {
            // 并发引用兜底：FK 命中（DbErr → DatabaseError(DB_RELATION)，分类见 utils/error.rs）
            // 降级为业务错误，事务整体回滚不留半删状态，失败信封不外泄 500
            if matches!(
                e,
                AppError::DatabaseError(ref msg)
                    if msg == crate::utils::messages::err_msg::DB_RELATION
            ) {
                tracing::warn!(
                    "角色 {} 删除命中外键约束（并发引用）：整事务回滚并降级为业务错误",
                    role_id
                );
                return Err(AppError::business_displayable(
                    "该角色正被其他数据引用，删除已取消，请稍后重试或先处理关联数据",
                ));
            }
            return Err(e);
        }
        txn.commit().await?;
        tracing::info!("角色 {} 删除成功", role_id);

        // P0-D03：失效角色缓存（角色已删除）
        redis_cache_del(&cache_key("role", role_id)).await;

        Ok(())
    }

    /// 获取角色列表（分页）
    pub async fn list_roles(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<role::Model>, u64), AppError> {
        let paginator = role::Entity::find().paginate(&*self.db, page_size);

        // 批次 255 修复：接入 paginate_with_total 统一分页逻辑
        // 修复原 bug：fetch_page(page) 未做 saturating_sub(1) 偏移，导致第一页跳到第二页
        // 补充 page.clamp(1, 1000) 防 DoS
        let (roles, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((roles, total))
    }

    /// 获取所有角色（不分页）
    /// P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
    pub async fn get_all_roles(&self) -> Result<Vec<role::Model>, AppError> {
        role::Entity::find().limit(10_000).all(&*self.db).await
    }
}

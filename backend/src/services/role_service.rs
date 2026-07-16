use crate::models::role;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
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
    pub async fn find_by_id(&self, id: i32) -> Result<role::Model, AppError> {
        role::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 ID {} 不存在", id)))
    }

    /// 根据编码查找角色
    pub async fn find_by_code(&self, code: &str) -> Result<role::Model, AppError> {
        role::Entity::find()
            .filter(role::Column::Code.eq(code))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色编码 {} 不存在", code)))
    }

    /// 创建角色
    ///
    /// V15 P0-S01：新增 data_scope 参数（all/dept/self），默认 self
    pub async fn create_role(
        &self,
        name: String,
        code: String,
        description: Option<String>,
        permissions: Option<String>,
        is_system: bool,
        data_scope: Option<String>,
    ) -> Result<role::Model, AppError> {
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

    /// 更新角色信息
    ///
    /// 批次 86 v2 复审 P2-1 修复：find + 状态门 + update 移入单一事务 + lock_exclusive 串行化
    /// 原实现全程用 self.db，无 txn 无 lock，存在 TOCTOU
    /// （并发 update/delete 会基于过期状态通过检查后写入）
    pub async fn update_role(
        &self,
        role_id: i32,
        name: Option<String>,
        code: Option<String>,
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
        if let Some(code) = code {
            role_active.code = Set(code);
        }
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
        Ok(result)
    }

    /// 删除角色
    ///
    /// 批次 86 v2 复审 P2-2 修复：find + 状态门 + delete 移入单一事务 + lock_exclusive 串行化
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

        let role_active: role::ActiveModel = role_model.into();
        role_active.delete(&txn).await?;
        txn.commit().await?;
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

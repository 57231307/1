use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::models::warehouse::{self, Entity as WarehouseEntity};
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;

crate::define_service!(WarehouseService);

impl WarehouseService {
    /// 获取仓库列表（支持分页和过滤）
    pub async fn list(
        &self,
        query: crate::handlers::warehouse_handler::WarehouseListQuery,
    ) -> Result<crate::utils::response::PaginatedResponse<warehouse::Model>, AppError> {
        let mut q = WarehouseEntity::find();

        // 应用过滤条件
        if let Some(s) = query.status {
            q = q.filter(warehouse::Column::IsActive.eq(s == "active"));
        }

        if let Some(keyword) = query.search {
            let pattern = safe_like_pattern(&keyword);
            q = q.filter(
                warehouse::Column::Name
                    .like(&pattern)
                    .or(warehouse::Column::WarehouseCode.like(&pattern)),
            );
        }

        // 获取总数
        let total = q.clone().count(&*self.db).await?;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        // 应用分页和排序
        let warehouses = q
            .order_by(warehouse::Column::WarehouseCode, Order::Asc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .into_model::<warehouse::Model>()
            .all(&*self.db)
            .await?;

        Ok(crate::utils::response::PaginatedResponse::new(
            warehouses, total, page, page_size,
        ))
    }

    /// 获取仓库详情
    pub async fn get(&self, id: i32) -> Result<warehouse::Model, AppError> {
        WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", id)))
    }

    /// 创建仓库
    pub async fn create(
        &self,
        req: crate::handlers::warehouse_handler::CreateWarehouseRequest,
    ) -> Result<warehouse::Model, AppError> {
        // 自动生成仓库编码
        let code = match req.code {
            Some(c) if !c.is_empty() => c,
            _ => {
                let timestamp = Utc::now().timestamp_millis();
                let random_suffix = crate::utils::random::random_4_digit();
                format!("WH{:013}{:04}", timestamp, random_suffix)
            }
        };

        // 批次 93 P1 扩展：接入 manager（解析为 manager_id，与 update 方法对齐）
        let manager_id = match req.manager {
            Some(m) if !m.is_empty() => match m.parse::<i32>() {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    tracing::warn!("仓库经理ID解析失败: {} ({})", m, e);
                    return Err(AppError::bad_request(format!("仓库经理ID格式错误：{}", m)));
                }
            },
            _ => None,
        };

        let active_model = warehouse::ActiveModel {
            id: NotSet,
            warehouse_code: Set(code),
            name: Set(req
                .name
                .unwrap_or_else(|| format!("仓库_{}", Utc::now().timestamp()))),
            address: Set(req.address),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            phone: Set(req.phone),
            email: Set(None),
            manager_id: Set(manager_id),
            is_active: Set(true),
            // 批次 93 P1 扩展：接入 description（写入 notes 列，实现原 TODO 占位）
            notes: Set(req.description),
            // 批次 158 v11 真实接入：capacity 字段持久化（原 #[allow(dead_code)] 移除）
            capacity: Set(req.capacity),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新仓库
    ///
    /// 批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，
    /// 保证审计日志能追溯实际更新人。
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        req: crate::handlers::warehouse_handler::UpdateWarehouseRequest,
    ) -> Result<warehouse::Model, AppError> {
        let mut wh: warehouse::ActiveModel = WarehouseEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", id)))?
            .into();

        if let Some(n) = req.name {
            wh.name = Set(n);
        }
        if let Some(a) = req.address {
            wh.address = Set(Some(a));
        }
        if let Some(m) = req.manager {
            // 仓库经理 ID 解析失败时记录 warn 并跳过更新，避免脏数据
            match m.parse::<i32>() {
                Ok(parsed) => wh.manager_id = Set(Some(parsed)),
                Err(e) => {
                    tracing::warn!("仓库经理ID解析失败: {} ({})", m, e);
                    return Err(AppError::bad_request(format!("仓库经理ID格式错误：{}", m)));
                }
            }
        }
        if let Some(p) = req.phone {
            wh.phone = Set(Some(p));
        }
        if let Some(s) = req.status {
            wh.is_active = Set(s == "active");
        }
        // 批次 158 v11 真实接入：capacity 字段持久化（原 #[allow(dead_code)] 移除）
        if let Some(c) = req.capacity {
            wh.capacity = Set(Some(c));
        }

        wh.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            wh,
            Some(user_id),
        )
        .await?;
        Ok(result)
    }

    /// 删除仓库
    ///
    /// 批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，
    /// 保证审计日志能追溯实际删除人。
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            WarehouseEntity,
            _,
        >(&*self.db, "warehouse", id, Some(user_id))
        .await
    }
}

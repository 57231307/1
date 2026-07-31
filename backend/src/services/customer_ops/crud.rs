//! 客户 CRUD impl 子模块（customer_ops/crud）
//!
//! 拆分：从原 `customer_service.rs` 迁移 CustomerService 的基础 CRUD 方法：
//! - create_customer（事务 + lock_exclusive 防重复编码 + ES 同步）
//! - get_customer（读穿透 Redis 缓存 + 行级数据权限校验）
//! - list_customers（基础列表 + 数据权限过滤 + 分页）
//! - delete_customer（软删除 + 状态门 + 事务 + 审计 + 缓存失效 + ES 同步）
//!
//! redis_cache 调用保留在原函数内：get_customer 读穿透 + 回填，delete_customer 失效缓存。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};

use crate::models::customer::{self, Entity as CustomerEntity};
use crate::models::dto::PageRequest;
use crate::models::status::master_data;
use crate::services::customer_ops::types::CreateCustomerArgs;
use crate::services::customer_service::CustomerService;
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
// P0-D03（Batch 488）：Redis 分布式缓存接入（get_customer 读穿透 + 写失效）
// V15 P2 B07-P2-6：使用差异化 TTL（CUSTOMER_CACHE_TTL_SECS=300s，客户数据中低波动率）
use crate::utils::redis_cache::{
    cache_key, redis_cache_del, redis_cache_get_json, redis_cache_set_json, CUSTOMER_CACHE_TTL_SECS,
};
use crate::utils::PaginatedResponse;

impl CustomerService {
    /// 创建客户（事务 + lock_exclusive 防重复编码 + PG 提交后 ES 同步）
    pub async fn create_customer(
        &self,
        args: CreateCustomerArgs,
    ) -> Result<customer::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 检查客户编码是否已存在（lock_exclusive 防止并发创建相同编码）
        let existing = CustomerEntity::find()
            .filter(customer::Column::CustomerCode.eq(&args.customer_code))
            .lock_exclusive()
            .one(&txn)
            .await?;

        if existing.is_some() {
            return Err(AppError::business("客户编码已存在"));
        }

        let customer = Self::build_customer_active_model(args);
        let result = customer.insert(&txn).await.map_err(AppError::from)?;
        txn.commit().await?;

        // PG 事务提交后同步到 ES（最终一致性，ES 失败仅记日志）
        self.sync_customer_to_es(&result, "create").await;

        Ok(result)
    }

    /// 获取客户详情（读穿透 Redis 缓存 + 行级数据权限校验）
    pub async fn get_customer(
        &self,
        customer_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<customer::Model, AppError> {
        // P0-D03：先查 Redis 缓存
        let cache_key_str = cache_key("customer", customer_id);
        let cached: Option<customer::Model> =
            redis_cache_get_json::<customer::Model>(&cache_key_str).await;

        let customer = if let Some(model) = cached {
            model
        } else {
            // 缓存未命中 → 查询 DB
            let model = CustomerEntity::find_by_id(customer_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;
            // 回填 Redis 缓存（V15 P2 B07-P2-6：客户数据 5 分钟 TTL，中低波动率）
            redis_cache_set_json(&cache_key_str, &model, CUSTOMER_CACHE_TTL_SECS).await;
            model
        };

        // V15 P0-S01：行级数据权限校验（IDOR 防护，customer 表无 department_id 退化为 Self）
        // P0-D03：缓存命中的 model 同样需要校验权限，防止越权读取缓存
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, customer.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问客户 {}（数据范围限制）",
                    customer_id
                )));
            }
        }

        Ok(customer)
    }

    /// 获取客户列表（基础筛选：状态 / 类型 / 关键词 + 行级数据权限过滤）
    pub async fn list_customers(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<PaginatedResponse<customer::Model>, AppError> {
        let mut query = CustomerEntity::find();

        // V15 P0-S01：行级数据权限过滤（customer 表无 department_id 退化为 Self）
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                customer::Column::CreatedBy,
                customer::Column::CreatedBy,
            );
        }

        if let Some(status) = status {
            query = query.filter(customer::Column::Status.eq(status));
        }
        if let Some(customer_type) = customer_type {
            query = query.filter(customer::Column::CustomerType.eq(customer_type));
        }
        if let Some(keyword) = keyword {
            query = query.filter(
                customer::Column::CustomerName
                    .contains(&keyword)
                    .or(customer::Column::CustomerCode.contains(&keyword)),
            );
        }

        let total = query.clone().count(&*self.db).await?;
        let offset = page_req.page.saturating_sub(1) * page_req.page_size;
        let customers = query
            .order_by(customer::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(page_req.page_size)
            .all(&*self.db)
            .await?;

        Ok(PaginatedResponse::new(
            customers,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 删除客户（软删除，将状态改为 inactive）
    pub async fn delete_customer(
        &self,
        customer_id: i32,
        user_id: i32,
    ) -> Result<customer::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // lock_exclusive 串行化并发软删除，防止 TOCTOU
        let customer = CustomerEntity::find_by_id(customer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

        // 状态门：已 inactive 的客户拒绝重复软删除
        if customer.status == master_data::INACTIVE {
            return Err(AppError::business(format!(
                "客户 {} 已删除，无需重复操作",
                customer_id
            )));
        }

        let mut customer_update: customer::ActiveModel = customer.into();
        customer_update.status = sea_orm::ActiveValue::Set(master_data::INACTIVE.to_string());
        customer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // 事务内 update_with_audit，原子写入软删除 + 审计日志
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "customer",
            customer_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // P0-D03：失效客户缓存（客户状态已变更为 inactive）
        redis_cache_del(&cache_key("customer", customer_id)).await;

        // 软删除后同步 status=inactive 到 ES（不删除 ES 文档，便于搜索历史客户）
        self.sync_customer_to_es(&updated, "delete").await;

        Ok(updated)
    }
}

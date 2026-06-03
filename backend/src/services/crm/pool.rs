//! CRM 公海服务（crm/pool）
//!
//! 包含公海客户的领取、释放等。
//! 拆分自原 `crm_service.rs`。

use crate::models::customer;
use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;

use super::cust::CrmService;

impl CrmService {
    /// 从公海领取客户
    pub async fn claim_pool_customers(
        &self,
        customer_ids: Vec<i32>,
        user_id: i32,
    ) -> Result<usize, AppError> {
        if customer_ids.is_empty() {
            return Ok(0);
        }

        let mut claimed = 0;
        for cid in customer_ids {
            // 验证客户存在且在公海（owner_id 为空）
            let customer = customer::Entity::find_by_id(cid)
                .one(&*self.db)
                .await?;

            if let Some(c) = customer {
                if c.owner_id.is_some() {
                    tracing::warn!("客户 {} 已被 {} 拥有，无法领取", cid, c.owner_id.unwrap());
                    continue;
                }

                // 领取
                let mut customer_active: customer::ActiveModel = c.into();
                customer_active.owner_id = Set(Some(user_id));
                customer_active.updated_at = Set(chrono::Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &*self.db,
                    "auto_audit",
                    customer_active,
                    Some(0),
                )
                .await?;
                claimed += 1;
            } else {
                tracing::warn!("客户 {} 不存在", cid);
            }
        }

        Ok(claimed)
    }

    /// 列出公海客户（owner_id 为空的客户）
    pub async fn list_pool_customers(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<customer::Model>, u64), AppError> {
        let paginator = customer::Entity::find()
            .filter(customer::Column::OwnerId.is_null())
            .paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;
        Ok((items, total))
    }

    /// 释放客户到公海
    pub async fn release_to_pool(
        &self,
        customer_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let customer = customer::Entity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        // 只能释放自己的客户
        if let Some(owner) = customer.owner_id {
            if owner != user_id {
                return Err(AppError::permission_denied(
                    "只能释放自己的客户到公海".to_string(),
                ));
            }
        } else {
            return Err(AppError::business("客户已在公海".to_string()));
        }

        let mut customer_active: customer::ActiveModel = customer.into();
        customer_active.owner_id = Set(None);
        customer_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            customer_active,
            Some(0),
        )
        .await?;
        Ok(())
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;

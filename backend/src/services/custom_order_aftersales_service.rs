//! 定制订单售后服务
//!
//! 4 种售后类型：客诉 / 维修 / 换货 / 退款
//! 状态机：opened → processing → resolved/closed/rejected
//! 创建时间: 2026-06-17

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::models::after_sales::{self, ActiveModel, Entity};
use crate::utils::app_state::AppState;

/// 创建售后工单 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateAfterSalesDto {
    pub custom_order_id: i64,
    pub customer_id: i64,
    /// 售后类型：complaint / repair / exchange / refund
    pub issue_type: String,
    pub description: String,
    pub refund_amount: Option<Decimal>,
}

/// 更新售后工单 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateAfterSalesDto {
    pub status: Option<String>,
    pub resolution: Option<String>,
    pub refund_amount: Option<Decimal>,
}

/// 业务错误
#[derive(Debug, Error)]
pub enum AfterSalesError {
    #[error("售后工单不存在")]
    NotFound,
    #[error("非法状态: {0}")]
    InvalidState(String),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 售后服务
pub struct CustomOrderAfterSalesService {
    db: Arc<DatabaseConnection>,
}

impl CustomOrderAfterSalesService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建售后工单
    pub async fn create(
        &self,
        dto: CreateAfterSalesDto,
        tenant_id: i64,
    ) -> Result<after_sales::Model, AfterSalesError> {
        // 校验售后类型
        if !["complaint", "repair", "exchange", "refund"].contains(&dto.issue_type.as_str()) {
            return Err(AfterSalesError::Validation(format!(
                "非法售后类型: {}",
                dto.issue_type
            )));
        }

        // 退款类型必须有金额
        if dto.issue_type == "refund" && dto.refund_amount.is_none() {
            return Err(AfterSalesError::Validation(
                "退款类型工单必须填写退款金额".to_string(),
            ));
        }

        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            custom_order_id: Set(dto.custom_order_id),
            issue_type: Set(dto.issue_type),
            customer_id: Set(dto.customer_id),
            description: Set(dto.description),
            status: Set("opened".to_string()),
            opened_at: Set(now),
            closed_at: Set(None),
            resolution: Set(None),
            refund_amount: Set(dto.refund_amount),
            tenant_id: Set(tenant_id),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新售后工单
    pub async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        dto: UpdateAfterSalesDto,
    ) -> Result<after_sales::Model, AfterSalesError> {
        let existing = Entity::find_by_id(id)
            .filter(after_sales::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        // 校验状态转换
        if let Some(new_status) = &dto.status {
            if !is_valid_transition(&existing.status, new_status) {
                return Err(AfterSalesError::InvalidState(format!(
                    "{} → {}",
                    existing.status, new_status
                )));
            }
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        if let Some(v) = dto.status {
            active.status = Set(v.clone());
            if v == "closed" || v == "resolved" || v == "rejected" {
                active.closed_at = Set(Some(now));
            }
        }
        if let Some(v) = dto.resolution {
            active.resolution = Set(Some(v));
        }
        if let Some(v) = dto.refund_amount {
            active.refund_amount = Set(Some(v));
        }
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 列出订单的售后工单
    pub async fn list_by_order(
        &self,
        order_id: i64,
        tenant_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<after_sales::Model>, u64), AfterSalesError> {
        let query = Entity::find()
            .filter(after_sales::Column::CustomOrderId.eq(order_id))
            .filter(after_sales::Column::TenantId.eq(tenant_id));

        let paginator = query
            .order_by_desc(after_sales::Column::OpenedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 按 ID 获取
    pub async fn get_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<after_sales::Model, AfterSalesError> {
        Entity::find_by_id(id)
            .filter(after_sales::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(AfterSalesError::NotFound)
    }
}

/// 状态转换校验
fn is_valid_transition(from: &str, to: &str) -> bool {
    use std::collections::HashMap;
    let mut valid: HashMap<&str, Vec<&str>> = HashMap::new();
    valid.insert("opened", vec!["processing", "rejected", "closed"]);
    valid.insert("processing", vec!["resolved", "closed", "rejected"]);
    valid.insert("resolved", vec!["closed"]);
    valid.insert("closed", vec![]);
    valid.insert("rejected", vec![]);

    valid.get(from).map(|v| v.contains(&to)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transition() {
        assert!(is_valid_transition("opened", "processing"));
        assert!(is_valid_transition("processing", "resolved"));
        assert!(!is_valid_transition("closed", "processing"));
        assert!(!is_valid_transition("opened", "resolved"));
    }
}

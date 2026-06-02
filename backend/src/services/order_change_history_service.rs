#![allow(dead_code)]
use std::sync::Arc;

use chrono::Utc;
use sea_orm::ColumnTrait;
use sea_orm::{
    ActiveValue, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::models::sales_order_change_history;
use crate::utils::error::AppError;

pub struct OrderChangeHistoryService {
    db: Arc<DatabaseConnection>,
}

impl OrderChangeHistoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 记录订单变更历史
    #[allow(clippy::too_many_arguments)]
    pub async fn record_change(
        &self,
        order_id: i32,
        change_type: &str,
        field_name: Option<&str>,
        old_value: Option<&str>,
        new_value: Option<&str>,
        changed_by: i32,
        change_reason: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AppError> {
        let history = sales_order_change_history::ActiveModel {
            order_id: ActiveValue::Set(order_id),
            change_type: ActiveValue::Set(change_type.to_string()),
            field_name: ActiveValue::Set(field_name.map(|s| s.to_string())),
            old_value: ActiveValue::Set(old_value.map(|s| s.to_string())),
            new_value: ActiveValue::Set(new_value.map(|s| s.to_string())),
            changed_by: ActiveValue::Set(changed_by),
            changed_at: ActiveValue::Set(Utc::now()),
            change_reason: ActiveValue::Set(change_reason.map(|s| s.to_string())),
            ip_address: ActiveValue::Set(ip_address.map(|s| s.to_string())),
            user_agent: ActiveValue::Set(user_agent.map(|s| s.to_string())),
            created_at: ActiveValue::Set(Utc::now()),
            updated_at: ActiveValue::Set(Utc::now()),
            ..Default::default()
        };

        sales_order_change_history::Entity::insert(history)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::internal(format!("记录变更历史失败: {}", e)))?;

        Ok(())
    }

    /// 获取订单变更历史列表
    pub async fn get_history_by_order(
        &self,
        order_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<sales_order_change_history::Model>, u64), AppError> {
        let offset = (page - 1) * page_size;

        let histories = sales_order_change_history::Entity::find()
            .filter(sales_order_change_history::Column::OrderId.eq(order_id))
            .order_by_desc(sales_order_change_history::Column::ChangedAt)
            .offset(offset)
            .limit(page_size)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::internal(format!("查询变更历史失败: {}", e)))?;

        let total = sales_order_change_history::Entity::find()
            .filter(sales_order_change_history::Column::OrderId.eq(order_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::internal(format!("查询变更历史总数失败: {}", e)))?;

        Ok((histories, total))
    }

    /// 记录订单创建
    pub async fn record_order_created(
        &self,
        order_id: i32,
        changed_by: i32,
    ) -> Result<(), AppError> {
        self.record_change(
            order_id,
            "CREATE",
            None,
            None,
            None,
            changed_by,
            Some("创建订单"),
            None,
            None,
        )
        .await
    }

    /// 记录字段变更
    pub async fn record_field_change(
        &self,
        order_id: i32,
        field_name: &str,
        old_value: &str,
        new_value: &str,
        changed_by: i32,
        change_reason: Option<&str>,
    ) -> Result<(), AppError> {
        self.record_change(
            order_id,
            "UPDATE",
            Some(field_name),
            Some(old_value),
            Some(new_value),
            changed_by,
            change_reason,
            None,
            None,
        )
        .await
    }

    /// 记录状态变更
    pub async fn record_status_change(
        &self,
        order_id: i32,
        old_status: &str,
        new_status: &str,
        changed_by: i32,
        change_reason: Option<&str>,
    ) -> Result<(), AppError> {
        self.record_change(
            order_id,
            "STATUS_CHANGE",
            Some("status"),
            Some(old_status),
            Some(new_status),
            changed_by,
            change_reason,
            None,
            None,
        )
        .await
    }
}

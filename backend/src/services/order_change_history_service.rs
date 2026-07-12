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

/// 订单变更历史记录参数对象
///
/// 批次 332 v10 复审 P3 修复：引入参数对象消除 record_change 的 too_many_arguments 警告。
/// 聚合订单变更所需的全部字段，避免函数签名携带 9 个参数。
#[derive(Debug, Clone)]
pub struct OrderChangeRecord {
    /// 订单 ID
    pub order_id: i32,
    /// 变更类型（如 CREATE / UPDATE / STATUS_CHANGE）
    pub change_type: String,
    /// 变更字段名（可选，UPDATE 时填写）
    pub field_name: Option<String>,
    /// 旧值（可选）
    pub old_value: Option<String>,
    /// 新值（可选）
    pub new_value: Option<String>,
    /// 操作人 ID
    pub changed_by: i32,
    /// 变更原因（可选）
    pub change_reason: Option<String>,
    /// 操作 IP 地址（可选，审计用）
    pub ip_address: Option<String>,
    /// 操作 User-Agent（可选，审计用）
    pub user_agent: Option<String>,
}

impl OrderChangeHistoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 记录订单变更历史
    ///
    /// 批次 332 v10 复审 P3 修复：签名从 9 参数改为单一参数对象 `OrderChangeRecord`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn record_change(&self, record: OrderChangeRecord) -> Result<(), AppError> {
        let history = sales_order_change_history::ActiveModel {
            order_id: ActiveValue::Set(record.order_id),
            change_type: ActiveValue::Set(record.change_type),
            field_name: ActiveValue::Set(record.field_name),
            old_value: ActiveValue::Set(record.old_value),
            new_value: ActiveValue::Set(record.new_value),
            changed_by: ActiveValue::Set(record.changed_by),
            changed_at: ActiveValue::Set(Utc::now()),
            change_reason: ActiveValue::Set(record.change_reason),
            ip_address: ActiveValue::Set(record.ip_address),
            user_agent: ActiveValue::Set(record.user_agent),
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
        let offset = page.saturating_sub(1) * page_size;

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
        // 批次 332 v10 复审 P3 修复：使用 OrderChangeRecord 参数对象替代多参数
        let record = OrderChangeRecord {
            order_id,
            change_type: "CREATE".to_string(),
            field_name: None,
            old_value: None,
            new_value: None,
            changed_by,
            change_reason: Some("创建订单".to_string()),
            ip_address: None,
            user_agent: None,
        };
        self.record_change(record).await
    }


}

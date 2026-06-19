// model.rs - 数据模型
// 用途：与数据库表 notification_messages 映射
// 多租户：tenant_id 必填且不可变

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 数据库行结构（与 notification_messages 表对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRow {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub category: String,
    pub priority: i16,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

/// 新建通知参数
#[derive(Debug, Clone)]
pub struct NewNotification {
    pub tenant_id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub category: String,
    pub priority: i16,
}

impl NewNotification {
    /// 校验优先级范围 1-10
    pub fn validate(&self) -> Result<(), String> {
        if self.priority < 1 || self.priority > 10 {
            return Err(format!("优先级必须在 1-10 之间，当前值 {}", self.priority));
        }
        if self.title.trim().is_empty() {
            return Err("标题不能为空".to_string());
        }
        if self.content.trim().is_empty() {
            return Err("内容不能为空".to_string());
        }
        if self.tenant_id <= 0 {
            return Err("租户 ID 无效".to_string());
        }
        if self.user_id <= 0 {
            return Err("用户 ID 无效".to_string());
        }
        Ok(())
    }
}

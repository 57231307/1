//! 催收任务 DTO（V15 P0-B03 Batch 481 创建）

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 自动生成催收任务请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AutoGenerateTasksRequest {
    /// 逾期天数下限（默认 1）
    pub min_overdue_days: Option<i32>,
    /// 截止日期（默认今天）
    pub as_of_date: Option<NaiveDate>,
}

/// 手动创建催收任务
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateTaskRequest {
    pub customer_id: i64,
    pub ar_invoice_id: Option<i32>,
    pub overdue_amount: Decimal,
    pub overdue_days: i32,
    pub task_type: String,
    pub priority: Option<String>,
    pub due_date: NaiveDate,
    pub assigned_to: i32,
    pub remark: Option<String>,
}

/// 记录催收结果
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecordContactRequest {
    pub contact_result: String,
    pub next_action_date: Option<NaiveDate>,
    pub next_action_type: Option<String>,
    /// 是否标记完成
    pub mark_completed: Option<bool>,
    pub remark: Option<String>,
}

/// 重新分配
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReassignTaskRequest {
    pub assigned_to: i32,
    pub remark: Option<String>,
}

/// 取消任务
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CancelTaskRequest {
    pub cancel_reason: String,
}

/// 查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListTaskQuery {
    pub customer_id: Option<i64>,
    pub ar_invoice_id: Option<i32>,
    pub assigned_to: Option<i32>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub task_type: Option<String>,
    pub overdue_only: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

//! 财务预警 DTO（V15 P0-B04 Batch 481 创建）

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 手动触发预警扫描
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TriggerScanRequest {
    /// 预警类型过滤（None=全部 4 类）
    pub alert_type: Option<String>,
}

/// 创建预警
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateAlertRequest {
    pub alert_type: String,
    pub alert_level: String,
    pub title: String,
    pub content: String,
    pub target_module: Option<String>,
    pub target_id: Option<i64>,
    pub threshold_value: Option<Decimal>,
    pub actual_value: Option<Decimal>,
    pub value_unit: Option<String>,
}

/// 确认预警
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AcknowledgeAlertRequest {
    pub remark: Option<String>,
}

/// 解决预警
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResolveAlertRequest {
    pub resolve_note: String,
}

/// 查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListAlertQuery {
    pub alert_type: Option<String>,
    pub alert_level: Option<String>,
    pub status: Option<String>,
    pub target_module: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

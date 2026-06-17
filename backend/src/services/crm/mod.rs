//! CRM 服务模块（crm = customer relationship management）
//!
//! 由原 `services/crm_service.rs`（1469 行）按业务子领域拆分而来。
//! 子模块：
//! - `lead`    线索（潜在客户）管理
//! - `opp`     商机（opportunity）管理
//! - `cust`    客户管理（增强 CRUD、360 视图、跟进记录、RFM 分析）
//! - `pool`    公海（客户池）领取
//! - `assign`  分配（assignment）占位
//!
//! 兼容说明：原 `crate::services::crm::cust::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::crm::*;` 重新导出以保持向后兼容。

use sea_orm::FromQueryResult;
use serde::Serialize;

pub mod assign;
pub mod cust;
pub mod lead;
pub mod opp;
pub mod pool;

// =====================================================
// 关联数据结构
// =====================================================

/// 线索关联信息（合并展示客户 360 视图中的"线索来源"等）
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct LeadRelationInfo {
    pub id: i32,
    pub lead_no: String,
    pub lead_source: Option<String>,
    pub lead_status: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 商机简报（嵌入在客户 360 视图中）
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct OpportunityBrief {
    pub id: i32,
    pub opportunity_name: String,
    pub opportunity_stage: Option<String>,
    pub estimated_amount: Option<rust_decimal::Decimal>,
    pub actual_amount: Option<rust_decimal::Decimal>,
    pub expected_close_date: Option<chrono::NaiveDate>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 客户关联摘要
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct CustomerRelationSummary {
    pub customer_id: i32,
    pub total_leads: i64,
    pub total_opportunities: i64,
    pub total_orders: i64,
    pub total_order_amount: Option<rust_decimal::Decimal>,
    pub last_interaction_at: Option<chrono::DateTime<chrono::Utc>>,
    pub follow_up_count: i64,
}

// =====================================================
// 统一对外导出
// =====================================================

#[allow(unused_imports)] // TODO(tech-debt): 公共 API 重导出，业务接入后评估是否保留
pub use cust::CrmService;

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
//! 兼容说明：原 `crate::services::crm_service::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::crm::*;` 重新导出以保持向后兼容。

use sea_orm::FromQueryResult;
use serde::Serialize;
use std::sync::Arc;
use validator::Validate;

pub mod assign;
pub mod cust;
pub mod lead;
pub mod opp;
pub mod pool;

// =====================================================
// DTO 数据结构
// =====================================================

/// 线索关联信息（合并展示客户 360 视图中的"线索来源"等）
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct LeadRelationInfo {
    pub lead_id: i32,
    pub lead_name: String,
    pub lead_source: Option<String>,
    pub lead_status: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 商机简报（嵌入在客户 360 视图中）
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct OpportunityBrief {
    pub id: i32,
    pub name: String,
    pub stage: Option<String>,
    pub expected_amount: Option<rust_decimal::Decimal>,
    pub actual_amount: Option<rust_decimal::Decimal>,
    pub expected_close_date: Option<chrono::NaiveDate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
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

/// 创建线索请求
#[derive(Debug, Validate, serde::Deserialize)]
pub struct CreateLeadRequest {
    #[validate(length(min = 1, max = 100, message = "线索名称长度必须在1-100之间"))]
    pub name: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub company: Option<String>,
    pub source: Option<String>,
    pub industry: Option<String>,
    pub expected_amount: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    pub owner_id: Option<i32>,
}

/// 更新线索请求
#[derive(Debug, Default, serde::Deserialize)]
pub struct UpdateLeadRequest {
    pub name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub company: Option<String>,
    pub source: Option<String>,
    pub industry: Option<String>,
    pub expected_amount: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    pub owner_id: Option<i32>,
}

/// 线索状态更新请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateLeadStatusRequest {
    pub status: String,
    pub reason: Option<String>,
}

/// 创建商机请求
#[derive(Debug, Validate, serde::Deserialize)]
pub struct CreateOpportunityRequest {
    pub customer_id: Option<i32>,
    #[validate(length(min = 1, max = 200, message = "商机名称长度必须在1-200之间"))]
    pub name: String,
    pub amount: Option<rust_decimal::Decimal>,
    pub stage: Option<String>,
    pub probability: Option<rust_decimal::Decimal>,
    pub expected_close_date: Option<chrono::NaiveDate>,
    pub owner_id: Option<i32>,
    pub source: Option<String>,
    pub description: Option<String>,
    pub lead_id: Option<i32>,
}

/// 更新商机请求
#[derive(Debug, Default, serde::Deserialize)]
pub struct UpdateOpportunityRequest {
    pub name: Option<String>,
    pub amount: Option<rust_decimal::Decimal>,
    pub stage: Option<String>,
    pub probability: Option<rust_decimal::Decimal>,
    pub expected_close_date: Option<chrono::NaiveDate>,
    pub owner_id: Option<i32>,
    pub description: Option<String>,
}

/// 客户增强更新请求
#[derive(Debug, Default, serde::Deserialize)]
pub struct UpdateCustomerEnhancedRequest {
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub industry: Option<String>,
    pub level: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub owner_id: Option<i32>,
}

/// 创建跟进记录请求
#[derive(Debug, Validate, serde::Deserialize)]
pub struct CreateFollowUpRequest {
    pub customer_id: i32,
    pub follow_up_type: String,
    pub content: String,
    pub follow_up_at: chrono::DateTime<chrono::Utc>,
    pub next_follow_up_at: Option<chrono::DateTime<chrono::Utc>>,
    pub notes: Option<String>,
}

// =====================================================
// 统一对外导出
// =====================================================

pub use cust::CrmService;

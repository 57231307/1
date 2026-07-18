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
use crate::models::quality_issue;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 创建售后工单 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateAfterSalesDto {
    pub custom_order_id: i64,
    pub customer_id: i64,
    /// 售后类型：complaint / repair / exchange / refund
    pub issue_type: String,
    pub description: String,
    pub refund_amount: Option<Decimal>,
    /// V15 P0-B12：可选关联已有质量异常 ID
    /// 若不填，可后续调用 trigger_quality_investigation 方法自动创建质量异常并回填
    pub quality_issue_id: Option<i64>,
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
    /// 批次 263：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
    /// V15 P0-B12：售后工单已关联质量异常，禁止重复触发
    #[error("售后工单 {0} 已关联质量异常 {1}，禁止重复触发质量调查")]
    AlreadyLinked(i64, i64),
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
            quality_issue_id: Set(dto.quality_issue_id),
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
        dto: UpdateAfterSalesDto,
    ) -> Result<after_sales::Model, AfterSalesError> {
        let existing = Entity::find_by_id(id)
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

    /// V15 P0-B12：触发质量调查
    ///
    /// 根据售后工单信息自动创建一条 quality_issue 记录，并回填 quality_issue_id 到售后工单。
    /// 用于售后→质量改进闭环：客诉/维修/换货类售后工单可触发质量调查，避免同类问题重复发生。
    ///
    /// 业务规则：
    ///   1. 售后工单必须存在且未关闭（status != closed/rejected）
    ///   2. 售后工单不能已关联 quality_issue_id（禁止重复触发，避免产生冗余质量异常）
    ///   3. 自动创建的 quality_issue 字段映射：
    ///      - custom_order_id：从售后工单继承
    ///      - issue_type："after_sales_reported"（售后上报）
    ///      - severity：根据售后类型推断（complaint=high / repair=medium / exchange=low / refund=high）
    ///      - description：售后工单描述
    ///      - discovered_at：当前时间
    ///      - status："open"
    ///   4. 注：8D 流程（quality_8d_service）当前不存在，本方法仅创建 quality_issue 记录，
    ///      8D 触发部分待后续批次补齐
    ///
    /// 参数说明：
    /// - `after_sales_id`：售后工单 ID
    /// - `severity_override`：可选严重程度覆盖（high/medium/low），None 时按售后类型自动推断
    ///
    /// 返回：(更新后的售后工单, 新创建的质量异常)
    pub async fn trigger_quality_investigation(
        &self,
        after_sales_id: i64,
        severity_override: Option<String>,
    ) -> Result<(after_sales::Model, quality_issue::Model), AfterSalesError> {
        let existing = Entity::find_by_id(after_sales_id)
            .one(&*self.db)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        // 校验：已关闭/已拒绝的售后工单不允许触发质量调查
        if existing.status == "closed" || existing.status == "rejected" {
            return Err(AfterSalesError::Validation(format!(
                "售后工单状态为 {}，已关闭/拒绝的工单不允许触发质量调查",
                existing.status
            )));
        }

        // 校验：禁止重复触发（已关联 quality_issue_id 的工单不允许再次触发）
        if let Some(existing_qi_id) = existing.quality_issue_id {
            return Err(AfterSalesError::AlreadyLinked(after_sales_id, existing_qi_id));
        }

        // 严重程度推断：优先使用 severity_override，否则按售后类型自动推断
        let severity = severity_override.unwrap_or_else(|| {
            match existing.issue_type.as_str() {
                "complaint" | "refund" => "high".to_string(),
                "repair" => "medium".to_string(),
                "exchange" => "low".to_string(),
                _ => "medium".to_string(),
            }
        });

        let now = Utc::now();

        // 创建 quality_issue 记录
        let new_issue = quality_issue::ActiveModel {
            id: Default::default(),
            custom_order_id: Set(existing.custom_order_id),
            process_node_id: Set(None),
            issue_type: Set("after_sales_reported".to_string()),
            severity: Set(severity),
            description: Set(format!(
                "[售后工单 #{}] {}",
                after_sales_id, existing.description
            )),
            discovered_at: Set(now),
            resolved_at: Set(None),
            resolution: Set(None),
            status: Set("open".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let inserted_issue = new_issue.insert(&*self.db).await?;

        // 回填 quality_issue_id 到售后工单
        let mut active: ActiveModel = existing.into();
        active.quality_issue_id = Set(Some(inserted_issue.id));
        active.updated_at = Set(now);
        let updated_after_sales = active.update(&*self.db).await?;

        Ok((updated_after_sales, inserted_issue))
    }

    /// 列出订单的售后工单
    /// 按订单查询售后工单列表（分页）
    ///
    /// 批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
    /// paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
    /// 补 clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
    pub async fn list_by_order(
        &self,
        order_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<after_sales::Model>, u64), AfterSalesError> {
        let query = Entity::find()
            .filter(after_sales::Column::CustomOrderId.eq(order_id));

        let paginator = query
            .order_by_desc(after_sales::Column::OpenedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 按 ID 获取
    pub async fn get_by_id(
        &self,
        id: i64,
    ) -> Result<after_sales::Model, AfterSalesError> {
        Entity::find_by_id(id)
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

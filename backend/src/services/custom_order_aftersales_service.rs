//! 定制订单售后服务
//!
//! 4 种售后类型：客诉 / 维修 / 换货 / 退款
//! 状态机：opened → processing → resolved/closed/rejected
//! 创建时间: 2026-06-17

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::models::after_sales::{self, ActiveModel, Entity};
use crate::models::quality_issue;
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
    pub quality_issue_id: Option<i64>,
    /// V15 P1 batch-19 缺陷 23.3.3：原因分类（quality/logistics/customer_preference/other）
    pub reason_category: Option<String>,
    /// V15 P1 batch-19 缺陷 23.3.3：原因明细
    pub reason_detail: Option<String>,
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
        // V15 P2 23.3 缺陷1 修复：增加 return_goods（退货）类型。
        // 原因：审计计划 23.3 要求支持退货/换货/维修/投诉 4 类，原实现仅有
        // complaint/repair/exchange/refund，缺失"退货"独立类型；退货涉及物流收货、
        // 库存回库，与退款（财务出账）是不同业务。此处保留 refund 以兼容既有场景。
        if !["complaint", "repair", "exchange", "return_goods", "refund"].contains(&dto.issue_type.as_str()) {
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
            reason_category: Set(dto.reason_category),
            reason_detail: Set(dto.reason_detail),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
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
    /// 根据售后工单信息自动创建一条 quality_issue 记录，并回填 quality_issue_id 到售后工单。；用于售后→质量改进闭环：客诉/维修/换货类售后工单可触发质量调查，避免同类问题重复发生。；业务规则：1. 售后工单必须存在且未关闭（status != closed/rejected）；2. 售后工单不能已关联 quality_issue_id（禁止重复触发，避免产生冗余质量异常）；3. 自动创建的 quality_issue 字段映射：custom_order_id：从售后工单继承；issue_type："after_sales_reported"（售后上报）；severity：根据售后类型推断（complaint=high / repair=medium / exchange=low / refund=high）；description：售后工单描述；discovered_at：当前时间；status："open"；4. 注：8D 流程（quality_8d_service）当前不存在，本方法仅创建 quality_issue 记录，；8D 触发部分待后续批次补齐；参数说明：`after_sales_id`：售后工单 ID；`severity_override`：可选严重程度覆盖（high/medium/low），None 时按售后类型自动推断；返回：(更新后的售后工单, 新创建的质量异常)
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
            return Err(AfterSalesError::AlreadyLinked(
                after_sales_id,
                existing_qi_id,
            ));
        }

        // 严重程度推断：优先使用 severity_override，否则按售后类型自动推断
        let severity = severity_override.unwrap_or_else(|| match existing.issue_type.as_str() {
            "complaint" | "refund" => "high".to_string(),
            "repair" => "medium".to_string(),
            "exchange" => "low".to_string(),
            _ => "medium".to_string(),
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
            ..Default::default()
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
    /// 按订单查询售后工单列表（分页）；批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。；paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。；补 clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
    pub async fn list_by_order(
        &self,
        order_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<after_sales::Model>, u64), AfterSalesError> {
        let query = Entity::find().filter(after_sales::Column::CustomOrderId.eq(order_id));

        let paginator = query
            .order_by_desc(after_sales::Column::OpenedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// V15 P1 batch-19 缺陷 23.3.2：受理售后工单（opened → accepted）
    pub async fn accept_after_sales(&self, id: i64) -> Result<after_sales::Model, AfterSalesError> {
        let txn = self.db.begin().await?;
        let existing = Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        if existing.status != "opened" {
            return Err(AfterSalesError::InvalidState(format!(
                "当前状态 {} 不允许受理",
                existing.status
            )));
        }

        let mut active: ActiveModel = existing.into();
        active.status = Set("accepted".to_string());
        active.accepted_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// V15 P1 batch-19 缺陷 23.3.2：客户评价售后处理结果（resolved → evaluated）
    pub async fn evaluate_after_sales(
        &self,
        id: i64,
        score: i32,
        comment: Option<String>,
    ) -> Result<after_sales::Model, AfterSalesError> {
        if !(1..=5).contains(&score) {
            return Err(AfterSalesError::Validation(
                "评价分数必须在 1-5 之间".to_string(),
            ));
        }

        let txn = self.db.begin().await?;
        let existing = Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        if existing.status != "resolved" {
            return Err(AfterSalesError::InvalidState(format!(
                "当前状态 {} 不允许评价",
                existing.status
            )));
        }

        let mut active: ActiveModel = existing.into();
        active.status = Set("evaluated".to_string());
        active.evaluation_score = Set(Some(score));
        active.evaluation_comment = Set(comment);
        active.evaluated_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// V15 P1 batch-19 缺陷 23.3.3：生成售后原因 TOP5 月报
    pub async fn monthly_top5_report(
        &self,
        year: i32,
        month: u32,
    ) -> Result<MonthlyTop5Report, AfterSalesError> {
        use sea_orm::{FromQueryResult, Statement};

        #[derive(Debug, FromQueryResult)]
        struct Row {
            reason_category: String,
            reason_detail: Option<String>,
            count: i64,
        }

        let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or(AfterSalesError::Validation("无效的年月".to_string()))?;
        let next_month = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or(AfterSalesError::Validation("无效的年月".to_string()))?;

        let sql = r#"
            SELECT reason_category, reason_detail, COUNT(*) as count
            FROM after_sales
            WHERE created_at >= $1 AND created_at < $2
            AND reason_category IS NOT NULL
            GROUP BY reason_category, reason_detail
            ORDER BY count DESC
            LIMIT 5
        "#;

        let rows = Row::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            sql,
            vec![
                start_date.and_hms_opt(0, 0, 0).unwrap().into(),
                next_month.and_hms_opt(0, 0, 0).unwrap().into(),
            ],
        ))
        .all(&*self.db)
        .await?;

        let items: Vec<Top5ReasonItem> = rows
            .into_iter()
            .map(|r| Top5ReasonItem {
                reason_category: r.reason_category,
                reason_detail: r.reason_detail,
                count: r.count,
            })
            .collect();

        Ok(MonthlyTop5Report { year, month, items })
    }
}

/// V15 P1 batch-19 缺陷 23.3.3：售后 TOP5 原因月报 DTO
#[derive(Debug, Serialize)]
pub struct MonthlyTop5Report {
    pub year: i32,
    pub month: u32,
    pub items: Vec<Top5ReasonItem>,
}

/// V15 P1 batch-19 缺陷 23.3.3：TOP5 原因项
#[derive(Debug, Serialize)]
pub struct Top5ReasonItem {
    pub reason_category: String,
    pub reason_detail: Option<String>,
    pub count: i64,
}

/// 状态转换校验（V15 P1 batch-19 缺陷 23.3.2：补齐 accepted/evaluated 步骤）
fn is_valid_transition(from: &str, to: &str) -> bool {
    use std::collections::HashMap;
    let mut valid: HashMap<&str, Vec<&str>> = HashMap::new();
    valid.insert("opened", vec!["accepted", "rejected", "closed"]);
    valid.insert("accepted", vec!["processing", "rejected", "closed"]);
    valid.insert("processing", vec!["resolved", "closed", "rejected"]);
    valid.insert("resolved", vec!["evaluated", "closed"]);
    valid.insert("evaluated", vec!["closed"]);
    valid.insert("closed", vec![]);
    valid.insert("rejected", vec![]);

    valid.get(from).map(|v| v.contains(&to)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transition() {
        // V15 P1 batch-19：opened → accepted → processing → resolved → evaluated → closed
        assert!(is_valid_transition("opened", "accepted"));
        assert!(is_valid_transition("accepted", "processing"));
        assert!(is_valid_transition("processing", "resolved"));
        assert!(is_valid_transition("resolved", "evaluated"));
        assert!(is_valid_transition("evaluated", "closed"));
        assert!(!is_valid_transition("closed", "processing"));
        // opened 不能直接跳到 processing（需先 accepted）
        assert!(!is_valid_transition("opened", "processing"));
        // opened 不能直接跳到 resolved
        assert!(!is_valid_transition("opened", "resolved"));
    }
}

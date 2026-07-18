//! 财务预警服务（V15 P0-B04 Batch 481 创建）
//!
//! 4 类预警：
//! - ar_overdue：应收超额（扫描逾期未收 ar_invoice）
//! - inventory_backlog：库存积压（扫描超过 max_stock_point 的库存）
//! - cash_flow_shortage：现金流不足（扫描余额低于阈值的资金账户）
//! - budget_overrun：预算超支（扫描大额预算执行记录）
//!
//! 3 级预警：info / warning / critical
//! 状态机：active → acknowledged → resolved / expired
//!
//! 关联任务：P0-B04（§17.5-D1）
//! 关联文件：models/finance_alert.rs / models/finance_alert_dto.rs /
//!          handlers/finance_alert_handler.rs / routes/finance_alert.rs

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::ar_invoice;
use crate::models::budget_execution;
use crate::models::finance_alert::{self, ActiveModel, Entity};
use crate::models::finance_alert_dto::{
    AcknowledgeAlertRequest, CreateAlertRequest, ListAlertQuery, ResolveAlertRequest,
    TriggerScanRequest,
};
use crate::models::fund_account;
use crate::models::inventory_stock;
use crate::models::notification::{NotificationPriority, NotificationType};
use crate::services::notification_service::{
    CreateNotificationRequest, NotificationService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 业务错误
#[derive(Debug, Error)]
pub enum FinanceAlertError {
    #[error("财务预警不存在")]
    NotFound,
    #[error("当前状态 {current} 不允许此操作（期望 {expected}）")]
    InvalidState {
        current: String,
        expected: &'static str,
    },
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// paginate_with_total 返回 AppError，透传所需
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 预警类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    ArOverdue,
    InventoryBacklog,
    CashFlowShortage,
    BudgetOverrun,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArOverdue => "ar_overdue",
            Self::InventoryBacklog => "inventory_backlog",
            Self::CashFlowShortage => "cash_flow_shortage",
            Self::BudgetOverrun => "budget_overrun",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ar_overdue" => Some(Self::ArOverdue),
            "inventory_backlog" => Some(Self::InventoryBacklog),
            "cash_flow_shortage" => Some(Self::CashFlowShortage),
            "budget_overrun" => Some(Self::BudgetOverrun),
            _ => None,
        }
    }
}

/// 预警状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Expired,
}

impl AlertStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
            Self::Expired => "expired",
        }
    }
}

/// 预警级别辅助：根据 ar_overdue 逾期天数评估
fn ar_overdue_level(overdue_days: i64) -> &'static str {
    if overdue_days > 90 {
        "critical"
    } else if overdue_days > 30 {
        "warning"
    } else {
        "info"
    }
}

/// 预警级别映射到通知优先级
fn alert_level_to_priority(level: &str) -> NotificationPriority {
    match level {
        "critical" => NotificationPriority::Urgent,
        "warning" => NotificationPriority::High,
        _ => NotificationPriority::Normal,
    }
}

/// 默认阈值
const CASH_FLOW_MIN_THRESHOLD: Decimal = Decimal::ZERO;
// rust_decimal 1.x 中 Decimal::new 非 const fn，改用函数返回运行期构造的值
fn budget_overrun_amount_threshold() -> Decimal {
    Decimal::new(100_000, 2)
}

/// 财务预警服务
pub struct FinanceAlertService {
    db: Arc<DatabaseConnection>,
}

impl FinanceAlertService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 触发预警扫描
    ///
    /// 按 alert_type 过滤扫描（None=全部 4 类），生成 active 状态预警
    /// 已存在同类型同 target 的 active 预警则跳过（幂等）
    pub async fn trigger_scan(
        &self,
        req: TriggerScanRequest,
        triggered_by: Option<i32>,
    ) -> Result<Vec<finance_alert::Model>, FinanceAlertError> {
        let scan_types: Vec<AlertType> = match req.alert_type.as_deref() {
            Some(t) => vec![AlertType::from_str(t)
                .ok_or_else(|| FinanceAlertError::Validation(format!("非法 alert_type: {}", t)))?],
            None => vec![
                AlertType::ArOverdue,
                AlertType::InventoryBacklog,
                AlertType::CashFlowShortage,
                AlertType::BudgetOverrun,
            ],
        };

        let txn = (*self.db).begin().await?;
        let now = Utc::now();
        let today = now.date_naive();
        let mut created: Vec<finance_alert::Model> = Vec::new();

        for scan_type in scan_types {
            let candidates = self.scan_candidates(scan_type, &txn).await?;
            for cand in candidates {
                // 幂等：同类型同 target 的 active 预警已存在则跳过
                if let (Some(module), Some(target_id)) = (cand.target_module, cand.target_id) {
                    let existing = Entity::find()
                        .filter(finance_alert::Column::AlertType.eq(cand.alert_type.as_str()))
                        .filter(finance_alert::Column::TargetModule.eq(module))
                        .filter(finance_alert::Column::TargetId.eq(target_id))
                        .filter(
                            finance_alert::Column::Status
                                .is_in([AlertStatus::Active.as_str(), AlertStatus::Acknowledged.as_str()]),
                        )
                        .one(&txn)
                        .await?;
                    if existing.is_some() {
                        continue;
                    }
                }

                // 创建预警记录
                let alert_no = format!(
                    "FA-{}-{:04}",
                    today.format("%Y%m%d"),
                    created.len() + 1
                );
                let active = ActiveModel {
                    id: Default::default(),
                    alert_no: Set(alert_no),
                    alert_type: Set(cand.alert_type.as_str().to_string()),
                    alert_level: Set(cand.alert_level.to_string()),
                    title: Set(cand.title),
                    content: Set(cand.content),
                    target_module: Set(cand.target_module.map(String::from)),
                    target_id: Set(cand.target_id),
                    threshold_value: Set(cand.threshold_value),
                    actual_value: Set(cand.actual_value),
                    value_unit: Set(cand.value_unit.map(String::from)),
                    triggered_at: Set(now),
                    triggered_by: Set(triggered_by),
                    status: Set(AlertStatus::Active.as_str().to_string()),
                    acknowledged_at: Set(None),
                    acknowledged_by: Set(None),
                    resolved_at: Set(None),
                    resolved_by: Set(None),
                    resolve_note: Set(None),
                    expired_at: Set(None),
                    notification_id: Set(None),
                    remark: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                };
                let model = active.insert(&txn).await?;

                // 复用 NotificationService 创建通知（如有触发人）
                if let Some(user_id) = triggered_by {
                    let notif_req = CreateNotificationRequest {
                        user_id,
                        notification_type: NotificationType::System,
                        title: format!("[财务预警] {}", model.title),
                        content: model.content.clone(),
                        priority: alert_level_to_priority(&model.alert_level),
                        business_type: Some("finance_alert".to_string()),
                        business_id: Some(model.id as i32),
                        action_url: Some(format!("/finance/alerts/{}", model.id)),
                        sender_id: None,
                        sender_name: Some("系统财务预警".to_string()),
                    };
                    let notif_service = NotificationService::new(self.db.clone());
                    if let Ok(notif) = notif_service.create_notification(notif_req).await {
                        // 回填 notification_id
                        let mut active: ActiveModel = model.into();
                        active.notification_id = Set(Some(notif.id));
                        active.updated_at = Set(now);
                        let updated = active.update(&txn).await?;
                        created.push(updated);
                    } else {
                        created.push(model);
                    }
                } else {
                    created.push(model);
                }
            }
        }

        txn.commit().await?;
        Ok(created)
    }

    /// 扫描指定类型的候选预警
    async fn scan_candidates(
        &self,
        scan_type: AlertType,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<AlertCandidate>, FinanceAlertError> {
        match scan_type {
            AlertType::ArOverdue => self.scan_ar_overdue(txn).await,
            AlertType::InventoryBacklog => self.scan_inventory_backlog(txn).await,
            AlertType::CashFlowShortage => self.scan_cash_flow_shortage(txn).await,
            AlertType::BudgetOverrun => self.scan_budget_overrun(txn).await,
        }
    }

    /// 扫描应收超额（按客户聚合逾期未收 ar_invoice）
    async fn scan_ar_overdue(
        &self,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<AlertCandidate>, FinanceAlertError> {
        let today = Utc::now().date_naive();
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(ar_invoice::Column::DueDate.lt(today))
            .filter(ar_invoice::Column::ApprovalStatus.eq("approved"))
            .all(txn)
            .await?;

        // 按客户聚合
        let mut customer_map: std::collections::HashMap<i64, (Decimal, i64, Option<String>)> =
            std::collections::HashMap::new();
        for inv in invoices {
            let overdue_days = (today - inv.due_date).num_days();
            let customer_id = inv.customer_id as i64;
            let entry = customer_map
                .entry(customer_id)
                .or_insert((Decimal::ZERO, 0, inv.customer_name.clone()));
            entry.0 += inv.unpaid_amount;
            if overdue_days > entry.1 {
                entry.1 = overdue_days;
            }
        }

        let mut candidates = Vec::new();
        for (customer_id, (total_unpaid, max_days, customer_name)) in customer_map {
            let level = ar_overdue_level(max_days);
            let name = customer_name.unwrap_or_else(|| format!("客户#{}", customer_id));
            candidates.push(AlertCandidate {
                alert_type: AlertType::ArOverdue,
                alert_level: level.to_string(),
                title: format!("应收逾期预警：{} 逾期 {} 天", name, max_days),
                content: format!(
                    "客户 {} 应收逾期 {} 天，未收金额 {} 元。请尽快安排催收。",
                    name, max_days, total_unpaid
                ),
                target_module: Some("customer"),
                target_id: Some(customer_id),
                threshold_value: Some(Decimal::ZERO),
                actual_value: Some(total_unpaid),
                value_unit: Some("CNY"),
            });
        }
        Ok(candidates)
    }

    /// 扫描库存积压（quantity_on_hand > max_stock_point 且 max_stock_point > 0）
    async fn scan_inventory_backlog(
        &self,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<AlertCandidate>, FinanceAlertError> {
        // sea_orm gt 不支持列对列比较，先取 max_stock_point > 0 的记录再在 Rust 中过滤
        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::MaxStockPoint.gt(Decimal::ZERO))
            .all(txn)
            .await?;

        let mut candidates = Vec::new();
        for s in stocks {
            if s.quantity_on_hand <= s.max_stock_point {
                continue;
            }
            let over_ratio = if s.max_stock_point > Decimal::ZERO {
                s.quantity_on_hand / s.max_stock_point
            } else {
                Decimal::ZERO
            };
            let level = if over_ratio > Decimal::from(2) {
                "critical"
            } else {
                "warning"
            };
            candidates.push(AlertCandidate {
                alert_type: AlertType::InventoryBacklog,
                alert_level: level.to_string(),
                title: format!("库存积压预警：产品#{} 批次 {}", s.product_id, s.batch_no),
                content: format!(
                    "产品#{} 批次 {} 库存 {} 超过上限 {}（倍率 {:.2}）",
                    s.product_id, s.batch_no, s.quantity_on_hand, s.max_stock_point, over_ratio
                ),
                target_module: Some("inventory_stock"),
                target_id: Some(s.id as i64),
                threshold_value: Some(s.max_stock_point),
                actual_value: Some(s.quantity_on_hand),
                value_unit: Some("unit"),
            });
        }
        Ok(candidates)
    }

    /// 扫描现金流不足（资金账户余额 < 0 或低于阈值）
    async fn scan_cash_flow_shortage(
        &self,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<AlertCandidate>, FinanceAlertError> {
        let accounts = fund_account::Entity::find()
            .filter(fund_account::Column::Balance.lt(CASH_FLOW_MIN_THRESHOLD))
            .all(txn)
            .await?;

        let mut candidates = Vec::new();
        for a in accounts {
            let level = if a.balance < Decimal::from(-100_000) {
                "critical"
            } else if a.balance < Decimal::ZERO {
                "warning"
            } else {
                "info"
            };
            candidates.push(AlertCandidate {
                alert_type: AlertType::CashFlowShortage,
                alert_level: level.to_string(),
                title: format!("现金流不足预警：账户 {}", a.account_name),
                content: format!(
                    "资金账户 {}（{}）余额为 {}，低于安全阈值 {}",
                    a.account_name, a.account_no, a.balance, CASH_FLOW_MIN_THRESHOLD
                ),
                target_module: Some("fund_account"),
                target_id: Some(a.id as i64),
                threshold_value: Some(CASH_FLOW_MIN_THRESHOLD),
                actual_value: Some(a.balance),
                value_unit: Some("CNY"),
            });
        }
        Ok(candidates)
    }

    /// 扫描预算超支（execution_type='使用' 且 amount > 阈值）
    async fn scan_budget_overrun(
        &self,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<AlertCandidate>, FinanceAlertError> {
        let executions = budget_execution::Entity::find()
            .filter(budget_execution::Column::ExecutionType.eq("使用"))
            .filter(
                budget_execution::Column::Amount.gt(budget_overrun_amount_threshold()),
            )
            .all(txn)
            .await?;

        let mut candidates = Vec::new();
        for e in executions {
            let level = if e.amount > Decimal::from_i128_with_scale(500_000, 2) {
                "critical"
            } else {
                "warning"
            };
            let expense = e.expense_type.unwrap_or_else(|| "未分类".to_string());
            candidates.push(AlertCandidate {
                alert_type: AlertType::BudgetOverrun,
                alert_level: level.to_string(),
                title: format!("预算超支预警：{} 大额支出", expense),
                content: format!(
                    "费用类型 {} 发生大额支出 {} 元（执行明细#{}），请核查预算执行情况",
                    expense, e.amount, e.id
                ),
                target_module: Some("budget_execution"),
                target_id: Some(e.id as i64),
                threshold_value: Some(budget_overrun_amount_threshold()),
                actual_value: Some(e.amount),
                value_unit: Some("CNY"),
            });
        }
        Ok(candidates)
    }

    /// 手动创建预警
    pub async fn create_alert(
        &self,
        req: CreateAlertRequest,
    ) -> Result<finance_alert::Model, FinanceAlertError> {
        // 校验 alert_type
        if AlertType::from_str(&req.alert_type).is_none() {
            return Err(FinanceAlertError::Validation(format!(
                "非法 alert_type: {}，合法值：ar_overdue/inventory_backlog/cash_flow_shortage/budget_overrun",
                req.alert_type
            )));
        }
        // 校验 alert_level
        if !["info", "warning", "critical"].contains(&req.alert_level.as_str()) {
            return Err(FinanceAlertError::Validation(format!(
                "非法 alert_level: {}，合法值：info/warning/critical",
                req.alert_level
            )));
        }
        if req.title.trim().is_empty() {
            return Err(FinanceAlertError::Validation("title 不能为空".to_string()));
        }
        if req.content.trim().is_empty() {
            return Err(FinanceAlertError::Validation(
                "content 不能为空".to_string(),
            ));
        }

        let now = Utc::now();
        let today = now.date_naive();
        // 生成 alert_no：FA-YYYYMMDD-NNN（基于当日已有数 + 1）
        let prefix = format!("FA-{}-", today.format("%Y%m%d"));
        let count_today = Entity::find()
            .filter(finance_alert::Column::AlertNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;
        let alert_no = format!("FA-{}-{:04}", today.format("%Y%m%d"), count_today + 1);

        let active = ActiveModel {
            id: Default::default(),
            alert_no: Set(alert_no),
            alert_type: Set(req.alert_type),
            alert_level: Set(req.alert_level),
            title: Set(req.title),
            content: Set(req.content),
            target_module: Set(req.target_module),
            target_id: Set(req.target_id),
            threshold_value: Set(req.threshold_value),
            actual_value: Set(req.actual_value),
            value_unit: Set(req.value_unit),
            triggered_at: Set(now),
            triggered_by: Set(None),
            status: Set(AlertStatus::Active.as_str().to_string()),
            acknowledged_at: Set(None),
            acknowledged_by: Set(None),
            resolved_at: Set(None),
            resolved_by: Set(None),
            resolve_note: Set(None),
            expired_at: Set(None),
            notification_id: Set(None),
            remark: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// 确认预警（active → acknowledged）
    pub async fn acknowledge(
        &self,
        alert_id: i64,
        user_id: i32,
        _req: AcknowledgeAlertRequest,
    ) -> Result<finance_alert::Model, FinanceAlertError> {
        let txn = (*self.db).begin().await?;
        let existing = Entity::find_by_id(alert_id)
            .one(&txn)
            .await?
            .ok_or(FinanceAlertError::NotFound)?;

        if existing.status != AlertStatus::Active.as_str() {
            return Err(FinanceAlertError::InvalidState {
                current: existing.status,
                expected: "active",
            });
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        active.status = Set(AlertStatus::Acknowledged.as_str().to_string());
        active.acknowledged_at = Set(Some(now));
        active.acknowledged_by = Set(Some(user_id));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 解决预警（acknowledged → resolved，终态）
    pub async fn resolve(
        &self,
        alert_id: i64,
        user_id: i32,
        req: ResolveAlertRequest,
    ) -> Result<finance_alert::Model, FinanceAlertError> {
        if req.resolve_note.trim().is_empty() {
            return Err(FinanceAlertError::Validation(
                "resolve_note 不能为空".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;
        let existing = Entity::find_by_id(alert_id)
            .one(&txn)
            .await?
            .ok_or(FinanceAlertError::NotFound)?;

        if existing.status != AlertStatus::Acknowledged.as_str() {
            return Err(FinanceAlertError::InvalidState {
                current: existing.status,
                expected: "acknowledged",
            });
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        active.status = Set(AlertStatus::Resolved.as_str().to_string());
        active.resolved_at = Set(Some(now));
        active.resolved_by = Set(Some(user_id));
        active.resolve_note = Set(Some(req.resolve_note));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_alert(
        &self,
        alert_id: i64,
    ) -> Result<finance_alert::Model, FinanceAlertError> {
        Entity::find_by_id(alert_id)
            .one(&*self.db)
            .await?
            .ok_or(FinanceAlertError::NotFound)
    }

    /// 列表查询
    pub async fn list_alerts(
        &self,
        query: ListAlertQuery,
    ) -> Result<(Vec<finance_alert::Model>, u64), FinanceAlertError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut select = Entity::find();
        if let Some(v) = query.alert_type {
            if AlertType::from_str(&v).is_none() {
                return Err(FinanceAlertError::Validation(format!(
                    "非法 alert_type: {}",
                    v
                )));
            }
            select = select.filter(finance_alert::Column::AlertType.eq(v));
        }
        if let Some(v) = query.alert_level {
            if !["info", "warning", "critical"].contains(&v.as_str()) {
                return Err(FinanceAlertError::Validation(format!(
                    "非法 alert_level: {}",
                    v
                )));
            }
            select = select.filter(finance_alert::Column::AlertLevel.eq(v));
        }
        if let Some(v) = query.status {
            if !["active", "acknowledged", "resolved", "expired"].contains(&v.as_str()) {
                return Err(FinanceAlertError::Validation(format!(
                    "非法 status: {}",
                    v
                )));
            }
            select = select.filter(finance_alert::Column::Status.eq(v));
        }
        if let Some(v) = query.target_module {
            select = select.filter(finance_alert::Column::TargetModule.eq(v));
        }

        let paginator = select
            .order_by_desc(finance_alert::Column::TriggeredAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }
}

/// 预警候选（内部辅助结构，扫描结果）
struct AlertCandidate {
    alert_type: AlertType,
    alert_level: String,
    title: String,
    content: String,
    target_module: Option<&'static str>,
    target_id: Option<i64>,
    threshold_value: Option<Decimal>,
    actual_value: Option<Decimal>,
    value_unit: Option<&'static str>,
}

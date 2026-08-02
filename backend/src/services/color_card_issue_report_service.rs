//! 色卡发放报表服务
//! V15 P2 类九 10.3-3：5 类报表（发放明细/发放汇总/客户色卡台账/过期未使用/订单关联）
//!
//! 报表口径：全部基于 color_card_issues 表真实数据聚合，联查色卡/客户名称；
//! 支持 .xlsx 导出（规则 3，由 handler 层调用 build_xlsx_response）。

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_issue::{self, Entity as IssueEntity};
use crate::models::customer::{self, Entity as CustomerEntity};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::*;
use serde_json::{json, Value};
use std::sync::Arc;

/// 报表查询参数
#[derive(Debug, Clone, Default)]
pub struct ReportParams {
    pub customer_id: Option<i32>,
    pub color_card_id: Option<i32>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 色卡发放报表服务
pub struct ColorCardIssueReportService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardIssueReportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 构建发放记录基础查询条件（is_deleted=false + 可选过滤）
    fn base_cond(params: &ReportParams) -> Condition {
        let mut cond = Condition::all().add(color_card_issue::Column::IsDeleted.eq(false));
        if let Some(customer_id) = params.customer_id {
            cond = cond.add(color_card_issue::Column::CustomerId.eq(customer_id as i64));
        }
        if let Some(color_card_id) = params.color_card_id {
            cond = cond.add(color_card_issue::Column::ColorCardId.eq(color_card_id as i64));
        }
        if let Some(start) = params.start_date {
            let start_dt = start.and_hms_opt(0, 0, 0);
            if let Some(sdt) = start_dt {
                let start_utc =
                    chrono::DateTime::<Utc>::from_naive_utc_and_offset(sdt, chrono::Utc);
                cond = cond.add(color_card_issue::Column::IssuedAt.gte(start_utc));
            }
        }
        if let Some(end) = params.end_date {
            let end_dt = end.and_hms_opt(23, 59, 59);
            if let Some(edt) = end_dt {
                let end_utc = chrono::DateTime::<Utc>::from_naive_utc_and_offset(edt, chrono::Utc);
                cond = cond.add(color_card_issue::Column::IssuedAt.lte(end_utc));
            }
        }
        cond
    }

    /// 联查色卡与客户名称，组装报表行
    async fn build_rows(
        &self,
        issues: Vec<color_card_issue::Model>,
    ) -> Result<Vec<Value>, AppError> {
        let card_ids: Vec<i64> = issues.iter().map(|i| i.color_card_id).collect();
        let customer_ids: Vec<i32> = issues.iter().map(|i| i.customer_id as i32).collect();

        let cards = if card_ids.is_empty() {
            Vec::new()
        } else {
            ColorCardEntity::find()
                .filter(color_card::Column::Id.is_in(card_ids))
                .all(&*self.db)
                .await?
        };
        let customers = if customer_ids.is_empty() {
            Vec::new()
        } else {
            CustomerEntity::find()
                .filter(customer::Column::Id.is_in(customer_ids))
                .all(&*self.db)
                .await?
        };
        let card_map: std::collections::HashMap<i64, color_card::Model> =
            cards.into_iter().map(|c| (c.id, c)).collect();
        let customer_map: std::collections::HashMap<i32, customer::Model> =
            customers.into_iter().map(|c| (c.id, c)).collect();

        Ok(issues
            .into_iter()
            .map(|i| {
                let card = card_map.get(&i.color_card_id);
                let cust = customer_map.get(&(i.customer_id as i32));
                json!({
                    "issue_id": i.id,
                    "color_card_id": i.color_card_id,
                    "card_no": card.map(|c| c.card_no.clone()).unwrap_or_default(),
                    "card_name": card.map(|c| c.card_name.clone()).unwrap_or_default(),
                    "customer_id": i.customer_id,
                    "customer_name": cust.map(|c| c.customer_name.clone()).unwrap_or_default(),
                    "issue_qty": i.issue_qty,
                    "issued_at": i.issued_at,
                    "expected_return_date": i.expected_return_date,
                    "actual_return_date": i.actual_return_date,
                    "status": i.status,
                    "purpose": i.purpose,
                    "remark": i.remark,
                    "compensation_amount": i.compensation_amount,
                    "dye_lot_no": i.dye_lot_no,
                    "sales_order_id": i.sales_order_id,
                })
            })
            .collect())
    }

    /// 发放明细报表（按客户/色卡/时间过滤，发放时间倒序）
    pub async fn issue_detail_report(&self, params: ReportParams) -> Result<Vec<Value>, AppError> {
        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 200);

        let paginator = IssueEntity::find()
            .filter(Self::base_cond(&params))
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .paginate(&*self.db, page_size);
        let issues = paginator.fetch_page(page.saturating_sub(1)).await?;
        self.build_rows(issues).await
    }

    /// 发放汇总报表（按客户 + 色卡 + 状态维度聚合）
    pub async fn issue_summary_report(&self, params: ReportParams) -> Result<Vec<Value>, AppError> {
        // 汇总取全量（聚合粒度不依赖分页，故忽略 page/page_size）
        let issues = IssueEntity::find()
            .filter(Self::base_cond(&params))
            .all(&*self.db)
            .await?;

        let mut summary: std::collections::HashMap<(i64, i64, String), (i64, i64)> =
            std::collections::HashMap::new();
        for issue in issues {
            let key = (issue.customer_id, issue.color_card_id, issue.status.clone());
            let entry = summary.entry(key).or_insert((0, 0));
            entry.0 += 1; // 发放次数
            entry.1 += issue.issue_qty as i64; // 发放总数
        }

        let mut result = Vec::new();
        for ((customer_id, color_card_id, status), (issue_count, total_qty)) in summary {
            let card = ColorCardEntity::find_by_id(color_card_id)
                .one(&*self.db)
                .await?;
            let cust = CustomerEntity::find_by_id(customer_id as i32)
                .one(&*self.db)
                .await?;
            result.push(json!({
                "customer_id": customer_id,
                "customer_name": cust.map(|c| c.customer_name).unwrap_or_default(),
                "color_card_id": color_card_id,
                "card_no": card.as_ref().map(|c| c.card_no.clone()).unwrap_or_default(),
                "card_name": card.map(|c| c.card_name).unwrap_or_default(),
                "status": status,
                "issue_count": issue_count,
                "total_issue_qty": total_qty,
            }));
        }
        result.sort_by(|a, b| a["customer_id"].as_i64().cmp(&b["customer_id"].as_i64()));
        Ok(result)
    }

    /// 客户色卡台账（某客户全部发放记录，按发放时间倒序）
    pub async fn customer_color_card_ledger(
        &self,
        customer_id: i32,
    ) -> Result<Vec<Value>, AppError> {
        let issues = IssueEntity::find()
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::CustomerId.eq(customer_id as i64))
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .all(&*self.db)
            .await?;
        self.build_rows(issues).await
    }

    /// 过期未使用色卡报表（status='issued' 且 expected_return_date < today）
    pub async fn expired_unused_report(&self) -> Result<Vec<Value>, AppError> {
        let today = Utc::now().date_naive();
        let issues = IssueEntity::find()
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::Status.eq("issued"))
            .filter(color_card_issue::Column::ExpectedReturnDate.is_not_null())
            .filter(color_card_issue::Column::ExpectedReturnDate.lt(today))
            .order_by_asc(color_card_issue::Column::ExpectedReturnDate)
            .all(&*self.db)
            .await?;
        self.build_rows(issues).await
    }

    /// 订单关联报表（按销售订单 ID 过滤发放记录）
    pub async fn order_related_report(&self, sales_order_id: i32) -> Result<Vec<Value>, AppError> {
        let issues = IssueEntity::find()
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::SalesOrderId.eq(sales_order_id as i64))
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .all(&*self.db)
            .await?;
        self.build_rows(issues).await
    }
}

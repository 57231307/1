//! Incoterms 贸易术语服务
//!
//! V15 P1 batch-19 缺陷 23.5.2/23.5.4：
//! - 缺陷 23.5.2：术语与价格构成集成（按 Incoterm 自动计算运费/保费/关税）
//! - 缺陷 23.5.4：术语使用月报（按术语统计出口量/金额）

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait, FromQueryResult, Statement, Value};
use serde::Serialize;
use std::sync::Arc;

use crate::container::AppState;
use crate::models::sales_quotation::Entity as QuotationEntity;
use crate::utils::data_scope::{DataScope, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::incoterms::{CostBearer, Incoterms2020, Party};

/// V15 P1 batch-19 缺陷 23.5.2：价格构成 DTO
#[derive(Debug, Serialize)]
pub struct PriceComposition {
    pub incoterm: String,
    pub product_cost: Decimal,
    pub freight_cost: Option<Decimal>,
    pub insurance_cost: Option<Decimal>,
    pub duty_cost: Option<Decimal>,
    pub total_amount: Decimal,
    /// V15 P2 23.5 缺陷3：风险转移点（结构化）
    pub risk_transfer_point: &'static str,
    /// V15 P2 23.5 缺陷3：主费用承担方（卖方/买方/双方共担）
    pub cost_bearer: &'static str,
    /// V15 P2 23.5 缺陷3：出口清关责任方
    pub export_clearance_party: &'static str,
    /// V15 P2 23.5 缺陷3：进口清关责任方
    pub import_clearance_party: &'static str,
    /// V15 P2 23.5 缺陷3：适用运输方式（海运/内河 或 任意运输方式）
    pub transport_mode: &'static str,
}

/// V15 P1 batch-19 缺陷 23.5.4：术语使用月报 DTO
#[derive(Debug, Serialize)]
pub struct IncotermsMonthlyReport {
    pub year: i32,
    pub month: u32,
    pub items: Vec<IncotermUsageItem>,
}

/// V15 P1 batch-19 缺陷 23.5.4：术语使用统计项
#[derive(Debug, Serialize, FromQueryResult)]
pub struct IncotermUsageItem {
    pub incoterm: String,
    pub count: i64,
    pub total_amount: Decimal,
    pub freight_cost: Decimal,
    pub insurance_cost: Decimal,
    pub duty_cost: Decimal,
}

/// Incoterms 服务
pub struct IncotermsService {
    db: Arc<DatabaseConnection>,
}

impl IncotermsService {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// V15 P1 batch-19 缺陷 23.5.2：获取报价单价格构成（按 Incoterm 解析）
    pub async fn get_price_composition(
        &self,
        quotation_id: i64,
    ) -> Result<PriceComposition, AppError> {
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        let incoterm =
            Incoterms2020::from_code(&quotation.price_terms).map_err(AppError::business)?;

        Ok(PriceComposition {
            incoterm: incoterm.code().to_string(),
            product_cost: quotation.subtotal - quotation.tax_amount,
            freight_cost: quotation.freight_cost,
            insurance_cost: quotation.insurance_cost,
            duty_cost: quotation.duty_cost,
            total_amount: quotation.total_amount,
            risk_transfer_point: incoterm.risk_transfer_point(),
            cost_bearer: match incoterm.cost_bearer() {
                CostBearer::Seller => "卖方",
                CostBearer::Buyer => "买方",
                CostBearer::Both => "双方共担",
            },
            export_clearance_party: match incoterm.export_clearance_party() {
                Party::Seller => "卖方",
                Party::Buyer => "买方",
            },
            import_clearance_party: match incoterm.import_clearance_party() {
                Party::Seller => "卖方",
                Party::Buyer => "买方",
            },
            transport_mode: if incoterm.is_sea_only() {
                "海运/内河运输"
            } else {
                "任意运输方式"
            },
        })
    }

    /// V15 P1 batch-19 缺陷 23.5.2：按 Incoterm 计算价格构成各成本项
    /// 返回 (product_cost, freight, insurance, duty) 完整价格构成；product_cost 为基础成本始终返回。
    pub fn calculate_costs_by_incoterm(
        incoterm: Incoterms2020,
        product_cost: Decimal,
        freight_cost: Option<Decimal>,
        insurance_cost: Option<Decimal>,
        duty_cost: Option<Decimal>,
    ) -> (Decimal, Option<Decimal>, Option<Decimal>, Option<Decimal>) {
        // EXW/FCA/FAS/FOB 不含主运费（FOB 主运费由买方订立，ICC Incoterms 2020）
        let freight = if incoterm.includes_freight() {
            freight_cost
        } else {
            None
        };
        // CIF/CIP/DDP 含保险
        let insurance = if incoterm.includes_insurance() {
            insurance_cost
        } else {
            None
        };
        // 仅 DDP 含关税
        let duty = if incoterm.requires_duty_paid() {
            duty_cost
        } else {
            None
        };
        (product_cost, freight, insurance, duty)
    }

    /// 生成术语使用月报（按数据范围过滤可见报价单行）
    pub async fn monthly_usage_report(
        &self,
        year: i32,
        month: u32,
        ctx: Option<&DataScopeContext>,
    ) -> Result<IncotermsMonthlyReport, AppError> {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::validation_displayable("无效的年月".to_string()))?;
        let next_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| AppError::validation_displayable("无效的年月".to_string()))?;

        // 行级数据范围：报价单的归属人是 sales_user_id（与报价域列表/单行门同列，
        // 本表另有记录建档人的 created_by，不可混用）。All 不加过滤；Dept 按
        // 「sales_user_id ∈ 可见部门成员集合」，Self 仅本人。
        // Some(ids)=按这些归属人过滤；None=不加行级过滤（All 范围，以及不传 ctx 的
        // 内部调度通路，后者由调用方 RBAC 兜底）。
        let owner_filter: Option<Vec<i32>> = ctx.and_then(|c| match c.scope {
            DataScope::All => None,
            DataScope::Self_ => Some(vec![c.user_id]),
            DataScope::Dept => Some(if c.dept_member_user_ids.is_empty() {
                vec![c.user_id]
            } else {
                c.dept_member_user_ids.clone()
            }),
        });

        // 成员逐个展开成位置参数（$3..$n）拼 IN 列表：参数个数由服务端集合长度决定，
        // 值全部来自会话态整型 ID，不经字符串拼接。
        let (scope_fragment, scope_params): (String, Vec<Value>) = match owner_filter {
            Some(ids) => {
                let placeholders = ids
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", i + 3))
                    .collect::<Vec<_>>()
                    .join(", ");
                let params = ids.into_iter().map(|id| Value::Int(Some(id))).collect();
                (format!("AND sales_user_id IN ({placeholders})"), params)
            }
            None => (String::new(), Vec::new()),
        };

        let sql = format!(
            "SELECT \
                price_terms as incoterm, \
                COUNT(*) as count, \
                COALESCE(SUM(total_amount), 0) as total_amount, \
                COALESCE(SUM(freight_cost), 0) as freight_cost, \
                COALESCE(SUM(insurance_cost), 0) as insurance_cost, \
                COALESCE(SUM(duty_cost), 0) as duty_cost \
            FROM sales_quotations \
            WHERE quotation_date >= $1 AND quotation_date < $2 \
            AND status = 'approved' {} \
            GROUP BY price_terms \
            ORDER BY count DESC",
            scope_fragment
        );

        let mut bind_values: Vec<Value> = vec![start_date.into(), next_month.into()];
        bind_values.extend(scope_params);

        let items = IncotermUsageItem::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            &sql,
            bind_values,
        ))
        .all(&*self.db)
        .await?;

        Ok(IncotermsMonthlyReport { year, month, items })
    }
}

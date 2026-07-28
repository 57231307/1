use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

use crate::models::{
    assist_accounting_dimension, assist_accounting_record, assist_accounting_summary,
};
use crate::utils::error::AppError;

/// 辅助核算服务
#[derive(Debug, Clone)]
pub struct AssistAccountingService {
    db: Arc<DatabaseConnection>,
}

/// V15 P1 17.2-D1：主辅账平衡校验结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssistVsGeneralBalanceResult {
    /// 会计期间（YYYY-MM）
    pub accounting_period: String,
    /// 辅助核算借方总额
    pub assist_total_debit: Decimal,
    /// 辅助核算贷方总额
    pub assist_total_credit: Decimal,
    /// 总账借方总额
    pub general_total_debit: Decimal,
    /// 总账贷方总额
    pub general_total_credit: Decimal,
    /// 借方差异（辅助 - 总账）
    pub debit_diff: Decimal,
    /// 贷方差异（辅助 - 总账）
    pub credit_diff: Decimal,
    /// 是否平衡（差异均为零）
    pub is_balanced: bool,
}

impl AssistAccountingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 初始化 8 个辅助核算维度
    ///
    /// 批次 120 P2-7 修复：原方法保留 `#[allow(dead_code)]` 标记，违反规则 0（真实实现强制）。
    /// 已接入 main.rs 启动流程：服务启动时调用一次（在 init_event_bus_with_kafka_config 之后），
    /// 内部先检查每个维度是否存在再插入，重启不会重复创建（幂等实现）。
    pub async fn initialize_dimensions(&self) -> Result<(), AppError> {
        let dimensions = [
            ("BATCH", "批次核算", "按生产批次进行辅助核算"),
            ("COLOR", "色号核算", "按产品色号进行辅助核算"),
            ("DYE_LOT", "缸号核算", "按染色缸次进行辅助核算"),
            ("GRADE", "等级核算", "按产品质量等级进行辅助核算"),
            ("WORKSHOP", "车间核算", "按生产车间进行辅助核算"),
            ("WAREHOUSE", "仓库核算", "按仓库进行辅助核算"),
            ("CUSTOMER", "客户核算", "按客户进行辅助核算"),
            ("SUPPLIER", "供应商核算", "按供应商进行辅助核算"),
        ];

        for (i, (code, name, desc)) in dimensions.iter().enumerate() {
            let dimension = assist_accounting_dimension::ActiveModel {
                id: Default::default(),
                dimension_code: Set(code.to_string()),
                dimension_name: Set(name.to_string()),
                description: Set(Some(desc.to_string())),
                is_active: Set(true),
                sort_order: Set((i + 1) as i32),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };

            // 如果不存在则插入
            let exists = assist_accounting_dimension::Entity::find()
                .filter(assist_accounting_dimension::Column::DimensionCode.eq(*code))
                .one(&*self.db)
                .await?;

            if exists.is_none() {
                dimension.insert(&*self.db).await?;
            }
        }

        Ok(())
    }

    /// 按业务类型和业务单号查询辅助核算记录
    pub async fn find_by_business(
        &self,
        business_type: &str,
        business_no: &str,
    ) -> Result<Vec<assist_accounting_record::Model>, AppError> {
        assist_accounting_record::Entity::find()
            .filter(assist_accounting_record::Column::BusinessType.eq(business_type))
            .filter(assist_accounting_record::Column::BusinessNo.eq(business_no))
            .order_by(assist_accounting_record::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 按五维 ID 查询辅助核算记录
    pub async fn find_by_five_dimension(
        &self,
        five_dimension_id: &str,
    ) -> Result<Vec<assist_accounting_record::Model>, AppError> {
        assist_accounting_record::Entity::find()
            .filter(assist_accounting_record::Column::FiveDimensionId.eq(five_dimension_id))
            .order_by(assist_accounting_record::Column::CreatedAt, Order::Desc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 按会计期间和维度查询汇总
    pub async fn find_summary_by_period_and_dimension(
        &self,
        accounting_period: &str,
        dimension_code: &str,
    ) -> Result<Vec<assist_accounting_summary::Model>, AppError> {
        assist_accounting_summary::Entity::find()
            .filter(assist_accounting_summary::Column::AccountingPeriod.eq(accounting_period))
            .filter(assist_accounting_summary::Column::DimensionCode.eq(dimension_code))
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }


    /// 查询辅助核算明细（带过滤）
    ///
    /// V15 P1 17.2-D2 修复：dimension_code 现在会真实过滤对应维度字段非空的记录。
    /// - "BATCH"：过滤 batch_no 非空
    /// - "COLOR"：过滤 color_no 非空
    /// - "DYE_LOT"：过滤 dye_lot_no 非空
    /// - "GRADE"：过滤 grade 非空
    /// - "WORKSHOP"：过滤 workshop_id 非空
    /// - "WAREHOUSE"：过滤 warehouse_id 非空（与 warehouse_id 参数叠加）
    /// - "CUSTOMER"：过滤 customer_id 非空
    /// - "SUPPLIER"：过滤 supplier_id 非空
    pub async fn query_assist_records(
        &self,
        accounting_period: Option<&str>,
        dimension_code: Option<&str>,
        business_type: Option<&str>,
        warehouse_id: Option<i32>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<assist_accounting_record::Model>, u64), AppError> {
        let query = assist_accounting_record::Entity::find();
        let query = Self::apply_period_filter(query, accounting_period);
        let query = Self::apply_assist_filters(query, dimension_code, business_type, warehouse_id);

        // 分页查询
        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        // SeaORM fetch_page 为 0-indexed，HTTP 层 page 为 1-indexed，需减 1 对齐
        let records = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((records, total))
    }

    /// V15 P1 17.2-D1：主辅账平衡校验
    ///
    /// 校验指定期间内，辅助核算记录的借贷总额与总账（凭证分录）的借贷总额是否一致。
    /// 用于期末对账，发现辅助核算与总账数据不一致问题。
    ///
    /// 校验逻辑：
    /// 1. 汇总指定期间内所有辅助核算记录的 debit_amount/credit_amount 总额
    /// 2. 汇总同期总账（voucher_item JOIN voucher，status=posted）的 debit/credit 总额
    /// 3. 比较两者，返回差异详情
    ///
    /// 返回 (辅助核算借方总额, 辅助核算贷方总额, 总账借方总额, 总账贷方总额, 是否平衡)
    pub async fn check_assist_vs_general_balance(
        &self,
        accounting_period: &str,
    ) -> Result<AssistVsGeneralBalanceResult, AppError> {
        use crate::models::{voucher, voucher_item};
        use crate::models::status::voucher::VOUCHER_POSTED;
        use sea_orm::sea_query::Expr;

        // 1. 汇总辅助核算记录的借贷总额（按期间过滤）
        let (start_date, end_date) = parse_period_range(accounting_period)?;
        let assist_agg: Option<(Option<Decimal>, Option<Decimal>)> =
            assist_accounting_record::Entity::find()
                .filter(assist_accounting_record::Column::CreatedAt.gte(start_date))
                .filter(assist_accounting_record::Column::CreatedAt.lte(end_date))
                .select_only()
                .column_as(
                    Expr::col(assist_accounting_record::Column::DebitAmount).sum(),
                    "total_debit",
                )
                .column_as(
                    Expr::col(assist_accounting_record::Column::CreditAmount).sum(),
                    "total_credit",
                )
                .into_tuple()
                .one(&*self.db)
                .await?;
        let (assist_debit_opt, assist_credit_opt) = assist_agg.unwrap_or((None, None));
        let assist_total_debit = assist_debit_opt.unwrap_or(Decimal::ZERO);
        let assist_total_credit = assist_credit_opt.unwrap_or(Decimal::ZERO);

        // 2. 汇总总账（已过账凭证分录）的借贷总额
        let general_agg: Option<(Option<Decimal>, Option<Decimal>)> = voucher_item::Entity::find()
            .join(
                sea_orm::JoinType::InnerJoin,
                voucher_item::Relation::Voucher.def(),
            )
            .filter(voucher::Column::Status.eq(VOUCHER_POSTED))
            .filter(voucher::Column::VoucherDate.gte(start_date))
            .filter(voucher::Column::VoucherDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(voucher_item::Column::Debit).sum(), "total_debit")
            .column_as(Expr::col(voucher_item::Column::Credit).sum(), "total_credit")
            .into_tuple()
            .one(&*self.db)
            .await?;
        let (general_debit_opt, general_credit_opt) = general_agg.unwrap_or((None, None));
        let general_total_debit = general_debit_opt.unwrap_or(Decimal::ZERO);
        let general_total_credit = general_credit_opt.unwrap_or(Decimal::ZERO);

        // 3. 比较差异
        let debit_diff = assist_total_debit - general_total_debit;
        let credit_diff = assist_total_credit - general_total_credit;
        let is_balanced = debit_diff == Decimal::ZERO && credit_diff == Decimal::ZERO;

        Ok(AssistVsGeneralBalanceResult {
            accounting_period: accounting_period.to_string(),
            assist_total_debit,
            assist_total_credit,
            general_total_debit,
            general_total_credit,
            debit_diff,
            credit_diff,
            is_balanced,
        })
    }

    /// 应用会计期间过滤，将期间字符串解析为日期范围
    fn apply_period_filter(
        query: sea_orm::Select<assist_accounting_record::Entity>,
        accounting_period: Option<&str>,
    ) -> sea_orm::Select<assist_accounting_record::Entity> {
        if let Some(period) = accounting_period {
            if let Ok((year, month)) = parse_period(period) {
                let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1).map(|d| {
                    d.and_hms_opt(0, 0, 0)
                        .unwrap_or_else(|| d.and_hms_opt(0, 0, 0).unwrap_or_default())
                        .and_utc()
                });
                let end_date = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                } else {
                    chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                }
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|d| d - chrono::Duration::seconds(1))
                .map(|d| d.and_utc());

                let mut query = query;
                if let Some(start) = start_date {
                    query = query.filter(assist_accounting_record::Column::CreatedAt.gte(start));
                }
                if let Some(end) = end_date {
                    query = query.filter(assist_accounting_record::Column::CreatedAt.lte(end));
                }
                return query;
            }
        }
        query
    }

    /// 应用维度、业务类型、仓库过滤
    ///
    /// V15 P1 17.2-D2 修复：实现各维度的真实过滤逻辑。
    /// 每个维度过滤对应字段非空/非零的记录，确保维度过滤实际生效。
    fn apply_assist_filters(
        query: sea_orm::Select<assist_accounting_record::Entity>,
        dimension_code: Option<&str>,
        business_type: Option<&str>,
        warehouse_id: Option<i32>,
    ) -> sea_orm::Select<assist_accounting_record::Entity> {
        let mut query = query;
        // 按维度过滤：确保对应维度字段非空/非零
        if let Some(dimension) = dimension_code {
            match dimension {
                "BATCH" => {
                    // 批次过滤：batch_no 非空字符串
                    query = query.filter(assist_accounting_record::Column::BatchNo.ne(""));
                    query = query.filter(assist_accounting_record::Column::BatchNo.is_not_null());
                }
                "COLOR" => {
                    // 色号过滤：color_no 非空字符串
                    query = query.filter(assist_accounting_record::Column::ColorNo.ne(""));
                    query = query.filter(assist_accounting_record::Column::ColorNo.is_not_null());
                }
                "DYE_LOT" => {
                    // 缸号过滤：dye_lot_no 非空
                    query = query.filter(assist_accounting_record::Column::DyeLotNo.is_not_null());
                    query = query.filter(assist_accounting_record::Column::DyeLotNo.ne(""));
                }
                "GRADE" => {
                    // 等级过滤：grade 非空字符串
                    query = query.filter(assist_accounting_record::Column::Grade.ne(""));
                    query = query.filter(assist_accounting_record::Column::Grade.is_not_null());
                }
                "WORKSHOP" => {
                    // 车间过滤：workshop_id 非空
                    query = query.filter(assist_accounting_record::Column::WorkshopId.is_not_null());
                }
                "WAREHOUSE" => {
                    // 仓库过滤：warehouse_id 非零（所有记录都有 warehouse_id，这里过滤有效仓库）
                    query = query.filter(assist_accounting_record::Column::WarehouseId.gt(0));
                }
                "CUSTOMER" => {
                    // 客户过滤：customer_id 非空
                    query = query.filter(assist_accounting_record::Column::CustomerId.is_not_null());
                }
                "SUPPLIER" => {
                    // 供应商过滤：supplier_id 非空
                    query = query.filter(assist_accounting_record::Column::SupplierId.is_not_null());
                }
                _ => {}
            }
        }

        if let Some(biz_type) = business_type {
            query = query.filter(assist_accounting_record::Column::BusinessType.eq(biz_type));
        }
        if let Some(wid) = warehouse_id {
            query = query.filter(assist_accounting_record::Column::WarehouseId.eq(wid));
        }
        query
    }


    /// 查询所有启用的辅助核算维度
    pub async fn list_dimensions(
        &self,
    ) -> Result<Vec<assist_accounting_dimension::Model>, AppError> {
        assist_accounting_dimension::Entity::find()
            .filter(assist_accounting_dimension::Column::IsActive.eq(true))
            .order_by(assist_accounting_dimension::Column::SortOrder, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }
}

fn parse_period(period: &str) -> Result<(i32, u32), AppError> {
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() != 2 {
        return Err(AppError::validation(
            "期间格式错误，应为 YYYY-MM".to_string(),
        ));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::validation("年份解析错误"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::validation("月份解析错误"))?;
    if !(1..=12).contains(&month) {
        return Err(AppError::validation("月份必须在1-12之间"));
    }
    Ok((year, month))
}

/// V15 P1 17.2-D1：解析期间字符串为日期范围 (start, end)
///
/// 返回该月第一天 00:00:00 UTC 到该月最后一天 23:59:59 UTC。
fn parse_period_range(period: &str) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>), AppError> {
    let (year, month) = parse_period(period)?;
    let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::validation(format!("无效的起始日期: {}-{:02}-01", year, month)))?
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_utc();
    let end_date = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| AppError::validation(format!("无效的结束日期: {}-{:02}", year, month)))?
    .and_hms_opt(0, 0, 0)
    .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    .and_utc()
    - chrono::Duration::seconds(1);
    Ok((start_date, end_date))
}

use chrono::Utc;
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
    pub async fn query_assist_records(
        &self,
        accounting_period: Option<&str>,
        dimension_code: Option<&str>,
        business_type: Option<&str>,
        warehouse_id: Option<i32>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<assist_accounting_record::Model>, u64), AppError> {
        use sea_orm::ColumnTrait;

        let mut query = assist_accounting_record::Entity::find();

        // 按会计期间过滤（将期间转换为日期范围）
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

                if let Some(start) = start_date {
                    query = query.filter(assist_accounting_record::Column::CreatedAt.gte(start));
                }
                if let Some(end) = end_date {
                    query = query.filter(assist_accounting_record::Column::CreatedAt.lte(end));
                }
            }
        }

        // 按维度过滤
        if let Some(dimension) = dimension_code {
            match dimension {
                "BATCH" => {
                    // 批次过滤通过five_dimension_id或其他方式
                }
                "COLOR" => {
                    // 色号过滤
                }
                "DYE_LOT" => {
                    // 缸号过滤
                }
                "GRADE" => {
                    // 等级过滤
                }
                "WORKSHOP" => {
                    // 车间过滤
                }
                "WAREHOUSE" => {
                    // 仓库过滤（已有warehouse_id过滤）
                }
                "CUSTOMER" => {
                    // 客户过滤
                }
                "SUPPLIER" => {
                    // 供应商过滤
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

        // 分页查询
        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        // SeaORM fetch_page 为 0-indexed，HTTP 层 page 为 1-indexed，需减 1 对齐
        let records = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((records, total))
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

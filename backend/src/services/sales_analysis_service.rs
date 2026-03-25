use crate::models::sales_analysis;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, PaginatorTrait, Order, Set,
};
use std::sync::Arc;
use crate::utils::error::AppError;
use tracing::info;
use serde::Deserialize;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Default)]
pub struct SalesStatisticQueryParams {
    pub statistic_type: Option<String>,
    pub period: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CreateSalesTargetInput {
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: Decimal,
    pub start_date: String,
    pub end_date: String,
}

pub struct SalesAnalysisService {
    db: Arc<DatabaseConnection>,
}

impl SalesAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_statistics_list(
        &self,
        params: SalesStatisticQueryParams,
    ) -> Result<(Vec<sales_analysis::Model>, u64), AppError> {
        let mut query = sales_analysis::Entity::find();

        if let Some(statistic_type) = &params.statistic_type {
            query = query.filter(sales_analysis::Column::StatisticType.eq(statistic_type));
        }

        if let Some(period) = &params.period {
            query = query.filter(sales_analysis::Column::Period.eq(period));
        }

        let total = query.clone().count(&*self.db).await?;

        let statistics = query
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((statistics, total))
    }

    #[allow(dead_code)]
    pub async fn get_trends(
        &self,
        period: &str,
    ) -> Result<Vec<sales_analysis::Model>, AppError> {
        info!("查询销售趋势，周期：{}", period);

        let trends = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::Period.eq(period))
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(trends)
    }

    #[allow(dead_code)]
    pub async fn get_rankings(
        &self,
        period: Option<&str>,
        limit: i64,
    ) -> Result<Vec<sales_analysis::Model>, AppError> {
        info!("查询销售排名，周期：{:?}", period);

        let mut query = sales_analysis::Entity::find();

        if let Some(p) = period {
            query = query.filter(sales_analysis::Column::Period.eq(p));
        }

        let rankings = query
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        Ok(rankings)
    }

    #[allow(dead_code)]
    pub async fn get_targets(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<sales_analysis::Model>, u64), AppError> {
        info!("查询销售目标列表");

        let query = sales_analysis::Entity::find()
            .filter(sales_analysis::Column::StatisticType.eq("target".to_string()));

        let total = query.clone().count(&*self.db).await?;

        let targets = query
            .order_by(sales_analysis::Column::Id, Order::Desc)
            .offset((page * page_size) as u64)
            .limit(page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((targets, total))
    }

    #[allow(dead_code)]
    pub async fn create_target(
        &self,
        req: CreateSalesTargetInput,
        _user_id: i32,
    ) -> Result<sales_analysis::Model, AppError> {
        info!("正在创建销售目标");

        let active_target = sales_analysis::ActiveModel {
            statistic_type: Set("target".to_string()),
            period: Set(req.period),
            dimension_type: Set(req.target_type),
            dimension_id: Set(Some(req.target_id)),
            dimension_name: Set(Some(format!("开始日期: {}, 结束日期: {}", req.start_date, req.end_date))),
            total_amount: Set(req.target_amount),
            ..Default::default()
        };

        let target = active_target.insert(&*self.db).await?;
        info!("销售目标创建成功，ID: {}", target.id);
        Ok(target)
    }
}

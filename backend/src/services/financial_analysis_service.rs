use crate::models::financial_analysis;
use crate::models::financial_analysis_result;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    Set, QuerySelect, PaginatorTrait, Order,
};
use std::sync::Arc;
use rust_decimal::Decimal;
use crate::utils::error::AppError;
use tracing::info;
use serde::Deserialize;
use chrono::Utc;

#[derive(Debug, Clone, Default)]
pub struct IndicatorQueryParams {
    pub indicator_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CreateIndicatorRequest {
    pub indicator_name: String,
    pub indicator_code: String,
    pub indicator_type: String,
    pub formula: Option<String>,
    pub unit: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FinancialAnalysisRequest {
    pub analysis_type: String,
    pub period: String,
    pub indicator_id: i32,
    pub indicator_value: Decimal,
    pub target_value: Option<Decimal>,
}

pub struct FinancialAnalysisService {
    db: Arc<DatabaseConnection>,
}

impl FinancialAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_indicators_list(
        &self,
        params: IndicatorQueryParams,
    ) -> Result<(Vec<financial_analysis::Model>, u64), AppError> {
        let mut query = financial_analysis::Entity::find();

        if let Some(indicator_type) = &params.indicator_type {
            query = query.filter(financial_analysis::Column::IndicatorType.eq(indicator_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(financial_analysis::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let indicators = query
            .order_by(financial_analysis::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((indicators, total))
    }

    #[allow(dead_code)]
    pub async fn create_indicator(
        &self,
        req: CreateIndicatorRequest,
        _user_id: i32,
    ) -> Result<financial_analysis::Model, AppError> {
        info!("正在创建财务指标：{}", req.indicator_code);

        let active_indicator = financial_analysis::ActiveModel {
            indicator_name: Set(req.indicator_name),
            indicator_code: Set(req.indicator_code),
            indicator_type: Set(req.indicator_type),
            formula: Set(req.formula),
            unit: Set(req.unit),
            status: Set("active".to_string()),
            remark: Set(req.remark),
            ..Default::default()
        };

        let indicator = active_indicator.insert(&*self.db).await?;
        info!("财务指标创建成功：{}", indicator.indicator_code);
        Ok(indicator)
    }

    #[allow(dead_code)]
    pub async fn create_analysis_result(
        &self,
        req: FinancialAnalysisRequest,
        user_id: i32,
    ) -> Result<financial_analysis_result::Model, AppError> {
        info!("用户 {} 正在创建财务分析结果：类型={}, 周期={}, 指标ID={}",
              user_id, req.analysis_type, req.period, req.indicator_id);

        // 计算差异
        let variance = req.target_value.map(|t| req.indicator_value - t);

        // 计算差异率
        let variance_rate = req.target_value.and_then(|t| {
            if t != Decimal::ZERO {
                Some((req.indicator_value - t) / t * Decimal::from(100))
            } else {
                None
            }
        });

        // 判断趋势方向
        let trend = variance.map(|v| {
            if v > Decimal::ZERO {
                "上升".to_string()
            } else if v < Decimal::ZERO {
                "下降".to_string()
            } else {
                "持平".to_string()
            }
        });

        // 创建分析结果记录
        let active_result = financial_analysis_result::ActiveModel {
            analysis_type: Set(req.analysis_type),
            period: Set(req.period),
            indicator_id: Set(req.indicator_id),
            indicator_value: Set(req.indicator_value),
            target_value: Set(req.target_value),
            variance: Set(variance),
            variance_rate: Set(variance_rate),
            trend: Set(trend),
            analysis_date: Set(Some(Utc::now().date_naive())),
            created_by: Set(Some(user_id)),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        let result = active_result.insert(&*self.db).await?;
        info!("财务分析结果创建成功，记录ID：{}", result.id);
        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn get_trends(
        &self,
        indicator_id: i32,
        limit: i64,
    ) -> Result<Vec<financial_analysis_result::Model>, AppError> {
        info!("查询财务指标 {} 的趋势数据，限制：{} 条", indicator_id, limit);

        let results = financial_analysis_result::Entity::find()
            .filter(financial_analysis_result::Column::IndicatorId.eq(indicator_id))
            .order_by(financial_analysis_result::Column::AnalysisDate, Order::Desc)
            .limit(limit as u64)
            .all(&*self.db)
            .await?;

        info!("查询到 {} 条趋势数据", results.len());
        Ok(results)
    }
}

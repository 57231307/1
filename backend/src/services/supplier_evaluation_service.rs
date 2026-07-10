use crate::models::supplier_evaluation;
use crate::models::supplier_evaluation_record;
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
// 批次 258 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct EvaluationIndicatorQueryParams {
    pub category: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEvaluationIndicatorRequest {
    pub indicator_name: String,
    pub indicator_code: String,
    pub category: String,
    pub weight: Decimal,
    pub max_score: i32,
    pub evaluation_method: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SupplierEvaluationRequest {
    pub supplier_id: i32,
    pub evaluation_period: String,
    pub indicator_id: i32,
    pub score: Decimal,
    pub remark: Option<String>,
}

/// 供应商评分响应结构
#[derive(Debug, Clone, Serialize)]
pub struct SupplierScoreResponse {
    /// 供应商ID
    pub supplier_id: i32,
    /// 平均评分
    pub average_score: Decimal,
    /// 评估记录总数
    pub total_records: i64,
    /// 等级（A/B/C/D）
    pub rating: String,
    /// 最近评估日期
    pub latest_evaluation_date: Option<DateTime<Utc>>,
}

pub struct SupplierEvaluationService {
    db: Arc<DatabaseConnection>,
}

impl SupplierEvaluationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_indicators_list(
        &self,
        params: EvaluationIndicatorQueryParams,
    ) -> Result<(Vec<supplier_evaluation::Model>, u64), AppError> {
        let mut query = supplier_evaluation::Entity::find();

        if let Some(category) = &params.category {
            query = query.filter(supplier_evaluation::Column::Category.eq(category));
        }

        if let Some(status) = &params.status {
            query = query.filter(supplier_evaluation::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let indicators = query
            .order_by(supplier_evaluation::Column::Id, Order::Desc)
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((indicators, total))
    }

    pub async fn create_indicator(
        &self,
        req: CreateEvaluationIndicatorRequest,
        user_id: i32,
    ) -> Result<supplier_evaluation::Model, AppError> {
        info!("用户 {} 正在创建评估指标：{}", user_id, req.indicator_code);

        // 检查指标编码是否重复
        let existing = supplier_evaluation::Entity::find()
            .filter(supplier_evaluation::Column::IndicatorCode.eq(&req.indicator_code))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::validation(format!(
                "评估指标编码 '{}' 已存在",
                req.indicator_code
            )));
        }

        let active_indicator = supplier_evaluation::ActiveModel {
            indicator_name: Set(req.indicator_name),
            indicator_code: Set(req.indicator_code),
            category: Set(req.category),
            weight: Set(req.weight),
            max_score: Set(req.max_score),
            evaluation_method: Set(req.evaluation_method),
            status: Set(master_data::ACTIVE.to_string()),
            ..Default::default()
        };

        let indicator = active_indicator.insert(&*self.db).await?;
        info!("评估指标创建成功：{}", indicator.indicator_code);
        Ok(indicator)
    }

    pub async fn create_evaluation_record(
        &self,
        req: SupplierEvaluationRequest,
        user_id: i32,
    ) -> Result<supplier_evaluation_record::Model, AppError> {
        info!(
            "用户 {} 正在评估供应商 {}，指标ID：{}，得分：{}",
            user_id, req.supplier_id, req.indicator_id, req.score
        );

        // 检查供应商是否存在
        use crate::models::supplier;
        let supplier_exists = supplier::Entity::find_by_id(req.supplier_id)
            .one(&*self.db)
            .await?;
        if supplier_exists.is_none() {
            return Err(AppError::not_found(format!(
                "供应商不存在，ID：{}",
                req.supplier_id
            )));
        }

        // 查询指标信息以获取权重和满分
        let indicator = supplier_evaluation::Entity::find_by_id(req.indicator_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("评估指标不存在，ID：{}", req.indicator_id))
            })?;

        // 计算加权得分
        let weighted_score = if indicator.max_score > 0 {
            Some(req.score * indicator.weight / Decimal::from(indicator.max_score))
        } else {
            None
        };

        // 校验得分范围
        if req.score < Decimal::ZERO || req.score > Decimal::from(indicator.max_score) {
            return Err(AppError::validation(format!(
                "得分 {} 超出有效范围 [0, {}]",
                req.score, indicator.max_score
            )));
        }

        // 创建评估记录
        let active_record = supplier_evaluation_record::ActiveModel {
            supplier_id: Set(req.supplier_id),
            evaluation_period: Set(req.evaluation_period),
            indicator_id: Set(req.indicator_id),
            score: Set(req.score),
            max_score: Set(Some(indicator.max_score)),
            weighted_score: Set(weighted_score),
            evaluator_id: Set(Some(user_id)),
            evaluation_date: Set(Some(chrono::Utc::now().date_naive())),
            remark: Set(req.remark),
            created_at: Set(chrono::Utc::now()),
            ..Default::default()
        };

        let record = active_record.insert(&*self.db).await?;
        info!("供应商评估记录创建成功，记录ID：{}", record.id);
        Ok(record)
    }

    pub async fn get_supplier_score(
        &self,
        supplier_id: i32,
    ) -> Result<SupplierScoreResponse, AppError> {
        info!("查询供应商 {} 的评分", supplier_id);

        // 查询该供应商的所有评估记录
        let records = supplier_evaluation_record::Entity::find()
            .filter(supplier_evaluation_record::Column::SupplierId.eq(supplier_id))
            .all(&*self.db)
            .await?;

        if records.is_empty() {
            return Err(AppError::not_found(format!(
                "供应商 {} 暂无评估记录",
                supplier_id
            )));
        }

        // 计算加权平均分
        let total_weighted_score: Decimal = records.iter().filter_map(|r| r.weighted_score).sum();

        // 使用 HashSet 去重指标 ID，批量查询指标权重
        let indicator_ids: std::collections::HashSet<i32> =
            records.iter().map(|r| r.indicator_id).collect();
        let indicator_weight_map: HashMap<i32, Decimal> = supplier_evaluation::Entity::find()
            .filter(supplier_evaluation::Column::Id.is_in(indicator_ids.iter().cloned()))
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|ind| (ind.id, ind.weight))
            .collect();
        let total_weight: Decimal = indicator_weight_map.values().sum();

        let average_score = if total_weight > Decimal::ZERO {
            total_weighted_score / total_weight * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // 计算等级（解析失败时记 warn 并按 0 分处理）
        let rating = match average_score.to_string().parse::<i32>() {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("供应商评分解析失败: {} ({})", average_score, e);
                0
            }
        };
        let rating_label = match rating {
            90..=100 => "A".to_string(),
            80..=89 => "B".to_string(),
            70..=79 => "C".to_string(),
            _ => "D".to_string(),
        };

        // 获取最近评估日期
        let latest_evaluation_date =
            records
                .iter()
                .filter_map(|r| r.evaluation_date)
                .max()
                .map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(
                        d.and_hms_opt(0, 0, 0).unwrap_or_default(),
                        Utc,
                    )
                });

        info!(
            "供应商 {} 评分查询完成，平均分：{}，等级：{}",
            supplier_id, average_score, rating
        );

        Ok(SupplierScoreResponse {
            supplier_id,
            average_score,
            total_records: records.len() as i64,
            rating: rating_label,
            latest_evaluation_date,
        })
    }

    /// 查询供应商评级列表
    ///
    /// BE-P 优化（2026-06-26）：补齐分页参数，避免全量返回。
    pub async fn list_ratings(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<supplier_evaluation::Model>, u64), AppError> {
        info!("查询供应商评级列表，页码：{}，每页：{}", page, page_size);
        // 批次 258 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = supplier_evaluation::Entity::find()
            .order_by(supplier_evaluation::Column::Id, Order::Desc)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    pub async fn get_supplier_rankings(
        &self,
        limit: i64,
    ) -> Result<Vec<SupplierScoreResponse>, AppError> {
        // BE-P 说明（2026-06-26）：
        // 本方法涉及跨表权重查询 + 加权平均计算 + 等级评定，无法简单下推 SQL 聚合。
        // 当前全量加载 + 内存分组计算 + 排序截断是合理的业务实现。
        // 后续优化方向：将计算结果持久化到 supplier_score_summary 表，按 average_score 排序 + LIMIT 查询。
        info!("查询供应商排名榜，限制：{} 条", limit);

        let records = supplier_evaluation_record::Entity::find()
            .all(&*self.db)
            .await?;

        if records.is_empty() {
            return Ok(vec![]);
        }

        // 按供应商分组，收集加权得分和记录数
        let mut supplier_records: std::collections::HashMap<
            i32,
            Vec<&supplier_evaluation_record::Model>,
        > = std::collections::HashMap::new();

        for record in &records {
            supplier_records
                .entry(record.supplier_id)
                .or_default()
                .push(record);
        }

        // 批量查询所有指标的权重
        let all_indicator_ids: std::collections::HashSet<i32> =
            records.iter().map(|r| r.indicator_id).collect();
        let indicator_weight_map: HashMap<i32, Decimal> = supplier_evaluation::Entity::find()
            .filter(supplier_evaluation::Column::Id.is_in(all_indicator_ids.iter().cloned()))
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|ind| (ind.id, ind.weight))
            .collect();

        let mut rankings: Vec<SupplierScoreResponse> = Vec::new();
        for (supplier_id, recs) in &supplier_records {
            let total_weighted_score: Decimal = recs.iter().filter_map(|r| r.weighted_score).sum();
            let total_records = recs.len() as i64;

            // 计算每个供应商的总权重（与 get_supplier_score 一致）
            let indicator_ids: std::collections::HashSet<i32> =
                recs.iter().map(|r| r.indicator_id).collect();
            let total_weight: Decimal = indicator_ids
                .iter()
                .filter_map(|id| indicator_weight_map.get(id))
                .sum();

            let average_score = if total_weight > Decimal::ZERO {
                total_weighted_score / total_weight * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            // 计算等级（解析失败时记 warn 并按 0 分处理）
            let rating = match average_score.to_string().parse::<i32>() {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("供应商评分解析失败: {} ({})", average_score, e);
                    0
                }
            };
            let rating_label = match rating {
                90..=100 => "A".to_string(),
                80..=89 => "B".to_string(),
                70..=79 => "C".to_string(),
                _ => "D".to_string(),
            };

            let latest_evaluation_date =
                recs.iter()
                    .filter_map(|r| r.evaluation_date)
                    .max()
                    .map(|d| {
                        DateTime::<Utc>::from_naive_utc_and_offset(
                            d.and_hms_opt(0, 0, 0).unwrap_or_default(),
                            Utc,
                        )
                    });

            rankings.push(SupplierScoreResponse {
                supplier_id: *supplier_id,
                average_score,
                total_records,
                rating: rating_label,
                latest_evaluation_date,
            });
        }

        rankings.sort_by_key(|b| std::cmp::Reverse(b.average_score));
        rankings.truncate(limit as usize);

        info!("查询到 {} 个供应商排名", rankings.len());
        Ok(rankings)
    }

    pub async fn get_evaluation_records(
        &self,
        supplier_id: Option<i32>,
        period: Option<String>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<supplier_evaluation_record::Model>, AppError> {
        info!(
            "查询评估记录列表，supplier_id: {:?}, period: {:?}",
            supplier_id, period
        );

        let mut query = supplier_evaluation_record::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(supplier_evaluation_record::Column::SupplierId.eq(sid));
        }
        if let Some(p) = period {
            query = query.filter(supplier_evaluation_record::Column::EvaluationPeriod.eq(p));
        }

        let offset = ((page.clamp(1, 1000) - 1) * page_size) as u64;
        let limit = page_size as u64;
        let records = query
            .order_by(supplier_evaluation_record::Column::Id, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(&*self.db)
            .await?;

        info!("查询到 {} 条评估记录", records.len());
        Ok(records)
    }

    pub async fn get_evaluation_record_by_id(
        &self,
        id: i32,
    ) -> Result<supplier_evaluation_record::Model, AppError> {
        info!("查询评估记录详情：{}", id);

        let record = supplier_evaluation_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("评估记录不存在：{}", id)))?;

        Ok(record)
    }
}

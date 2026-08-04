//! AI 分析深化服务（ai_extend）
//!
//! 提供工艺优化与质量预测的持久化与历史查询能力。
//! 算法核心复用 `crate::services::ai::recipe_opt` 与
//! `crate::services::ai::quality_pred`；本模块只负责：
//! 1. 落库（创建 ai_process_optimizations / ai_quality_predictions 记录）
//! 2. 列表 / 详情查询
//! 3. 标记应用 / 确认
//! 4. 反馈打分
//! 5. 看板聚合

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::ai_process_optimization::{
    ActiveModel as ProcessActiveModel, Column as ProcessColumn, Entity as ProcessEntity,
    Model as ProcessModel,
};
use crate::models::ai_quality_prediction::{
    ActiveModel as QualityActiveModel, Column as QualityColumn, Entity as QualityEntity,
    Model as QualityModel,
};
use crate::services::lab_dip_service::{CreateLabDipRequestRequest, LabDipRequestService};
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

use super::ai::quality_pred::{QualityPredRequest, QualityPredResponse};
use super::ai::recipe_opt::{RecipeOptRequest, RecipeOptResponse};
use super::ai::AiAnalysisService;

// =====================================================
// 工艺优化 持久化
// =====================================================

#[derive(Debug, Deserialize)]
pub struct CreateProcessOptDto {
    /// 工艺优化请求体（color_no / fabric_type / dye_type / k）
    pub request: RecipeOptRequest,
    /// 操作员 ID（来自 auth context，可选）
    pub operator_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ProcessOptDetailVo {
    #[serde(flatten)]
    pub model: ProcessModel,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListProcessOptQuery {
    /// 页码（默认 1）
    pub page: Option<u64>,
    /// 每页大小（默认 20）
    pub page_size: Option<u64>,
    /// 按色号过滤
    pub color_no: Option<String>,
    /// 按布类过滤
    pub fabric_type: Option<String>,
    /// 按应用状态过滤
    pub is_applied: Option<bool>,
    /// 按来源过滤（knn / fallback）
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProcessOptListVo {
    pub items: Vec<ProcessModel>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct ApplyProcessOptDto {
    pub operator_id: Option<i64>,
    /// 反馈打分（1-5 星）
    pub feedback_score: Option<i16>,
    pub feedback_remark: Option<String>,
}

/// AI 工艺优化 Service
pub struct AiExtendService {
    pub(crate) db: std::sync::Arc<sea_orm::DatabaseConnection>,
}

// =====================================================
// 质量预测 持久化 DTO（file-level，避免 impl 内 pub struct 编译错误）
// =====================================================

#[derive(Debug, Deserialize)]
pub struct CreateQualityPredDto {
    pub request: QualityPredRequest,
    pub operator_id: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListQualityPredQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i64>,
    pub inspection_type: Option<String>,
    pub risk_level: Option<String>,
    pub is_acknowledged: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct QualityPredListVo {
    pub items: Vec<QualityModel>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct AcknowledgeQualityPredDto {
    pub operator_id: Option<i64>,
}

impl AiExtendService {
    pub fn new(db: std::sync::Arc<sea_orm::DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 触发工艺优化（算法 + 落库，返回响应 + 落库 ID）
    pub async fn create_process_optimization(
        &self,
        dto: CreateProcessOptDto,
    ) -> Result<(RecipeOptResponse, i64), AppError> {
        // 1. 调算法核心（V15 P2 14-10.3：记录推理耗时）
        let infer_start = std::time::Instant::now();
        let ai = AiAnalysisService::new(self.db.clone());
        let resp = ai.optimize_recipe(dto.request.clone()).await?;
        let latency_ms = infer_start.elapsed().as_millis().min(i32::MAX as u128) as i32;

        // 2. 落库
        let request_id = format!("proc-{}", Uuid::new_v4());
        let candidates_json = serde_json::to_value(&resp.candidates)
            .map_err(|e| AppError::internal(format!("序列化 candidates 失败: {}", e)))?;

        let now = chrono::Utc::now();
        let active = ProcessActiveModel {
            request_id: Set(request_id),
            color_no: Set(dto.request.color_no.clone()),
            color_name: Set(dto.request.color_name.clone()),
            fabric_type: Set(dto.request.fabric_type.clone()),
            dye_type: Set(dto.request.dye_type.clone()),
            recommended_temperature: Set(rust_decimal::Decimal::from_f64_retain(
                resp.recommended_params.temperature,
            )
            .unwrap_or_default()),
            recommended_time_minutes: Set(resp.recommended_params.time_minutes),
            recommended_ph_value: Set(rust_decimal::Decimal::from_f64_retain(
                resp.recommended_params.ph_value,
            )
            .unwrap_or_default()),
            recommended_liquor_ratio: Set(rust_decimal::Decimal::from_f64_retain(
                resp.recommended_params.liquor_ratio,
            )
            .unwrap_or_default()),
            similar_cases: Set(resp.similar_cases as i32),
            confidence: Set(
                rust_decimal::Decimal::from_f64_retain(resp.confidence).unwrap_or_default()
            ),
            source: Set(resp.source.clone()),
            reason: Set(Some(resp.reason.clone())),
            candidates_json: Set(Some(candidates_json)),
            is_applied: Set(false),
            applied_at: Set(None),
            applied_by: Set(None),
            feedback_score: Set(None),
            feedback_remark: Set(None),
            // V15 P2 14-10.3：推理耗时毫秒
            inference_latency_ms: Set(Some(latency_ms)),
            created_by: Set(dto.operator_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok((resp, model.id))
    }

    /// 仅调用 AI 算法不做落库（供批量接口阶段一使用）
    pub async fn optimize_recipe_only(
        &self,
        dto: &CreateProcessOptDto,
    ) -> Result<RecipeOptResponse, AppError> {
        let ai = AiAnalysisService::new(self.db.clone());
        ai.optimize_recipe(dto.request.clone()).await
    }

    /// 批量落库工艺优化记录（单事务，全部成功或全部回滚）
    pub async fn batch_insert_optimizations(
        &self,
        items: Vec<(RecipeOptResponse, CreateProcessOptDto)>,
    ) -> Result<Vec<i64>, AppError> {
        let txn = (*self.db).begin().await?;
        let mut ids = Vec::new();
        for (resp, dto) in items {
            let request_id = format!("proc-{}", Uuid::new_v4());
            let candidates_json = serde_json::to_value(&resp.candidates)
                .map_err(|e| AppError::internal(format!("序列化 candidates 失败: {}", e)))?;
            let now = chrono::Utc::now();
            let active = ProcessActiveModel {
                request_id: Set(request_id),
                color_no: Set(dto.request.color_no.clone()),
                color_name: Set(dto.request.color_name.clone()),
                fabric_type: Set(dto.request.fabric_type.clone()),
                dye_type: Set(dto.request.dye_type.clone()),
                recommended_temperature: Set(rust_decimal::Decimal::from_f64_retain(
                    resp.recommended_params.temperature,
                )
                .unwrap_or_default()),
                recommended_time_minutes: Set(resp.recommended_params.time_minutes),
                recommended_ph_value: Set(rust_decimal::Decimal::from_f64_retain(
                    resp.recommended_params.ph_value,
                )
                .unwrap_or_default()),
                recommended_liquor_ratio: Set(rust_decimal::Decimal::from_f64_retain(
                    resp.recommended_params.liquor_ratio,
                )
                .unwrap_or_default()),
                similar_cases: Set(resp.similar_cases as i32),
                confidence: Set(
                    rust_decimal::Decimal::from_f64_retain(resp.confidence).unwrap_or_default()
                ),
                source: Set(resp.source.clone()),
                reason: Set(Some(resp.reason.clone())),
                candidates_json: Set(Some(candidates_json)),
                is_applied: Set(false),
                applied_at: Set(None),
                applied_by: Set(None),
                feedback_score: Set(None),
                feedback_remark: Set(None),
                inference_latency_ms: Set(None),
                created_by: Set(dto.operator_id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            let model = active.insert(&txn).await?;
            ids.push(model.id);
        }
        txn.commit().await?;
        Ok(ids)
    }

    /// 工艺优化列表查询
    /// V15 P0-S27：注入行级数据权限过滤。；AI 推理记录语义为"我创建的工艺优化记录"，使用 created_by（i64 可空）作为 owner_column；AI 表无 department_id 字段，Dept 范围退化为 Self（与 CRM/销售域一致）。
    pub async fn list_process_optimizations(
        &self,
        q: ListProcessOptQuery,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<ProcessOptListVo, AppError> {
        let page = q.page.unwrap_or(1).clamp(1, 1000);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = ProcessEntity::find();
        if let Some(c) = &q.color_no {
            select = select.filter(ProcessColumn::ColorNo.eq(c));
        }
        if let Some(f) = &q.fabric_type {
            select = select.filter(ProcessColumn::FabricType.eq(f));
        }
        if let Some(a) = q.is_applied {
            select = select.filter(ProcessColumn::IsApplied.eq(a));
        }
        if let Some(s) = &q.source {
            select = select.filter(ProcessColumn::Source.eq(s));
        }
        // V15 P0-S27：注入数据范围过滤（无 department_id，Dept 退化为 Self，复用 created_by）
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                ProcessColumn::CreatedBy,
                ProcessColumn::CreatedBy, // 占位：Dept 退化为 Self 时不会用到 dept_column
            );
        }

        let total = select.clone().count(&*self.db).await?;
        let items = select
            .order_by_desc(ProcessColumn::CreatedAt)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok(ProcessOptListVo {
            items,
            total,
            page,
            page_size,
        })
    }

    /// 工艺优化详情（V15 P0-S27：注入归属校验（IDOR 防护）。）
    pub async fn get_process_optimization(
        &self,
        id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<ProcessModel, AppError> {
        let model = ProcessEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工艺优化记录不存在: id={}", id)))?;
        // V15 P0-S27：校验归属（created_by 为 i64，需转为 i32 比对 user_id）
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by.map(|i| i as i32), None) {
                return Err(AppError::permission_denied("无权访问该工艺优化记录"));
            }
        }
        Ok(model)
    }

    /// 按色号 + 布类查询工艺优化历史（V15 P0-S27：注入行级数据权限过滤。）
    pub async fn list_process_optimizations_by_color(
        &self,
        color_no: &str,
        fabric_type: &str,
        limit: u64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<ProcessModel>, AppError> {
        let mut select = ProcessEntity::find()
            .filter(ProcessColumn::ColorNo.eq(color_no))
            .filter(ProcessColumn::FabricType.eq(fabric_type));
        // V15 P0-S27：注入数据范围过滤
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                ProcessColumn::CreatedBy,
                ProcessColumn::CreatedBy,
            );
        }
        let items = select
            .order_by_desc(ProcessColumn::CreatedAt)
            .limit(limit.min(50))
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 标记工艺优化已应用 + 反馈打分（V15 P0-S27：写操作前校验资源归属（防 IDOR）。）
    pub async fn apply_process_optimization(
        &self,
        id: i64,
        dto: ApplyProcessOptDto,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<ProcessModel, AppError> {
        let model = ProcessEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工艺优化记录不存在: id={}", id)))?;
        // V15 P0-S27：校验归属
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by.map(|i| i as i32), None) {
                return Err(AppError::permission_denied("无权操作该工艺优化记录"));
            }
        }

        let now = chrono::Utc::now();
        let mut active: ProcessActiveModel = model.into();
        active.is_applied = Set(true);
        active.applied_at = Set(Some(now));
        active.applied_by = Set(dto.operator_id);
        if let Some(score) = dto.feedback_score {
            if !(1..=5).contains(&score) {
                return Err(AppError::validation("feedback_score 必须在 1-5 范围内"));
            }
            active.feedback_score = Set(Some(score));
        }
        if let Some(remark) = dto.feedback_remark {
            active.feedback_remark = Set(Some(remark));
        }
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除工艺优化记录（V15 P0-S27：写操作前校验资源归属（防 IDOR）。）
    pub async fn delete_process_optimization(
        &self,
        id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(), AppError> {
        // V15 P0-S27：先查记录校验归属，再删除
        let model = ProcessEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工艺优化记录不存在: id={}", id)))?;
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by.map(|i| i as i32), None) {
                return Err(AppError::permission_denied("无权删除该工艺优化记录"));
            }
        }
        let res = ProcessEntity::delete_many()
            .filter(ProcessColumn::Id.eq(id))
            .exec(&*self.db)
            .await?;
        if res.rows_affected == 0 {
            return Err(AppError::not_found(format!(
                "工艺优化记录不存在: id={}",
                id
            )));
        }
        Ok(())
    }

    // =====================================================
    // 质量预测 持久化
    // =====================================================

    /// 触发质量预测（算法 + 落库）
    pub async fn create_quality_prediction(
        &self,
        dto: CreateQualityPredDto,
    ) -> Result<(QualityPredResponse, i64), AppError> {
        let ai = AiAnalysisService::new(self.db.clone());
        let infer_start = std::time::Instant::now();
        let resp = ai.predict_quality(dto.request.clone()).await?;
        let latency_ms = infer_start.elapsed().as_millis().min(i32::MAX as u128) as i32;

        let request_id = format!("qual-{}", Uuid::new_v4());
        let top_issues_json = serde_json::to_value(&resp.top_issues)
            .map_err(|e| AppError::internal(format!("序列化 top_issues 失败: {}", e)))?;
        let recommendations_json = serde_json::to_value(&resp.recommendations)
            .map_err(|e| AppError::internal(format!("序列化 recommendations 失败: {}", e)))?;
        let period_breakdown_json = serde_json::to_value(&resp.period_breakdown)
            .map_err(|e| AppError::internal(format!("序列化 period_breakdown 失败: {}", e)))?;

        let now = chrono::Utc::now();
        let trend_label = match resp.trend.as_str() {
            "上升" => "up",
            "平稳" => "flat",
            "下降" => "down",
            _ => "nodata",
        };
        let risk_label = match resp.risk_level.as_str() {
            "高" => "high",
            "中" => "medium",
            _ => "low",
        };

        let active = QualityActiveModel {
            request_id: Set(request_id),
            product_id: Set(resp.product_id.map(|i| i as i64)),
            inspection_type: Set(resp.inspection_type.clone()),
            window_days: Set(resp.window_days),
            total_inspections: Set(resp.total_inspections),
            avg_qualification_rate: Set(rust_decimal::Decimal::from_f64_retain(
                resp.avg_qualification_rate,
            )
            .unwrap_or_default()),
            trend: Set(trend_label.to_string()),
            trend_rate: Set(
                rust_decimal::Decimal::from_f64_retain(resp.trend_rate).unwrap_or_default()
            ),
            risk_score: Set(resp.risk_score as i16),
            risk_level: Set(risk_label.to_string()),
            confidence: Set(
                rust_decimal::Decimal::from_f64_retain(resp.confidence).unwrap_or_default()
            ),
            top_issues_json: Set(Some(top_issues_json)),
            recommendations_json: Set(Some(recommendations_json)),
            period_breakdown_json: Set(Some(period_breakdown_json)),
            source: Set(resp.source.clone()),
            is_acknowledged: Set(false),
            acknowledged_at: Set(None),
            acknowledged_by: Set(None),
            // V15 P2 14-10.3：推理耗时毫秒
            inference_latency_ms: Set(Some(latency_ms)),
            created_by: Set(dto.operator_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok((resp, model.id))
    }

    /// 质量预测列表查询（V15 P0-S27：注入行级数据权限过滤。）
    pub async fn list_quality_predictions(
        &self,
        q: ListQualityPredQuery,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<QualityPredListVo, AppError> {
        let page = q.page.unwrap_or(1).clamp(1, 1000);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = QualityEntity::find();
        if let Some(pid) = q.product_id {
            select = select.filter(QualityColumn::ProductId.eq(pid));
        }
        if let Some(t) = &q.inspection_type {
            select = select.filter(QualityColumn::InspectionType.eq(t));
        }
        if let Some(r) = &q.risk_level {
            select = select.filter(QualityColumn::RiskLevel.eq(r));
        }
        if let Some(a) = q.is_acknowledged {
            select = select.filter(QualityColumn::IsAcknowledged.eq(a));
        }
        // V15 P0-S27：注入数据范围过滤
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                QualityColumn::CreatedBy,
                QualityColumn::CreatedBy,
            );
        }

        let total = select.clone().count(&*self.db).await?;
        let items = select
            .order_by_desc(QualityColumn::CreatedAt)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok(QualityPredListVo {
            items,
            total,
            page,
            page_size,
        })
    }

    /// 质量预测详情（V15 P0-S27：注入归属校验（IDOR 防护）。）
    pub async fn get_quality_prediction(
        &self,
        id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<QualityModel, AppError> {
        let model = QualityEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量预测记录不存在: id={}", id)))?;
        // V15 P0-S27：校验归属
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by.map(|i| i as i32), None) {
                return Err(AppError::permission_denied("无权访问该质量预测记录"));
            }
        }
        Ok(model)
    }

    /// 按产品查询质量预测历史（V15 P0-S27：注入行级数据权限过滤。）
    pub async fn list_quality_predictions_by_product(
        &self,
        product_id: i64,
        limit: u64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<QualityModel>, AppError> {
        let mut select = QualityEntity::find().filter(QualityColumn::ProductId.eq(product_id));
        // V15 P0-S27：注入数据范围过滤
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                QualityColumn::CreatedBy,
                QualityColumn::CreatedBy,
            );
        }
        let items = select
            .order_by_desc(QualityColumn::CreatedAt)
            .limit(limit.min(50))
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 标记质量预测已确认（V15 P0-S27：写操作前校验资源归属（防 IDOR）。）
    pub async fn acknowledge_quality_prediction(
        &self,
        id: i64,
        dto: AcknowledgeQualityPredDto,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<QualityModel, AppError> {
        let model = QualityEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量预测记录不存在: id={}", id)))?;
        // V15 P0-S27：校验归属
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by.map(|i| i as i32), None) {
                return Err(AppError::permission_denied("无权操作该质量预测记录"));
            }
        }

        let now = chrono::Utc::now();
        let mut active: QualityActiveModel = model.into();
        active.is_acknowledged = Set(true);
        active.acknowledged_at = Set(Some(now));
        active.acknowledged_by = Set(dto.operator_id);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除质量预测记录（V15 P0-S27：写操作前校验资源归属（防 IDOR）。）
    pub async fn delete_quality_prediction(
        &self,
        id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(), AppError> {
        // V15 P0-S27：先查记录校验归属，再删除
        let model = QualityEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量预测记录不存在: id={}", id)))?;
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by.map(|i| i as i32), None) {
                return Err(AppError::permission_denied("无权删除该质量预测记录"));
            }
        }
        let res = QualityEntity::delete_many()
            .filter(QualityColumn::Id.eq(id))
            .exec(&*self.db)
            .await?;
        if res.rows_affected == 0 {
            return Err(AppError::not_found(format!(
                "质量预测记录不存在: id={}",
                id
            )));
        }
        Ok(())
    }

    // =====================================================
    // 看板聚合
    // =====================================================

    /// AI 概览（应用率、平均风险、最新 5 条工艺优化 + 5 条质量预测）
    /// V15 P0-S27：注入行级数据权限过滤。；看板聚合数据应受限于调用者的数据范围：销售员仅看自己创建的 AI 推理记录统计，；部门经理看本部门（AI 表无 department_id，Dept 退化为 Self），管理员看全部。
    pub async fn ai_summary(
        &self,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let (total_proc, applied_proc, knn_proc, apply_rate) =
            self.collect_process_stats(data_scope).await?;
        let (total_qual, high_risk, unack) = self.collect_quality_stats(data_scope).await?;
        let latest_proc = self.fetch_latest_process(data_scope).await?;
        let latest_qual = self.fetch_latest_quality(data_scope).await?;
        Ok(serde_json::json!({
            "process_optimization": {
                "total": total_proc,
                "applied": applied_proc,
                "knn_recommended": knn_proc,
                "apply_rate": (apply_rate * 10000.0).round() / 10000.0,
            },
            "quality_prediction": {
                "total": total_qual,
                "high_risk": high_risk,
                "unacknowledged": unack,
            },
            "latest_process_optimizations": latest_proc,
            "latest_quality_predictions": latest_qual,
        }))
    }

    /// 统计工艺优化指标（总数 / 已应用 / KNN 推荐 / 应用率），应用数据范围过滤
    async fn collect_process_stats(
        &self,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(u64, u64, u64, f64), AppError> {
        let mut base = ProcessEntity::find();
        if let Some(ctx) = data_scope {
            base = apply_data_scope(
                base,
                ctx,
                ProcessColumn::CreatedBy,
                ProcessColumn::CreatedBy,
            );
        }
        let total = base.clone().count(&*self.db).await?;
        let applied = base
            .clone()
            .filter(ProcessColumn::IsApplied.eq(true))
            .count(&*self.db)
            .await?;
        let knn = base
            .filter(ProcessColumn::Source.eq("knn"))
            .count(&*self.db)
            .await?;
        let rate = if total > 0 {
            applied as f64 / total as f64
        } else {
            0.0
        };
        Ok((total, applied, knn, rate))
    }

    /// 统计质量预测指标（总数 / 高风险 / 未确认），应用数据范围过滤
    async fn collect_quality_stats(
        &self,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(u64, u64, u64), AppError> {
        let mut base = QualityEntity::find();
        if let Some(ctx) = data_scope {
            base = apply_data_scope(
                base,
                ctx,
                QualityColumn::CreatedBy,
                QualityColumn::CreatedBy,
            );
        }
        let total = base.clone().count(&*self.db).await?;
        let high_risk = base
            .clone()
            .filter(QualityColumn::RiskLevel.eq("high"))
            .count(&*self.db)
            .await?;
        let unack = base
            .filter(QualityColumn::IsAcknowledged.eq(false))
            .count(&*self.db)
            .await?;
        Ok((total, high_risk, unack))
    }

    /// 获取最新 5 条工艺优化记录（应用数据范围过滤，按创建时间倒序）
    async fn fetch_latest_process(
        &self,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<ProcessModel>, AppError> {
        let mut q = ProcessEntity::find();
        if let Some(ctx) = data_scope {
            q = apply_data_scope(q, ctx, ProcessColumn::CreatedBy, ProcessColumn::CreatedBy);
        }
        Ok(q.order_by_desc(ProcessColumn::CreatedAt)
            .limit(5)
            .all(&*self.db)
            .await?)
    }

    /// 获取最新 5 条质量预测记录（应用数据范围过滤，按创建时间倒序）
    async fn fetch_latest_quality(
        &self,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<QualityModel>, AppError> {
        let mut q = QualityEntity::find();
        if let Some(ctx) = data_scope {
            q = apply_data_scope(q, ctx, QualityColumn::CreatedBy, QualityColumn::CreatedBy);
        }
        Ok(q.order_by_desc(QualityColumn::CreatedAt)
            .limit(5)
            .all(&*self.db)
            .await?)
    }

    /// 返回 AI 模块算法元信息（v11 批次 155 P2-C：从 handler 下沉到 service，避免描述脱钩）
    pub fn algorithm_metadata() -> serde_json::Value {
        serde_json::json!({
            "process_optimization": {
                "algorithm": "k-NN + 加权平均",
                "fallback": "典型参数表（80°C/45min/pH6.0/浴比1:8）",
            },
            "quality_prediction": {
                "algorithm": "趋势分析 + 风险评分",
                "fallback": "保守默认（合格率 95% / 置信度 0.3）",
            },
        })
    }

    // =====================================================
    // V15 P1 1.3+8.1：工艺优化→化验室打样集成
    // =====================================================

    /// V15 P1 1.3+8.1：将工艺优化推荐参数推送到化验室打样系统
    /// 自动创建打样通知单（lab_dip_request），透传 color_no/fabric_type/dye_type，；并在通知单 remarks 中记录来源 AI 工艺优化 ID 以便回溯。
    pub async fn push_to_lab_dip(
        &self,
        process_opt_id: i64,
        operator_id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<i32, AppError> {
        let model = self
            .get_process_optimization(process_opt_id, data_scope)
            .await?;

        let now = chrono::Utc::now();
        let required_date = (now + chrono::Duration::days(7)).date_naive();
        let remarks = format!(
            "AI 工艺优化推送（proc_opt_id={}, 推荐温度{:.1}°C/时间{}min/pH{:.1}/浴比1:{:.1}）",
            model.id,
            model.recommended_temperature,
            model.recommended_time_minutes,
            model.recommended_ph_value,
            model.recommended_liquor_ratio,
        );

        let create_req = CreateLabDipRequestRequest {
            customer_id: None,
            customer_color_no: Some(model.color_no.clone()),
            customer_color_name: model.color_name.clone(),
            sample_type: Some("fabric".to_string()),
            fabric_spec: Some(model.fabric_type.clone()),
            fabric_component: None,
            sample_size: Some("30x15cm".to_string()),
            light_source: "D65".to_string(),
            secondary_light_source: None,
            color_fastness_req: None,
            eco_requirement: None,
            sample_versions: Some(4),
            dye_category: model.dye_type.clone(),
            required_date,
            expected_days: Some(3),
            remarks: Some(remarks),
            created_by: Some(operator_id as i32),
        };

        let svc = LabDipRequestService::new(self.db.clone());
        let lab_dip = svc.create(create_req).await?;

        // 回写 lab_dip_request_id 到 ai_process_optimizations（暂存于 reason 字段末尾便于回溯）
        let mut active: ProcessActiveModel = model.into();
        active.reason = Set(Some(format!(
            "已推送化验室打样（lab_dip_request_id={}）",
            lab_dip.id
        )));
        active.updated_at = Set(now);
        active.update(&*self.db).await?;

        Ok(lab_dip.id)
    }

    // =====================================================
    // V15 P1 8.2：工艺优化→生产执行集成
    // =====================================================

    /// V15 P1 8.2：将工艺优化推荐参数关联到生产配方（在 ai_process_optimizations.production_recipe_id 字段写入关联，；便于追踪"AI 推荐参数是否被生产执行采纳"。）
    pub async fn link_to_production_recipe(
        &self,
        process_opt_id: i64,
        production_recipe_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<ProcessModel, AppError> {
        let model = self
            .get_process_optimization(process_opt_id, data_scope)
            .await?;

        let now = chrono::Utc::now();
        let mut active: ProcessActiveModel = model.into();
        active.production_recipe_id = Set(Some(production_recipe_id));
        active.is_applied = Set(true);
        active.applied_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    // =====================================================
    // V15 P1 2.1+8.3：质量预测实际结果回填
    // =====================================================

    /// V15 P1 2.1+8.3：回填质量预测的实际结果（来自质检记录对账）
    /// 在 ai_quality_predictions 表写入 actual_risk_level/actual_avg_qualification_rate/actual_recorded_at，；供后续准确率报告统计使用。
    pub async fn record_actual_quality_result(
        &self,
        prediction_id: i64,
        actual_risk_level: String,
        actual_avg_qualification_rate: rust_decimal::Decimal,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<QualityModel, AppError> {
        let model = self
            .get_quality_prediction(prediction_id, data_scope)
            .await?;

        let now = chrono::Utc::now();
        let mut active: QualityActiveModel = model.into();
        active.actual_risk_level = Set(Some(actual_risk_level));
        active.actual_avg_qualification_rate = Set(Some(actual_avg_qualification_rate));
        active.actual_recorded_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

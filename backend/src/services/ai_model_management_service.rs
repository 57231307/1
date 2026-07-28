//! AI 模型管理与审计服务（V15 P1 批次14）
//!
//! 涵盖缺陷：
//! - 3.1+3.4+10.2：模型版本管理 + 评估指标 + 变更审计
//! - 3.5：模型漂移检测
//! - 2.4+8.3：质量预测准确率对账报告
//! - 10.1：AI 决策审计日志专用表
//!
//! 全部走 SQL 参数化（sea-orm ColumnTrait eq/filter），写操作 Set 注入，
//! 无字符串拼接 SQL，无 #[allow(...)] 抑制。

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::ai_decision_log::{
    ActiveModel as DecisionLogActiveModel, Entity as DecisionLogEntity, Model as DecisionLogModel,
};
use crate::models::ai_model_evaluation::{
    ActiveModel as ModelEvalActiveModel, Entity as ModelEvalEntity, Model as ModelEvalModel,
};
use crate::models::ai_model_version::{
    ActiveModel as ModelVersionActiveModel, Column as ModelVersionColumn,
    Entity as ModelVersionEntity, Model as ModelVersionModel,
};
use crate::models::ai_quality_accuracy_report::{
    ActiveModel as AccuracyReportActiveModel, Entity as AccuracyReportEntity,
    Model as AccuracyReportModel,
};
use crate::models::ai_quality_prediction::{
    Column as QualityColumn, Entity as QualityEntity, Model as QualityModel,
};
use crate::utils::error::AppError;

// =====================================================
// 模型版本管理 DTO（V15 P1 3.1 + 10.2）
// =====================================================

/// 创建模型版本请求
#[derive(Debug, Deserialize)]
pub struct CreateModelVersionRequest {
    pub model_name: String,
    pub version: String,
    pub algorithm: String,
    pub parameters_json: Option<serde_json::Value>,
    pub training_date: Option<chrono::NaiveDate>,
    pub training_dataset_size: Option<i32>,
    pub accuracy_metrics_json: Option<serde_json::Value>,
    pub change_reason: Option<String>,
    pub changed_by: Option<i32>,
}

/// 审批模型版本请求
#[derive(Debug, Deserialize)]
pub struct ApproveModelVersionRequest {
    pub approved_by: i32,
    pub approval_status: String,
}

/// 模型版本状态流转请求
#[derive(Debug, Deserialize)]
pub struct ChangeModelStatusRequest {
    pub new_status: String,
    pub changed_by: Option<i32>,
    pub change_reason: Option<String>,
}

// =====================================================
// 模型评估 DTO（V15 P1 3.4）
// =====================================================

/// 创建模型评估请求
#[derive(Debug, Deserialize)]
pub struct CreateModelEvaluationRequest {
    pub model_version_id: i32,
    pub accuracy: Option<Decimal>,
    pub precision: Option<Decimal>,
    pub recall: Option<Decimal>,
    pub f1_score: Option<Decimal>,
    pub sample_count: i32,
    pub evaluation_report: Option<String>,
}

// =====================================================
// AI 决策审计日志 DTO（V15 P1 10.1）
// =====================================================

/// 创建 AI 决策日志请求
#[derive(Debug, Deserialize)]
pub struct CreateDecisionLogRequest {
    pub decision_type: String,
    pub model_version_id: Option<i32>,
    pub input_json: Option<serde_json::Value>,
    pub output_json: Option<serde_json::Value>,
    pub user_id: Option<i32>,
    pub ip_address: Option<String>,
    pub latency_ms: Option<i32>,
    pub confidence: Option<Decimal>,
    pub source: Option<String>,
    pub degraded: bool,
}

/// 决策日志查询参数
#[derive(Debug, Deserialize, Default)]
pub struct DecisionLogQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub decision_type: Option<String>,
    pub user_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct DecisionLogListVo {
    pub items: Vec<DecisionLogModel>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

// =====================================================
// Service 定义
// =====================================================

/// AI 模型管理与审计 Service
pub struct AiModelManagementService {
    db: Arc<sea_orm::DatabaseConnection>,
}

impl AiModelManagementService {
    pub fn new(db: Arc<sea_orm::DatabaseConnection>) -> Self {
        Self { db }
    }

    // ===== 模型版本管理（P1 3.1 + 10.2）=====

    /// V15 P1 3.1+10.2：注册新模型版本（默认 draft + pending 审批）
    pub async fn create_model_version(
        &self,
        req: CreateModelVersionRequest,
    ) -> Result<ModelVersionModel, AppError> {
        Self::validate_model_status("draft")?;
        Self::validate_approval_status("pending")?;

        let now = chrono::Utc::now();
        let active = ModelVersionActiveModel {
            model_name: Set(req.model_name),
            version: Set(req.version),
            algorithm: Set(req.algorithm),
            parameters_json: Set(req.parameters_json),
            training_date: Set(req.training_date),
            training_dataset_size: Set(req.training_dataset_size),
            accuracy_metrics_json: Set(req.accuracy_metrics_json),
            status: Set("draft".to_string()),
            changed_by: Set(req.changed_by),
            change_reason: Set(req.change_reason),
            approval_status: Set("pending".to_string()),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// V15 P1 3.1：列出模型版本（支持按 model_name 过滤）
    pub async fn list_model_versions(
        &self,
        model_name: Option<String>,
    ) -> Result<Vec<ModelVersionModel>, AppError> {
        let mut q = ModelVersionEntity::find();
        if let Some(name) = model_name {
            q = q.filter(ModelVersionColumn::ModelName.eq(name));
        }
        Ok(q.order_by_desc(ModelVersionColumn::CreatedAt)
            .all(&*self.db)
            .await?)
    }

    /// V15 P1 3.1：获取当前生效模型版本（status=active）
    pub async fn get_active_model_version(
        &self,
        model_name: &str,
    ) -> Result<Option<ModelVersionModel>, AppError> {
        Ok(ModelVersionEntity::find()
            .filter(ModelVersionColumn::ModelName.eq(model_name))
            .filter(ModelVersionColumn::Status.eq("active"))
            .one(&*self.db)
            .await?)
    }

    /// V15 P1 10.2：审批模型版本
    ///
    /// 状态机：pending → approved/rejected；approved 后方可激活为 active。
    pub async fn approve_model_version(
        &self,
        version_id: i32,
        req: ApproveModelVersionRequest,
    ) -> Result<ModelVersionModel, AppError> {
        Self::validate_approval_status(&req.approval_status)?;
        let model = ModelVersionEntity::find_by_id(version_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("模型版本不存在: id={}", version_id)))?;
        if model.approval_status != "pending" {
            return Err(AppError::business(format!(
                "模型版本审批状态非法：当前 {}，仅 pending 可审批",
                model.approval_status
            )));
        }

        let now = chrono::Utc::now();
        let mut active: ModelVersionActiveModel = model.into();
        active.approval_status = Set(req.approval_status);
        active.approved_by = Set(Some(req.approved_by));
        active.approved_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// V15 P1 10.2：变更模型版本状态（draft → active → retired → archived）
    ///
    /// 仅审批通过的版本可激活为 active；激活新版本时旧 active 自动降级为 retired。
    pub async fn change_model_status(
        &self,
        version_id: i32,
        req: ChangeModelStatusRequest,
    ) -> Result<ModelVersionModel, AppError> {
        Self::validate_model_status(&req.new_status)?;
        let model = ModelVersionEntity::find_by_id(version_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("模型版本不存在: id={}", version_id)))?;

        if req.new_status == "active" && model.approval_status != "approved" {
            return Err(AppError::business(
                "仅审批通过（approved）的模型版本可激活为 active",
            ));
        }

        // 激活新版本前，将同 model_name 的旧 active 版本降级为 retired
        if req.new_status == "active" {
            self.retire_previous_active_versions(&model.model_name)
                .await?;
        }

        let now = chrono::Utc::now();
        let mut active: ModelVersionActiveModel = model.into();
        active.status = Set(req.new_status);
        active.changed_by = Set(req.changed_by);
        active.change_reason = Set(req.change_reason);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 将指定 model_name 下所有 active 版本降级为 retired（激活新版本前调用）
    async fn retire_previous_active_versions(&self, model_name: &str) -> Result<(), AppError> {
        use crate::models::ai_model_version::Column;
        let olds = ModelVersionEntity::find()
            .filter(Column::ModelName.eq(model_name))
            .filter(Column::Status.eq("active"))
            .all(&*self.db)
            .await?;
        let now = chrono::Utc::now();
        for old in olds {
            let mut a: ModelVersionActiveModel = old.into();
            a.status = Set("retired".to_string());
            a.updated_at = Set(now);
            a.update(&*self.db).await?;
        }
        Ok(())
    }

    fn validate_model_status(status: &str) -> Result<(), AppError> {
        if !matches!(status, "draft" | "active" | "retired" | "archived") {
            return Err(AppError::validation(format!(
                "模型状态非法：{}，应为 draft/active/retired/archived",
                status
            )));
        }
        Ok(())
    }

    fn validate_approval_status(status: &str) -> Result<(), AppError> {
        if !matches!(status, "pending" | "approved" | "rejected") {
            return Err(AppError::validation(format!(
                "审批状态非法：{}，应为 pending/approved/rejected",
                status
            )));
        }
        Ok(())
    }

    // ===== 模型评估（P1 3.4）=====

    /// V15 P1 3.4：创建模型评估记录
    pub async fn create_model_evaluation(
        &self,
        req: CreateModelEvaluationRequest,
    ) -> Result<ModelEvalModel, AppError> {
        Self::validate_metric_range("accuracy", req.accuracy)?;
        Self::validate_metric_range("precision", req.precision)?;
        Self::validate_metric_range("recall", req.recall)?;
        Self::validate_metric_range("f1_score", req.f1_score)?;

        // 校验 model_version_id 存在
        let version = ModelVersionEntity::find_by_id(req.model_version_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("模型版本不存在: id={}", req.model_version_id))
            })?;

        let now = chrono::Utc::now();
        let active = ModelEvalActiveModel {
            model_version_id: Set(version.id),
            evaluation_date: Set(now),
            accuracy: Set(req.accuracy),
            precision: Set(req.precision),
            recall: Set(req.recall),
            f1_score: Set(req.f1_score),
            sample_count: Set(req.sample_count),
            evaluation_report: Set(req.evaluation_report),
            created_at: Set(now),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// V15 P1 3.4：列出指定模型版本的评估记录
    pub async fn list_model_evaluations(
        &self,
        model_version_id: i32,
    ) -> Result<Vec<ModelEvalModel>, AppError> {
        use crate::models::ai_model_evaluation::Column;
        Ok(ModelEvalEntity::find()
            .filter(Column::ModelVersionId.eq(model_version_id))
            .order_by_desc(Column::EvaluationDate)
            .all(&*self.db)
            .await?)
    }

    fn validate_metric_range(name: &str, value: Option<Decimal>) -> Result<(), AppError> {
        if let Some(v) = value {
            if v < Decimal::ZERO || v > Decimal::ONE {
                return Err(AppError::validation(format!(
                    "{} 取值范围 [0.0, 1.0]，当前 {}",
                    name, v
                )));
            }
        }
        Ok(())
    }

    // ===== 模型漂移检测（P1 3.5）=====

    /// V15 P1 3.5：检测模型漂移
    ///
    /// 算法：对比最近评估准确率与历史平均准确率，下降超过 5% 触发告警。
    /// 返回 (drift_detected, current_accuracy, baseline_accuracy, drift_percentage)。
    pub async fn detect_model_drift(
        &self,
        model_version_id: i32,
    ) -> Result<(bool, Option<Decimal>, Option<Decimal>, f64), AppError> {
        let evals = self.list_model_evaluations(model_version_id).await?;
        if evals.is_empty() {
            return Ok((false, None, None, 0.0));
        }
        let latest = &evals[0];
        let current = latest.accuracy;
        // baseline = 除最新外的历史平均
        let historical: Vec<Decimal> = evals.iter().skip(1).filter_map(|e| e.accuracy).collect();
        if historical.is_empty() {
            return Ok((false, current, None, 0.0));
        }
        let baseline_dec: Decimal =
            historical.iter().sum::<Decimal>() / Decimal::from(historical.len());
        let baseline = Some(baseline_dec);
        let drift_pct = if let Some(c) = current {
            let c_f = c.to_f64().unwrap_or(0.0);
            let b_f = baseline_dec.to_f64().unwrap_or(0.0);
            if b_f > 0.0 {
                (c_f - b_f) / b_f * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        // 下降超过 5% 触发漂移告警
        let drift_detected = drift_pct < -5.0;
        Ok((drift_detected, current, baseline, drift_pct))
    }

    // ===== AI 决策审计日志（P1 10.1）=====

    /// V15 P1 10.1：记录 AI 决策日志（异步调用，不阻塞主流程）
    pub async fn log_decision(
        &self,
        req: CreateDecisionLogRequest,
    ) -> Result<DecisionLogModel, AppError> {
        Self::validate_decision_type(&req.decision_type)?;
        let now = chrono::Utc::now();
        let active = DecisionLogActiveModel {
            decision_type: Set(req.decision_type),
            model_version_id: Set(req.model_version_id),
            input_json: Set(req.input_json),
            output_json: Set(req.output_json),
            user_id: Set(req.user_id),
            ip_address: Set(req.ip_address),
            latency_ms: Set(req.latency_ms),
            confidence: Set(req.confidence),
            source: Set(req.source),
            degraded: Set(req.degraded),
            created_at: Set(now),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// V15 P1 10.1：查询 AI 决策日志
    pub async fn list_decision_logs(
        &self,
        q: DecisionLogQuery,
    ) -> Result<DecisionLogListVo, AppError> {
        use crate::models::ai_decision_log::Column;
        let page = q.page.unwrap_or(1).clamp(1, 1000);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = DecisionLogEntity::find();
        if let Some(dt) = &q.decision_type {
            select = select.filter(Column::DecisionType.eq(dt));
        }
        if let Some(uid) = q.user_id {
            select = select.filter(Column::UserId.eq(uid));
        }
        let total = select.clone().count(&*self.db).await?;
        let items = select
            .order_by_desc(Column::CreatedAt)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;
        Ok(DecisionLogListVo {
            items,
            total,
            page,
            page_size,
        })
    }

    fn validate_decision_type(dt: &str) -> Result<(), AppError> {
        let valid = matches!(
            dt,
            "process_optimization"
                | "quality_prediction"
                | "sales_forecast"
                | "inventory_optimization"
                | "anomaly_detection"
                | "recommendation"
        );
        if !valid {
            return Err(AppError::validation(format!("decision_type 非法：{}", dt)));
        }
        Ok(())
    }
}

// =====================================================
// AI 质量预测准确率对账服务（V15 P1 2.4 + 8.3）
// =====================================================

/// AI 质量预测准确率对账 Service
pub struct AiQualityReconciliationService {
    db: Arc<sea_orm::DatabaseConnection>,
}

/// 对账结果汇总
#[derive(Debug, Serialize)]
pub struct ReconciliationResult {
    pub report_period: String,
    pub total_predictions: i32,
    pub correct_predictions: i32,
    pub accuracy_rate: Option<Decimal>,
    pub precision_score: Option<Decimal>,
    pub recall_score: Option<Decimal>,
    pub f1_score: Option<Decimal>,
    pub accuracy_below_threshold: bool,
}

/// 准确率阈值（P1 2.1 要求 ≥ 80%）
pub const ACCURACY_THRESHOLD: f64 = 0.80;

impl AiQualityReconciliationService {
    pub fn new(db: Arc<sea_orm::DatabaseConnection>) -> Self {
        Self { db }
    }

    /// V15 P1 2.4+8.3：按月对账质量预测与实际结果
    ///
    /// 1. 拉取上月 ai_quality_predictions（含 actual_* 字段已回填的记录）
    /// 2. 对比预测 risk_level 与 actual_risk_level，相同视为正确
    /// 3. 计算 accuracy/precision/recall/F1（多分类 One-vs-Rest 简化为高/中/低三档）
    /// 4. 写入 ai_quality_accuracy_reports 表
    /// 5. 准确率 < 80% 时 accuracy_below_threshold=true 供告警使用
    pub async fn reconcile_monthly(
        &self,
        report_period: String,
    ) -> Result<ReconciliationResult, AppError> {
        let predictions = QualityEntity::find()
            .filter(QualityColumn::ActualRiskLevel.is_not_null())
            .all(&*self.db)
            .await?;

        let total = predictions.len() as i32;
        if total == 0 {
            return Ok(ReconciliationResult {
                report_period,
                total_predictions: 0,
                correct_predictions: 0,
                accuracy_rate: None,
                precision_score: None,
                recall_score: None,
                f1_score: None,
                accuracy_below_threshold: false,
            });
        }

        let (correct, tp, fp, fn_) = Self::compute_confusion_matrix(&predictions);
        let accuracy = Decimal::from(correct) / Decimal::from(total);
        let precision = if tp + fp > 0 {
            Some(Decimal::from(tp) / Decimal::from(tp + fp))
        } else {
            None
        };
        let recall = if tp + fn_ > 0 {
            Some(Decimal::from(tp) / Decimal::from(tp + fn_))
        } else {
            None
        };
        let f1 = match (precision, recall) {
            (Some(p), Some(r)) => {
                let p_f = p.to_f64().unwrap_or(0.0);
                let r_f = r.to_f64().unwrap_or(0.0);
                if p_f + r_f > 0.0 {
                    Decimal::from_f64_retain(2.0 * p_f * r_f / (p_f + r_f))
                } else {
                    None
                }
            }
            _ => None,
        };

        let accuracy_f = accuracy.to_f64().unwrap_or(0.0);
        let below_threshold = accuracy_f < ACCURACY_THRESHOLD;

        // 持久化报告（upsert by report_period）
        Self::upsert_report(
            &self.db,
            &report_period,
            total,
            correct,
            accuracy,
            precision,
            recall,
            f1,
        )
        .await?;

        Ok(ReconciliationResult {
            report_period,
            total_predictions: total,
            correct_predictions: correct,
            accuracy_rate: Some(accuracy),
            precision_score: precision,
            recall_score: recall,
            f1_score: f1,
            accuracy_below_threshold: below_threshold,
        })
    }

    /// 计算混淆矩阵（high 风险为正类）
    fn compute_confusion_matrix(predictions: &[QualityModel]) -> (i32, i32, i32, i32) {
        let mut correct = 0;
        let mut tp = 0;
        let mut fp = 0;
        let mut fn_ = 0;
        for p in predictions {
            let predicted = Self::normalize_risk(&p.risk_level);
            let actual = Self::normalize_risk(p.actual_risk_level.as_deref().unwrap_or(""));
            if predicted == actual {
                correct += 1;
            }
            // 正类 = high
            match (predicted.as_str(), actual.as_str()) {
                ("high", "high") => tp += 1,
                ("high", _) => fp += 1,
                (_, "high") => fn_ += 1,
                _ => {}
            }
        }
        (correct, tp, fp, fn_)
    }

    /// 归一化风险等级为 high/medium/low
    fn normalize_risk(level: &str) -> String {
        match level.to_lowercase().as_str() {
            "high" | "高" => "high".to_string(),
            "medium" | "中" => "medium".to_string(),
            "low" | "低" => "low".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// upsert 准确率报告（同 report_period 覆盖）
    async fn upsert_report(
        db: &sea_orm::DatabaseConnection,
        report_period: &str,
        total: i32,
        correct: i32,
        accuracy: Decimal,
        precision: Option<Decimal>,
        recall: Option<Decimal>,
        f1: Option<Decimal>,
    ) -> Result<(), AppError> {
        use crate::models::ai_quality_accuracy_report::Column;
        let existing = AccuracyReportEntity::find()
            .filter(Column::ReportPeriod.eq(report_period))
            .one(db)
            .await?;

        let now = chrono::Utc::now();
        if let Some(model) = existing {
            let mut active: AccuracyReportActiveModel = model.into();
            active.total_predictions = Set(total);
            active.correct_predictions = Set(correct);
            active.accuracy_rate = Set(Some(accuracy));
            active.precision_score = Set(precision);
            active.recall_score = Set(recall);
            active.f1_score = Set(f1);
            active.generated_at = Set(now);
            active.update(db).await?;
        } else {
            let active = AccuracyReportActiveModel {
                report_period: Set(report_period.to_string()),
                total_predictions: Set(total),
                correct_predictions: Set(correct),
                accuracy_rate: Set(Some(accuracy)),
                precision_score: Set(precision),
                recall_score: Set(recall),
                f1_score: Set(f1),
                mismatch_cases_json: Set(None),
                generated_at: Set(now),
                created_at: Set(now),
                ..Default::default()
            };
            active.insert(db).await?;
        }
        Ok(())
    }

    /// 查询准确率报告历史
    pub async fn list_accuracy_reports(
        &self,
        limit: u64,
    ) -> Result<Vec<AccuracyReportModel>, AppError> {
        use crate::models::ai_quality_accuracy_report::Column;
        Ok(AccuracyReportEntity::find()
            .order_by_desc(Column::ReportPeriod)
            .limit(limit.min(50))
            .all(&*self.db)
            .await?)
    }
}

// =====================================================
// 单元测试
// =====================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_model_status_valid() {
        assert!(AiModelManagementService::validate_model_status("draft").is_ok());
        assert!(AiModelManagementService::validate_model_status("active").is_ok());
        assert!(AiModelManagementService::validate_model_status("retired").is_ok());
        assert!(AiModelManagementService::validate_model_status("archived").is_ok());
    }

    #[test]
    fn test_validate_model_status_invalid() {
        assert!(AiModelManagementService::validate_model_status("invalid").is_err());
        assert!(AiModelManagementService::validate_model_status("").is_err());
    }

    #[test]
    fn test_validate_approval_status_valid() {
        assert!(AiModelManagementService::validate_approval_status("pending").is_ok());
        assert!(AiModelManagementService::validate_approval_status("approved").is_ok());
        assert!(AiModelManagementService::validate_approval_status("rejected").is_ok());
    }

    #[test]
    fn test_validate_decision_type_valid() {
        assert!(AiModelManagementService::validate_decision_type("process_optimization").is_ok());
        assert!(AiModelManagementService::validate_decision_type("quality_prediction").is_ok());
        assert!(AiModelManagementService::validate_decision_type("sales_forecast").is_ok());
    }

    #[test]
    fn test_validate_decision_type_invalid() {
        assert!(AiModelManagementService::validate_decision_type("invalid_type").is_err());
    }

    #[test]
    fn test_normalize_risk() {
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("high"),
            "high"
        );
        assert_eq!(AiQualityReconciliationService::normalize_risk("高"), "high");
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("medium"),
            "medium"
        );
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("中"),
            "medium"
        );
        assert_eq!(AiQualityReconciliationService::normalize_risk("low"), "low");
        assert_eq!(AiQualityReconciliationService::normalize_risk("低"), "low");
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("unknown_val"),
            "unknown"
        );
    }

    #[test]
    fn test_validate_metric_range() {
        let svc = AiModelManagementService::new(std::sync::Arc::new(
            sea_orm::DatabaseConnection::default(),
        ));
        let _ = svc; // 抑制 unused 警告
        assert!(AiModelManagementService::validate_metric_range(
            "accuracy",
            Some(Decimal::new(85, 2))
        )
        .is_ok());
        assert!(AiModelManagementService::validate_metric_range("accuracy", None).is_ok());
        assert!(AiModelManagementService::validate_metric_range(
            "accuracy",
            Some(Decimal::new(150, 2))
        )
        .is_err());
        assert!(AiModelManagementService::validate_metric_range(
            "accuracy",
            Some(Decimal::new(-5, 2))
        )
        .is_err());
    }
}

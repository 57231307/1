//! AI 质量预测服务（ai/quality_pred）
//!
//! 基于 `quality_inspection_records` 历史数据，向质量管理员输出
//! 产品级别的合格率趋势 / 风险评分 / 主要问题归因 / 建议措施。
//!
//! 算法概要：
//! 1. 按 `product_id` / `inspection_type` 过滤最近 N 天（默认 90 天）的检验记录
//! 2. 聚合得到平均合格率（按记录自身 `qualification_rate`，缺失时回退到
//!    `qualified_qty / inspected_qty`）；再按月分段生成 `period_breakdown`
//! 3. 趋势判定：最近 30 天 vs 之前 30 天移动平均
//!    - 变化率 > +5%   → 上升
//!    - 变化率 < -5%   → 下降
//!    - 其他           → 平稳
//!    - 样本不足       → 无数据
//! 4. 风险评分：
//!    `risk = (100 - avg_rate) * 0.6 + trend_penalty * 0.4`
//!    其中下降趋势额外 +15 分；最终 0-100 区间
//! 5. 问题归因：从 `remark` 字段关键词频次提取 top 3
//! 6. 建议措施：按风险等级（低/中/高）分档生成
//! 7. 退化路径：历史数据 < 5 条时使用保守默认值
//!    - 合格率 95%
//!    - 置信度 0.3
//!    - 风险等级：中
//!
//! 模块内拆出多个纯函数（`compute_risk_score` / `compute_trend` /
//! `classify_trend` / `classify_risk_level` / `compute_confidence` /
//! `extract_issue_keyword`），单元测试可直接调用，避免依赖数据库。

use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::models::quality_inspection_record::{
    Entity as QualityInspectionEntity, Model as QualityInspectionModel,
};
use crate::utils::error::AppError;

use super::{AiAnalysisService, mean};

/// 因子贡献（V15 P2 14.7.1：解释各评分因子的权重与贡献）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorContribution {
    pub factor_name: String,
    pub weight: f64,
    pub contribution: String,
}

// =====================================================
// 输入 / 输出 DTO
// =====================================================

/// 质量预测请求
#[derive(Debug, Clone, Deserialize, Default)]
pub struct QualityPredRequest {
    /// 可选：限定产品 ID
    pub product_id: Option<i32>,
    /// 可选：限定检验类型（进货/过程/成品/出货 等）
    pub inspection_type: Option<String>,
    /// 可选：时间窗口天数（默认 90 天，1-365）
    pub window_days: Option<i32>,
    /// V15 P1 2.2：染料类型特征（活性/分散/酸性/还原等）
    pub dye_type: Option<String>,
    /// V15 P1 2.2：助剂类型特征（可选）
    pub auxiliary_type: Option<String>,
    /// V15 P1 2.2：温度范围特征（°C，可选，如 [60.0, 100.0] 表示 60-100°C）
    pub temperature_range: Option<(f64, f64)>,
    /// V15 P1 2.2：缸号特征（可选，按缸号过滤）
    pub batch_no: Option<String>,
    /// V15 P1 2.2：胚布来源特征（可选）
    pub fabric_source: Option<String>,
}

/// 质量问题归因
#[derive(Debug, Clone, Serialize)]
pub struct QualityIssue {
    /// 问题类型（关键词归类：颜色差异 / 色牢度 / 克重 / 纬密 / 强度 / 其他）
    pub issue_type: String,
    /// 出现次数
    pub occurrences: i64,
    /// 占总不合格记录比例（百分比 0-100）
    pub percentage: f64,
}

/// 周期统计段
#[derive(Debug, Clone, Serialize)]
pub struct PeriodStat {
    /// 周期标签（"YYYY-MM"）
    pub period: String,
    /// 检验次数
    pub inspections: i64,
    /// 周期内平均合格率（百分比 0-100）
    pub avg_qualification_rate: f64,
}

/// 质量预测响应
#[derive(Debug, Clone, Serialize)]
pub struct QualityPredResponse {
    /// 入参产品 ID（透传，未指定为 None）
    pub product_id: Option<i32>,
    /// 实际生效的检验类型（"all" 表示未限定）
    pub inspection_type: String,
    /// 实际生效的时间窗口天数
    pub window_days: i32,
    /// 有效历史检验记录数
    pub total_inspections: i64,
    /// 平均合格率（百分比 0-100）
    pub avg_qualification_rate: f64,
    /// 趋势："上升" | "平稳" | "下降" | "无数据"
    pub trend: String,
    /// 趋势变化率（百分点，正数上升 / 负数下降）
    pub trend_rate: f64,
    /// 风险评分（0-100，越高越危险）
    pub risk_score: u32,
    /// 风险等级："低" | "中" | "高"
    pub risk_level: String,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 主要问题归因（top 3）
    pub top_issues: Vec<QualityIssue>,
    /// 建议措施（按风险等级生成）
    pub recommendations: Vec<String>,
    /// 按月分段统计
    pub period_breakdown: Vec<PeriodStat>,
    /// 数据来源标识："history" | "fallback"
    pub source: String,
    /// V15 P1 5.3：是否命中缓存（true 表示 5 分钟内相同入参已计算过）
    pub cache_hit: bool,
    /// V15 P1 9.1+9.5：是否为降级结果（true 表示推理超时或模型不可用时返回的兜底结果）
    pub degraded: bool,
    /// 人类可读的预测结果解释（V15 P2 14.1.71：说明预测依据和关键因素）
    pub explanation: Option<String>,
    /// V15 P2 14.7.1：评分因子贡献列表
    pub factors: Vec<FactorContribution>,
}

// =====================================================
// 内部纯函数（不依赖数据库，可直接单测）
// =====================================================

/// 风险评分最大理论值
pub(crate) const RISK_MAX: f64 = 100.0;
/// 风险评分中"平均合格率"权重
pub(crate) const RISK_WEIGHT_RATE: f64 = 0.6;
/// 风险评分中"下降趋势"权重
pub(crate) const RISK_WEIGHT_TREND: f64 = 0.4;
/// 下降趋势额外惩罚分（最大分）
pub(crate) const TREND_DOWN_PENALTY: f64 = 15.0;
/// 趋势判定阈值（百分点），变化率超过 ±5% 即认为显著
pub(crate) const TREND_THRESHOLD: f64 = 0.05;
/// 退化路径历史最少记录数
pub(crate) const MIN_HISTORY_RECORDS: i64 = 5;
/// 退化路径默认合格率（百分比）
pub(crate) const FALLBACK_QUALIFICATION_RATE: f64 = 95.0;
/// 退化路径默认置信度
pub const FALLBACK_CONFIDENCE: f64 = 0.3;
/// 风险等级阈值（≥ 高 / < 高 且 ≥ 中 / < 中）
pub(crate) const RISK_LEVEL_HIGH: f64 = 60.0;
pub(crate) const RISK_LEVEL_MEDIUM: f64 = 30.0;
/// 置信度上限对应的样本量（达到该样本数置信度封顶）
pub(crate) const CONFIDENCE_FULL_SAMPLE: i64 = 30;
/// V15 P1 6.2：质量预测记录上限（数据最小化，防止全表扫描 OOM）
pub(crate) const QUALITY_RECORD_LIMIT: u64 = 50_000;

/// 质量归因关键词库（中文常用术语）
/// 提取自 `remark` 字段，按出现频次归类问题类型。；颜色差异：颜色偏差 / 偏色 / 颜色不符 / 异色；色牢度：色牢度 / 褪色 / 沾色 / 耐洗；克重：克重 / 平米克重；纬密：纬密 / 密度 / 经密；强度：强度 / 强力 / 断裂；其他：未命中关键词的记录统一归为"其他"
const ISSUE_KEYWORDS: &[(&str, &[&str])] = &[
    ("颜色差异", &["颜色", "偏色", "异色", "色差", "色不符"]),
    ("色牢度", &["色牢度", "褪色", "沾色", "耐洗"]),
    ("克重偏差", &["克重", "平米克重"]),
    ("纬密偏差", &["纬密", "密度", "经密"]),
    ("强度不足", &["强度", "强力", "断裂"]),
];

/// defect_type → 标准归因类别的映射
/// A.15.3：质检记录新增结构化 `defect_type` 字段后，归因优先用该字段，
/// 避免依赖 remark 关键词匹配的不确定性。映射关系与 remark 关键词输出保持一致：
/// color_diff → "颜色差异"；color_fastness → "色牢度"；spec → "规格不符"；
/// damage → "破损"；other → "其他"；未识别值 → "其他"。
fn map_defect_type_to_issue(defect_type: &str) -> String {
    match defect_type {
        "color_diff" => "颜色差异".to_string(),
        "color_fastness" => "色牢度".to_string(),
        "spec" => "规格不符".to_string(),
        "damage" => "破损".to_string(),
        // other 及未识别值统一归为"其他"
        _ => "其他".to_string(),
    }
}

/// 从质检记录提取问题类型归因（A.15.3：优先用结构化 `defect_type` 字段映射，
/// 缺失或为空时降级回退到 `remark` 关键词匹配，保证既有逻辑兼容。）
/// - `defect_type`：质检记录的结构化缺陷类型（color_diff / color_fastness /
///   spec / damage / other），有值且非空时直接映射到标准归因类别，跳过 remark 匹配。
/// - `remark`：备注文本，仅在 `defect_type` 为 None 或空串时用于关键词匹配兜底。
/// 返回归因类别："颜色差异"/"色牢度"/"克重偏差"/"纬密偏差"/"强度不足"/"规格不符"/"破损"/"其他"。
pub fn extract_issue_keyword(
    defect_type: Option<&str>,
    remark: Option<&str>,
) -> String {
    // 优先使用结构化 defect_type（非空时直接映射，跳过 remark 关键词匹配）
    if let Some(dt) = defect_type {
        let dt = dt.trim();
        if !dt.is_empty() {
            return map_defect_type_to_issue(dt);
        }
    }
    // 降级兜底：defect_type 为 None 或空串时，回退到 remark 关键词匹配
    let text = match remark {
        Some(t) => t,
        None => return "其他".to_string(),
    };
    for (label, kws) in ISSUE_KEYWORDS {
        for kw in *kws {
            if text.contains(kw) {
                return (*label).to_string();
            }
        }
    }
    "其他".to_string()
}

/// 风险评分计算
/// 公式：`risk = (100 - avg_rate) * 0.6 + trend_down * 0.4`；`avg_rate`      当前平均合格率（百分比 0-100）；`trend_is_down` 是否处于下降趋势；输出 0-100，越高越危险。
pub fn compute_risk_score(avg_rate: f64, trend_is_down: bool) -> f64 {
    let rate_part = ((100.0 - avg_rate).max(0.0) * RISK_WEIGHT_RATE).min(60.0);
    let trend_part = if trend_is_down {
        TREND_DOWN_PENALTY * RISK_WEIGHT_TREND
    } else {
        0.0
    };
    (rate_part + trend_part).clamp(0.0, RISK_MAX)
}

/// 风险等级分类（`score >= 60`        → "高"；`30 <= score < 60`   → "中"；`score < 30`         → "低"）
pub fn classify_risk_level(score: f64) -> String {
    if score >= RISK_LEVEL_HIGH {
        "高".to_string()
    } else if score >= RISK_LEVEL_MEDIUM {
        "中".to_string()
    } else {
        "低".to_string()
    }
}

/// 趋势判定（基于变化率）（`rate >  5%`  → "上升"；`rate < -5%`  → "下降"；其他          → "平稳"）
pub fn classify_trend(rate: f64) -> String {
    if rate > TREND_THRESHOLD {
        "上升".to_string()
    } else if rate < -TREND_THRESHOLD {
        "下降".to_string()
    } else {
        "平稳".to_string()
    }
}

/// 趋势变化率计算
/// `recent` / `previous` 分别是最近 30 天 / 之前 30 天的平均合格率（百分比）。；返回 `(recent - previous) / previous`（previous=0 时返回 0.0 兜底）。
pub fn compute_trend_rate(recent: f64, previous: f64) -> f64 {
    if previous.abs() < 0.0001 {
        return 0.0;
    }
    (recent - previous) / previous
}

/// 置信度计算（公式：`min(sample_count / CONFIDENCE_FULL_SAMPLE, 1.0)`，四舍五入到 0.01。；退化路径由调用方传入固定 0.3。）
pub fn compute_confidence(sample_count: i64) -> f64 {
    if sample_count <= 0 {
        return FALLBACK_CONFIDENCE;
    }
    let ratio = (sample_count as f64 / CONFIDENCE_FULL_SAMPLE as f64).min(1.0);
    (ratio * 100.0).round() / 100.0
}

/// V15 P2 14.1.71：构建人类可读的预测结果解释
/// 汇总风险等级、趋势、合格率、主要问题和样本量生成一段说明文本。
fn build_explanation(
    risk_level: &str,
    trend: &str,
    avg_rate: f64,
    top_issues: &[QualityIssue],
    sample_count: usize,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "基于 {} 条历史检验记录，平均合格率 {:.1}%，风险等级为「{}」。",
        sample_count, avg_rate, risk_level
    ));
    if trend != "无数据" && trend != "平稳" {
        parts.push(format!("近期趋势为「{}」。", trend));
    }
    if !top_issues.is_empty() {
        let issue_names: Vec<&str> = top_issues.iter().map(|i| i.issue_type.as_str()).collect();
        parts.push(format!("主要问题类型：{}。", issue_names.join("、")));
    }
    parts.join("")
}

/// 风险等级 → 建议措施（严格按等级分档生成 1-3 条建议，确保 UI 列表非空。）
pub fn build_recommendations(level: &str) -> Vec<String> {
    match level {
        "高" => vec![
            "立即启动专项整改，召集工艺/质量/生产三方联合复盘".to_string(),
            "对近 30 天不合格批次执行 100% 复检并隔离处置".to_string(),
            "排查原料 / 设备 / 工艺参数异常点，更新控制计划".to_string(),
        ],
        "中" => vec![
            "加强抽检频次（建议从 1 次/周提升至 2-3 次/周）".to_string(),
            "重点关注最近 30 天趋势下降的产品，制定预防措施".to_string(),
            "对主要问题归因（top 1）开展专项分析".to_string(),
        ],
        _ => vec![
            "保持现有检验频次，持续监测合格率波动".to_string(),
            "每月汇总质量数据，更新风险等级评估".to_string(),
        ],
    }
}

/// 计算给定一组记录的平均合格率（优先使用记录自身的 `qualification_rate`（百分比 0-100）；缺失时回退到 `qualified_qty / inspected_qty`。；返回百分比 0-100。）
pub fn mean_qualification_rate(records: &[QualityInspectionModel]) -> f64 {
    if records.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    let mut count = 0_i64;
    for r in records {
        let rate = r
            .qualification_rate
            .as_ref()
            .and_then(|d| d.to_f64())
            .or_else(|| {
                let inspected = r.inspected_qty.to_f64().unwrap_or(0.0);
                let qualified = r
                    .qualified_qty
                    .as_ref()
                    .and_then(|d| d.to_f64())
                    .unwrap_or(0.0);
                if inspected > 0.0 {
                    Some((qualified / inspected) * 100.0)
                } else {
                    None
                }
            });
        if let Some(v) = rate {
            sum += v;
            count += 1;
        }
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

/// 保留 2 位小数
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// =====================================================
// Service 实现
// =====================================================

impl AiAnalysisService {
    /// 质量预测主入口：标准化 → 拉取 → 聚合或退化
    /// V15 P1 5.2：通过 Semaphore permits=10 限制并发，防止 CPU 过载；V15 P1 5.3：通过 moka 缓存（TTL 5min）避免相同入参重复计算；V15 P1 5.1+9.1+9.5：通过 tokio::time::timeout（2s）包装算法执行，；超时或模型不可用时返回降级结果（保守默认值 + degraded=true）。
    pub async fn predict_quality(
        &self,
        request: QualityPredRequest,
    ) -> Result<QualityPredResponse, AppError> {
        // V15 P1 5.3：缓存键 = 入参指纹，命中时直接返回（cache_hit=true）
        let cache_key = build_quality_cache_key(&request);
        if let Some(mut cached) = self.quality_cache.get(&cache_key).await {
            cached.cache_hit = true;
            return Ok(cached);
        }

        // V15 P1 5.2：获取并发许可，permit 在 scope 结束时自动释放
        let _permit = self.acquire_inference_permit().await?;

        let params = normalize_pred_params(request);

        // V15 P1 5.1+9.5：算法执行包装在 timeout 中，超时返回降级结果
        let timeout_dur = std::time::Duration::from_millis(super::AI_INFERENCE_TIMEOUT_MS);
        let inference_result = tokio::time::timeout(
            timeout_dur,
            self.run_quality_inference(&params, cache_key.clone()),
        )
        .await;

        match inference_result {
            Ok(Ok(response)) => Ok(response),
            // V15 P1 9.1：模型不可用（DB 错误等）→ 返回降级结果
            Ok(Err(_e)) => {
                tracing::warn!("AI 质量预测模型不可用，返回降级结果: {:?}", _e);
                let degraded = build_degraded_response(
                    params.product_id,
                    &params.type_label,
                    params.window_days,
                    "AI 推理模型不可用，已降级为保守默认值".to_string(),
                );
                Ok(degraded)
            }
            // V15 P1 5.1+9.5：推理超时 → 返回降级结果
            Err(_elapsed) => {
                tracing::warn!(
                    "AI 质量预测推理超时（>{}ms），返回降级结果",
                    super::AI_INFERENCE_TIMEOUT_MS
                );
                let degraded = build_degraded_response(
                    params.product_id,
                    &params.type_label,
                    params.window_days,
                    format!(
                        "AI 推理超时（>{}ms），已降级为保守默认值",
                        super::AI_INFERENCE_TIMEOUT_MS
                    ),
                );
                Ok(degraded)
            }
        }
    }

    /// V15 P1 5.1：实际算法执行（记录拉取 + 聚合 + 响应构建 + 缓存写入）（由 `predict_quality` 通过 `tokio::time::timeout` 包装调用，超时由外层处理。）
    async fn run_quality_inference(
        &self,
        params: &NormalizedPredParams,
        cache_key: String,
    ) -> Result<QualityPredResponse, AppError> {
        let records = self.fetch_quality_records(params).await?;

        let response = if (records.len() as i64) < MIN_HISTORY_RECORDS {
            build_fallback_response(params.product_id, &params.type_label, params.window_days)
        } else {
            build_history_response((*params).clone(), &records)
        };
        self.quality_cache.insert(cache_key, response.clone()).await;
        Ok(response)
    }

    /// 拉取指定时间窗口内的全部质量检验记录
    /// V15 P1 2.2：按产品/检验类型/染料/助剂/温度/缸号/胚布来源可选过滤；V15 P1 6.2：限制记录上限（QUALITY_RECORD_LIMIT）防止全表扫描 OOM；V15 P1 6.1：返回前对 remark 字段做 PII 脱敏，避免 top_issues_json 泄露客户信息；时间下界为 `today - window_days`。
    async fn fetch_quality_records(
        &self,
        params: &NormalizedPredParams,
    ) -> Result<Vec<QualityInspectionModel>, AppError> {
        let cutoff =
            chrono::Utc::now().date_naive() - chrono::Duration::days(params.window_days as i64);

        let mut select = QualityInspectionEntity::find()
            .filter(crate::models::quality_inspection_record::Column::InspectionDate.gte(cutoff));
        if let Some(pid) = params.product_id {
            select =
                select.filter(crate::models::quality_inspection_record::Column::ProductId.eq(pid));
        }
        if let Some(t) = params.inspection_type.as_deref() {
            select = select
                .filter(crate::models::quality_inspection_record::Column::InspectionType.eq(t));
        }
        // V15 P1 2.2：面料行业特征过滤
        if let Some(dye) = params.dye_type.as_deref() {
            select =
                select.filter(crate::models::quality_inspection_record::Column::DyeType.eq(dye));
        }
        if let Some(aux) = params.auxiliary_type.as_deref() {
            select = select
                .filter(crate::models::quality_inspection_record::Column::AuxiliaryType.eq(aux));
        }
        if let Some(batch) = params.batch_no.as_deref() {
            select =
                select.filter(crate::models::quality_inspection_record::Column::BatchNo.eq(batch));
        }
        if let Some(src) = params.fabric_source.as_deref() {
            select = select
                .filter(crate::models::quality_inspection_record::Column::FabricSource.eq(src));
        }
        if let Some((lo, hi)) = params.temperature_range {
            let lo_dec = rust_decimal::Decimal::from_f64_retain(lo).unwrap_or_default();
            let hi_dec = rust_decimal::Decimal::from_f64_retain(hi).unwrap_or_default();
            select = select
                .filter(crate::models::quality_inspection_record::Column::Temperature.gte(lo_dec))
                .filter(crate::models::quality_inspection_record::Column::Temperature.lte(hi_dec));
        }

        let records = select.limit(QUALITY_RECORD_LIMIT).all(&*self.db).await?;
        // V15 P1 6.1：对 remark 字段做 PII 脱敏（手机号/邮箱/身份证号），
        // 关键词匹配在脱敏后的文本上执行，不影响归因准确性
        Ok(records
            .into_iter()
            .map(|mut r| {
                if let Some(remark) = r.remark.take() {
                    r.remark = Some(crate::utils::field_mask::mask_text_pii(&remark));
                }
                r
            })
            .collect())
    }
}

// =====================================================
// predict_quality 内部辅助函数（不依赖数据库，可直接单测）
// =====================================================

/// `predict_quality` 入参标准化后的上下文
/// 封装 `window_days` / `inspection_type` / `product_id` / `type_label` 及面料特征，；避免主函数散落局部变量；参考已有 `WageTotals` / `ApproveContext` 模式。
#[derive(Clone)]
struct NormalizedPredParams {
    window_days: i32,
    inspection_type: Option<String>,
    product_id: Option<i32>,
    type_label: String,
    /// V15 P1 2.2：面料行业特征（染料/助剂/温度/缸号/胚布来源）
    dye_type: Option<String>,
    auxiliary_type: Option<String>,
    temperature_range: Option<(f64, f64)>,
    batch_no: Option<String>,
    fabric_source: Option<String>,
}

/// 标准化 `predict_quality` 入参
/// `window_days`：默认 90，限幅 1-365；`inspection_type`：trim 后若为空字符串则视为 None；`type_label`：用于响应的展示标签，未指定时为 "all"
fn normalize_pred_params(request: QualityPredRequest) -> NormalizedPredParams {
    let window_days = request.window_days.unwrap_or(90).clamp(1, 365);
    let inspection_type = request
        .inspection_type
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let product_id = request.product_id;
    let type_label = inspection_type.clone().unwrap_or_else(|| "all".to_string());
    let trim_opt = |s: Option<String>| -> Option<String> {
        s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    };
    NormalizedPredParams {
        window_days,
        inspection_type,
        product_id,
        type_label,
        dye_type: trim_opt(request.dye_type),
        auxiliary_type: trim_opt(request.auxiliary_type),
        temperature_range: request.temperature_range.filter(|(lo, hi)| lo <= hi),
        batch_no: trim_opt(request.batch_no),
        fabric_source: trim_opt(request.fabric_source),
    }
}

/// 构造历史数据不足时的退化响应（固定值：合格率 95% + 置信度 0.3 + 风险等级"中" + 风险分 30。）
fn build_fallback_response(
    product_id: Option<i32>,
    type_label: &str,
    window_days: i32,
) -> QualityPredResponse {
    let recommendations = build_recommendations("中");
    QualityPredResponse {
        product_id,
        inspection_type: type_label.to_string(),
        window_days,
        total_inspections: 0,
        avg_qualification_rate: FALLBACK_QUALIFICATION_RATE,
        trend: "无数据".to_string(),
        trend_rate: 0.0,
        risk_score: 30,
        risk_level: "中".to_string(),
        confidence: FALLBACK_CONFIDENCE,
        top_issues: Vec::new(),
        recommendations,
        period_breakdown: Vec::new(),
        source: "fallback".to_string(),
        cache_hit: false,
        degraded: false,
        explanation: None,
        factors: Vec::new(),
    }
}

/// V15 P1 9.1+9.5：构造推理超时/模型不可用时的降级响应
/// 与 `build_fallback_response` 区别：本函数用于异常场景（非算法退化），；`degraded=true` 标识前端可展示"AI 服务降级"提示，source="degraded"。
fn build_degraded_response(
    product_id: Option<i32>,
    type_label: &str,
    window_days: i32,
    _reason: String,
) -> QualityPredResponse {
    let recommendations = build_recommendations("中");
    QualityPredResponse {
        product_id,
        inspection_type: type_label.to_string(),
        window_days,
        total_inspections: 0,
        avg_qualification_rate: FALLBACK_QUALIFICATION_RATE,
        trend: "无数据".to_string(),
        trend_rate: 0.0,
        risk_score: 30,
        risk_level: "中".to_string(),
        confidence: FALLBACK_CONFIDENCE,
        top_issues: Vec::new(),
        recommendations,
        period_breakdown: Vec::new(),
        source: "degraded".to_string(),
        cache_hit: false,
        degraded: true,
        explanation: None,
        factors: Vec::new(),
    }
}

/// 基于历史记录构造预测响应（聚合 / 趋势 / 风险 / 归因 / 建议）
fn build_history_response(
    params: NormalizedPredParams,
    records: &[QualityInspectionModel],
) -> QualityPredResponse {
    let avg_rate = mean_qualification_rate(records);
    let period_breakdown = build_period_breakdown(records);
    let (trend_label, trend_rate_value, trend_is_down) = compute_recent_trend(records);
    let risk_score = compute_risk_score(avg_rate, trend_is_down);
    let risk_level = classify_risk_level(risk_score);
    let confidence = compute_confidence(records.len() as i64);
    let top_issues = compute_top_issues(records);
    let recommendations = build_recommendations(&risk_level);

    let explanation = build_explanation(
        &risk_level,
        &trend_label,
        avg_rate,
        &top_issues,
        records.len(),
    );

    // V15 P2 14.7.1：构建因子贡献列表
    let factors = build_quality_factors(avg_rate, trend_is_down, &top_issues, records.len());

    QualityPredResponse {
        product_id: params.product_id,
        inspection_type: params.type_label,
        window_days: params.window_days,
        total_inspections: records.len() as i64,
        avg_qualification_rate: round2(avg_rate),
        trend: trend_label,
        trend_rate: round2(trend_rate_value * 100.0), // 转为百分点
        risk_score: risk_score.round() as u32,
        risk_level,
        confidence,
        top_issues,
        recommendations,
        period_breakdown,
        source: "history".to_string(),
        cache_hit: false,
        degraded: false,
        explanation: Some(explanation),
        factors,
    }
}

/// V15 P1 5.3：构建质量预测缓存键（入参指纹）
/// 由 product_id/inspection_type/window_days/dye_type/auxiliary_type/temperature_range/batch_no/fabric_source；拼接而成，相同入参 5 分钟内命中缓存。
fn build_quality_cache_key(request: &QualityPredRequest) -> String {
    let temp_range = request
        .temperature_range
        .map(|(lo, hi)| format!("{}_{}", lo, hi))
        .unwrap_or_else(|| "none".to_string());
    format!(
        "quality_pred:{}|{}|{}|{}|{}|{}|{}|{}",
        request.product_id.unwrap_or(0),
        request.inspection_type.as_deref().unwrap_or("all"),
        request.window_days.unwrap_or(90),
        request.dye_type.as_deref().unwrap_or(""),
        request.auxiliary_type.as_deref().unwrap_or(""),
        temp_range,
        request.batch_no.as_deref().unwrap_or(""),
        request.fabric_source.as_deref().unwrap_or(""),
    )
}

/// 按月分段统计（以 `inspection_date` 的 `YYYY-MM` 为 key 聚合每条记录的 `qualification_rate`，；生成 `PeriodStat` 列表（BTreeMap 保证时间升序）。）
fn build_period_breakdown(records: &[QualityInspectionModel]) -> Vec<PeriodStat> {
    let mut monthly: std::collections::BTreeMap<String, Vec<f64>> =
        std::collections::BTreeMap::new();
    for r in records {
        let key = r.inspection_date.format("%Y-%m").to_string();
        let rate = r
            .qualification_rate
            .as_ref()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0);
        monthly.entry(key).or_default().push(rate);
    }
    monthly
        .iter()
        .map(|(k, v)| PeriodStat {
            period: k.clone(),
            inspections: v.len() as i64,
            avg_qualification_rate: round2(mean(v)),
        })
        .collect()
}

/// 计算最近 30 天 vs 之前 30 天的趋势
/// 返回 `(trend_label, trend_rate_value, trend_is_down)`：`trend_label`：上升 / 平稳 / 下降 / 无数据；`trend_rate_value`：原始变化率（如 0.125），由调用方转为百分点；`trend_is_down`：是否处于下降趋势（用于风险评分）
fn compute_recent_trend(records: &[QualityInspectionModel]) -> (String, f64, bool) {
    let now = chrono::Utc::now().date_naive();
    let recent_cutoff = now - chrono::Duration::days(30);
    let previous_cutoff = now - chrono::Duration::days(60);
    let recent_avg = mean_qualification_rate(
        &records
            .iter()
            .filter(|r| r.inspection_date >= recent_cutoff)
            .cloned()
            .collect::<Vec<_>>(),
    );
    let previous_avg = mean_qualification_rate(
        &records
            .iter()
            .filter(|r| r.inspection_date >= previous_cutoff && r.inspection_date < recent_cutoff)
            .cloned()
            .collect::<Vec<_>>(),
    );

    let trend_rate_value = compute_trend_rate(recent_avg, previous_avg);
    let trend_label = if recent_avg <= 0.0 || previous_avg <= 0.0 {
        "无数据".to_string()
    } else {
        classify_trend(trend_rate_value)
    };
    let trend_is_down = trend_label == "下降";
    (trend_label, trend_rate_value, trend_is_down)
}

/// 问题归因：仅统计不合格记录，按出现频次取 top 3（不合格定义：`qualification_rate < 100.0`。；归因类别由 `extract_issue_keyword` 从 `remark` 提取。）
fn compute_top_issues(records: &[QualityInspectionModel]) -> Vec<QualityIssue> {
    let mut issue_counter: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    let mut unqualified_total: i64 = 0;
    for r in records {
        let is_unqualified = r
            .qualification_rate
            .as_ref()
            .and_then(|d| d.to_f64())
            .map(|v| v < 100.0)
            .unwrap_or(false);
        if !is_unqualified {
            continue;
        }
        unqualified_total += 1;
        let key = extract_issue_keyword(r.defect_type.as_deref(), r.remark.as_deref());
        *issue_counter.entry(key).or_insert(0) += 1;
    }
    let mut top_issues: Vec<QualityIssue> = issue_counter
        .into_iter()
        .map(|(k, v)| {
            let pct = if unqualified_total > 0 {
                (v as f64 / unqualified_total as f64) * 100.0
            } else {
                0.0
            };
            QualityIssue {
                issue_type: k,
                occurrences: v,
                percentage: round2(pct),
            }
        })
        .collect();
    top_issues.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    top_issues.truncate(3);
    top_issues
}

/// V15 P2 14.7.1：构建质量预测的因子贡献列表
fn build_quality_factors(
    avg_rate: f64,
    trend_is_down: bool,
    top_issues: &[QualityIssue],
    sample_count: usize,
) -> Vec<FactorContribution> {
    let mut factors = Vec::new();

    // 合格率因子
    let rate_weight = RISK_WEIGHT_RATE;
    factors.push(FactorContribution {
        factor_name: "平均合格率".to_string(),
        weight: round2(rate_weight),
        contribution: format!(
            "当前合格率 {:.1}%，对风险评分贡献 {:.1} 分",
            avg_rate,
            (100.0 - avg_rate).max(0.0) * rate_weight
        ),
    });

    // 趋势因子
    let trend_weight = RISK_WEIGHT_TREND;
    factors.push(FactorContribution {
        factor_name: "趋势惩罚".to_string(),
        weight: round2(trend_weight),
        contribution: if trend_is_down {
            format!(
                "下降趋势触发惩罚 +{:.1} 分",
                TREND_DOWN_PENALTY * trend_weight
            )
        } else {
            "趋势平稳，无额外惩罚".to_string()
        },
    });

    // 问题归因因子
    if !top_issues.is_empty() {
        let top = &top_issues[0];
        factors.push(FactorContribution {
            factor_name: "主要问题归因".to_string(),
            weight: 0.0,
            contribution: format!(
                "最突出问题「{}」占比 {:.1}%（{} 次）",
                top.issue_type, top.percentage, top.occurrences
            ),
        });
    }

    // 样本量因子
    factors.push(FactorContribution {
        factor_name: "样本量置信度".to_string(),
        weight: round2(sample_count as f64 / CONFIDENCE_FULL_SAMPLE as f64).min(1.0),
        contribution: format!(
            "基于 {} 条历史记录，置信度 {:.0}%",
            sample_count,
            (sample_count as f64 / CONFIDENCE_FULL_SAMPLE as f64).min(1.0) * 100.0
        ),
    });

    factors
}

// =====================================================
// 单元测试（不依赖数据库，覆盖纯函数）
// =====================================================

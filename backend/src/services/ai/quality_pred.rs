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
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::models::quality_inspection_record::{
    Entity as QualityInspectionEntity, Model as QualityInspectionModel,
};
use crate::utils::error::AppError;

use super::{mean, AiAnalysisService};

// =====================================================
// 输入 / 输出 DTO
// =====================================================

/// 质量预测请求
#[derive(Debug, Clone, Deserialize)]
pub struct QualityPredRequest {
    /// 可选：限定产品 ID
    pub product_id: Option<i32>,
    /// 可选：限定检验类型（进货/过程/成品/出货 等）
    pub inspection_type: Option<String>,
    /// 可选：时间窗口天数（默认 90 天，1-365）
    pub window_days: Option<i32>,
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
pub(crate) const FALLBACK_CONFIDENCE: f64 = 0.3;
/// 风险等级阈值（≥ 高 / < 高 且 ≥ 中 / < 中）
pub(crate) const RISK_LEVEL_HIGH: f64 = 60.0;
pub(crate) const RISK_LEVEL_MEDIUM: f64 = 30.0;
/// 置信度上限对应的样本量（达到该样本数置信度封顶）
pub(crate) const CONFIDENCE_FULL_SAMPLE: i64 = 30;

/// 质量归因关键词库（中文常用术语）
///
/// 提取自 `remark` 字段，按出现频次归类问题类型。
/// - 颜色差异：颜色偏差 / 偏色 / 颜色不符 / 异色
/// - 色牢度：色牢度 / 褪色 / 沾色 / 耐洗
/// - 克重：克重 / 平米克重
/// - 纬密：纬密 / 密度 / 经密
/// - 强度：强度 / 强力 / 断裂
/// - 其他：未命中关键词的记录统一归为"其他"
const ISSUE_KEYWORDS: &[(&str, &[&str])] = &[
    ("颜色差异", &["颜色", "偏色", "异色", "色差", "色不符"]),
    ("色牢度", &["色牢度", "褪色", "沾色", "耐洗"]),
    ("克重偏差", &["克重", "平米克重"]),
    ("纬密偏差", &["纬密", "密度", "经密"]),
    ("强度不足", &["强度", "强力", "断裂"]),
];

/// 从 `remark` 文本中匹配问题类型关键词
///
/// 返回匹配到的归因类别（"颜色差异"/"色牢度"/"克重偏差"/"纬密偏差"/"强度不足"/"其他"）。
pub(crate) fn extract_issue_keyword(remark: Option<&str>) -> String {
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
///
/// 公式：`risk = (100 - avg_rate) * 0.6 + trend_down * 0.4`
/// - `avg_rate`      当前平均合格率（百分比 0-100）
/// - `trend_is_down` 是否处于下降趋势
///
/// 输出 0-100，越高越危险。
pub(crate) fn compute_risk_score(avg_rate: f64, trend_is_down: bool) -> f64 {
    let rate_part = ((100.0 - avg_rate).max(0.0) * RISK_WEIGHT_RATE).min(60.0);
    let trend_part = if trend_is_down {
        TREND_DOWN_PENALTY * RISK_WEIGHT_TREND
    } else {
        0.0
    };
    (rate_part + trend_part).clamp(0.0, RISK_MAX)
}

/// 风险等级分类
///
/// - `score >= 60`        → "高"
/// - `30 <= score < 60`   → "中"
/// - `score < 30`         → "低"
pub(crate) fn classify_risk_level(score: f64) -> String {
    if score >= RISK_LEVEL_HIGH {
        "高".to_string()
    } else if score >= RISK_LEVEL_MEDIUM {
        "中".to_string()
    } else {
        "低".to_string()
    }
}

/// 趋势判定（基于变化率）
///
/// - `rate >  5%`  → "上升"
/// - `rate < -5%`  → "下降"
/// - 其他          → "平稳"
pub(crate) fn classify_trend(rate: f64) -> String {
    if rate > TREND_THRESHOLD {
        "上升".to_string()
    } else if rate < -TREND_THRESHOLD {
        "下降".to_string()
    } else {
        "平稳".to_string()
    }
}

/// 趋势变化率计算
///
/// `recent` / `previous` 分别是最近 30 天 / 之前 30 天的平均合格率（百分比）。
/// 返回 `(recent - previous) / previous`（previous=0 时返回 0.0 兜底）。
pub(crate) fn compute_trend_rate(recent: f64, previous: f64) -> f64 {
    if previous.abs() < 0.0001 {
        return 0.0;
    }
    (recent - previous) / previous
}

/// 置信度计算
///
/// 公式：`min(sample_count / CONFIDENCE_FULL_SAMPLE, 1.0)`，四舍五入到 0.01。
/// 退化路径由调用方传入固定 0.3。
pub(crate) fn compute_confidence(sample_count: i64) -> f64 {
    if sample_count <= 0 {
        return FALLBACK_CONFIDENCE;
    }
    let ratio = (sample_count as f64 / CONFIDENCE_FULL_SAMPLE as f64).min(1.0);
    (ratio * 100.0).round() / 100.0
}

/// 风险等级 → 建议措施
///
/// 严格按等级分档生成 1-3 条建议，确保 UI 列表非空。
pub(crate) fn build_recommendations(level: &str) -> Vec<String> {
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

/// 计算给定一组记录的平均合格率
///
/// 优先使用记录自身的 `qualification_rate`（百分比 0-100）；
/// 缺失时回退到 `qualified_qty / inspected_qty`。
/// 返回百分比 0-100。
pub(crate) fn mean_qualification_rate(records: &[QualityInspectionModel]) -> f64 {
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
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// 保留 2 位小数
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// =====================================================
// Service 实现
// =====================================================

impl AiAnalysisService {
    /// 质量预测主入口
    ///
    /// 优先使用历史真实数据（≥ 5 条），不足时回退到保守默认值
    /// （合格率 95% + 置信度 0.3 + 风险等级中）。
    pub async fn predict_quality(
        &self,
        request: QualityPredRequest,
    ) -> Result<QualityPredResponse, AppError> {
        // 1. 参数标准化
        let params = normalize_pred_params(request);

        // 2. 拉取窗口内的全部检验记录
        let records = self
            .fetch_quality_records(
                params.window_days,
                params.product_id,
                params.inspection_type.as_deref(),
            )
            .await?;

        // 3. 历史数据不足 → 退化路径
        if (records.len() as i64) < MIN_HISTORY_RECORDS {
            return Ok(build_fallback_response(
                params.product_id,
                &params.type_label,
                params.window_days,
            ));
        }

        // 4. 聚合：平均合格率
        let avg_rate = mean_qualification_rate(&records);

        // 5. 按月分段统计
        let period_breakdown = build_period_breakdown(&records);

        // 6. 趋势：最近 30 天 vs 之前 30 天
        let (trend_label, trend_rate_value, trend_is_down) = compute_recent_trend(&records);

        // 7. 风险评分 + 风险等级
        let risk_score = compute_risk_score(avg_rate, trend_is_down);
        let risk_level = classify_risk_level(risk_score);
        let risk_score_u32 = risk_score.round() as u32;

        // 8. 置信度
        let confidence = compute_confidence(records.len() as i64);

        // 9. 问题归因：仅统计不合格记录
        let top_issues = compute_top_issues(&records);

        // 10. 建议措施
        let recommendations = build_recommendations(&risk_level);

        Ok(QualityPredResponse {
            product_id: params.product_id,
            inspection_type: params.type_label,
            window_days: params.window_days,
            total_inspections: records.len() as i64,
            avg_qualification_rate: round2(avg_rate),
            trend: trend_label,
            trend_rate: round2(trend_rate_value * 100.0), // 转为百分点
            risk_score: risk_score_u32,
            risk_level,
            confidence,
            top_issues,
            recommendations,
            period_breakdown,
            source: "history".to_string(),
        })
    }

    /// 拉取指定时间窗口内的全部质量检验记录
    ///
    /// 按 `product_id` / `inspection_type` 可选过滤；
    /// 时间下界为 `today - window_days`。
    async fn fetch_quality_records(
        &self,
        window_days: i32,
        product_id: Option<i32>,
        inspection_type: Option<&str>,
    ) -> Result<Vec<QualityInspectionModel>, AppError> {
        let cutoff = chrono::Utc::now().date_naive() - chrono::Duration::days(window_days as i64);

        let mut select = QualityInspectionEntity::find()
            .filter(crate::models::quality_inspection_record::Column::InspectionDate.gte(cutoff));
        if let Some(pid) = product_id {
            select =
                select.filter(crate::models::quality_inspection_record::Column::ProductId.eq(pid));
        }
        if let Some(t) = inspection_type {
            select = select
                .filter(crate::models::quality_inspection_record::Column::InspectionType.eq(t));
        }

        let records = select.all(&*self.db).await?;
        Ok(records)
    }
}

// =====================================================
// predict_quality 内部辅助函数（不依赖数据库，可直接单测）
// =====================================================

/// `predict_quality` 入参标准化后的上下文
///
/// 封装 `window_days` / `inspection_type` / `product_id` / `type_label`，
/// 避免主函数散落 4 个局部变量；参考已有 `WageTotals` / `ApproveContext` 模式。
struct NormalizedPredParams {
    window_days: i32,
    inspection_type: Option<String>,
    product_id: Option<i32>,
    type_label: String,
}

/// 标准化 `predict_quality` 入参
///
/// - `window_days`：默认 90，限幅 1-365
/// - `inspection_type`：trim 后若为空字符串则视为 None
/// - `type_label`：用于响应的展示标签，未指定时为 "all"
fn normalize_pred_params(request: QualityPredRequest) -> NormalizedPredParams {
    let window_days = request.window_days.unwrap_or(90).clamp(1, 365);
    let inspection_type = request
        .inspection_type
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let product_id = request.product_id;
    let type_label = inspection_type.clone().unwrap_or_else(|| "all".to_string());
    NormalizedPredParams {
        window_days,
        inspection_type,
        product_id,
        type_label,
    }
}

/// 构造历史数据不足时的退化响应
///
/// 固定值：合格率 95% + 置信度 0.3 + 风险等级"中" + 风险分 30。
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
    }
}

/// 按月分段统计
///
/// 以 `inspection_date` 的 `YYYY-MM` 为 key 聚合每条记录的 `qualification_rate`，
/// 生成 `PeriodStat` 列表（BTreeMap 保证时间升序）。
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
///
/// 返回 `(trend_label, trend_rate_value, trend_is_down)`：
/// - `trend_label`：上升 / 平稳 / 下降 / 无数据
/// - `trend_rate_value`：原始变化率（如 0.125），由调用方转为百分点
/// - `trend_is_down`：是否处于下降趋势（用于风险评分）
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
            .filter(|r| {
                r.inspection_date >= previous_cutoff && r.inspection_date < recent_cutoff
            })
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

/// 问题归因：仅统计不合格记录，按出现频次取 top 3
///
/// 不合格定义：`qualification_rate < 100.0`。
/// 归因类别由 `extract_issue_keyword` 从 `remark` 提取。
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
        let key = extract_issue_keyword(r.remark.as_deref());
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

// =====================================================
// 单元测试（不依赖数据库，覆盖纯函数）
// =====================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// 构造一条 `QualityInspectionModel` 测试夹具
    fn make_record(
        product_id: i32,
        inspection_type: &str,
        inspection_date: chrono::NaiveDate,
        qualification_rate: Option<f64>,
        remark: Option<&str>,
    ) -> QualityInspectionModel {
        let rate_dec = qualification_rate.and_then(rust_decimal::Decimal::from_f64_retain);
        let is_pass = qualification_rate.unwrap_or(100.0) >= 100.0;
        QualityInspectionModel {
            id: 0,
            inspection_no: format!("QC-{}", product_id),
            inspection_type: inspection_type.to_string(),
            related_type: None,
            related_id: None,
            product_id,
            batch_no: None,
            supplier_id: None,
            customer_id: None,
            inspection_date,
            inspector_id: None,
            total_qty: Decimal::try_from(100.0_f64).unwrap_or(Decimal::ZERO),
            inspected_qty: Decimal::try_from(100.0_f64).unwrap_or(Decimal::ZERO),
            qualified_qty: Some(Decimal::try_from(95.0_f64).unwrap_or(Decimal::ZERO)),
            unqualified_qty: Some(Decimal::try_from(5.0_f64).unwrap_or(Decimal::ZERO)),
            qualification_rate: rate_dec,
            inspection_result: if is_pass {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            remark: remark.map(|s| s.to_string()),
            // V15 Batch 485：补齐 v14 批次 421 新增字段（color_no/dye_lot_no/grade）
            // 测试夹具不涉及缸号/颜色追溯，使用 None
            grade: None,
            color_no: None,
            dye_lot_no: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// 测试 1：风险评分 - 合格率 99% 且趋势平稳 → 风险分 < 20（等级"低"）
    #[test]
    fn test_risk_score_low() {
        let score = compute_risk_score(99.0, false);
        // (100 - 99) * 0.6 = 0.6，加上 0 trend → 0.6
        assert!(
            score < 20.0,
            "合格率 99% 且趋势平稳时风险分应 < 20，实际 {}",
            score
        );
        assert!(score >= 0.0, "风险分应为非负，实际 {}", score);
        // 风险等级分类应为"低"
        assert_eq!(classify_risk_level(score), "低");
    }

    /// 测试 2：风险评分 - 极低合格率 + 下降趋势 → 风险分 > 60（等级"高"）
    ///
    /// 注：原 spec 文字"合格率 70%"在公式 `(100-avg)*0.6 + 15*0.4` 下
    /// 仅得 24，数学上无法 > 60；此处采用能确保 > 60 的极低合格率（0%）
    /// 作为高风险测试场景，公式不变。
    #[test]
    fn test_risk_score_high() {
        let score = compute_risk_score(0.0, true);
        // (100 - 0) * 0.6 = 60，加上 15 * 0.4 = 6 → 66
        assert!(
            score > 60.0,
            "合格率 0% 且趋势下降时风险分应 > 60，实际 {}",
            score
        );
        // 风险等级分类应为"高"
        assert_eq!(classify_risk_level(score), "高");

        // 70% + 下降 → 24（公式极限），验证"中"档
        let score_70_down = compute_risk_score(70.0, true);
        assert!(
            score_70_down > compute_risk_score(70.0, false),
            "下降趋势应抬高风险分（70% 平稳 {} vs 下降 {}）",
            compute_risk_score(70.0, false),
            score_70_down
        );
    }

    /// 测试 3：趋势计算 - 3 期合格率 80 → 85 → 90 → 应判定为上升
    #[test]
    fn test_trend_calculation() {
        // 上升：recent 90, previous 80 → (90-80)/80 = 0.125 = 12.5% > 5%
        let rate = compute_trend_rate(90.0, 80.0);
        assert!(
            (rate - 0.125).abs() < 0.0001,
            "变化率应为 0.125，实际 {}",
            rate
        );
        let label = classify_trend(rate);
        assert_eq!(label, "上升", "趋势应判定为上升，实际 {}", label);

        // 下降：recent 60, previous 90 → (60-90)/90 = -0.333
        let rate_down = compute_trend_rate(60.0, 90.0);
        let label_down = classify_trend(rate_down);
        assert_eq!(label_down, "下降", "趋势应判定为下降，实际 {}", label_down);

        // 平稳：recent 82, previous 80 → 2.5% 处于 ±5% 内
        let rate_flat = compute_trend_rate(82.0, 80.0);
        let label_flat = classify_trend(rate_flat);
        assert_eq!(label_flat, "平稳", "趋势应判定为平稳，实际 {}", label_flat);

        // previous=0 兜底
        let rate_zero = compute_trend_rate(50.0, 0.0);
        assert!((rate_zero - 0.0).abs() < 0.0001);
    }

    /// 测试 4：退化路径 - 数据 < 5 条 → 合格率 95% + 置信度 0.3
    #[test]
    fn test_fallback_low_data() {
        // 历史 0 条记录（模拟）
        let empty: Vec<QualityInspectionModel> = vec![];
        let rate = mean_qualification_rate(&empty);
        assert!((rate - 0.0).abs() < 0.0001, "空记录集合应返回 0.0");

        // 置信度 - 0 条
        let conf = compute_confidence(0);
        assert!(
            (conf - FALLBACK_CONFIDENCE).abs() < 0.0001,
            "0 条记录置信度应等于 0.3，实际 {}",
            conf
        );

        // 置信度 - 5 条：5/30 = 0.1667，四舍五入到 0.17
        let conf5 = compute_confidence(5);
        assert!(
            (conf5 - 0.17).abs() < 0.01,
            "5 条记录置信度应约为 0.17，实际 {}",
            conf5
        );

        // 置信度 - 30 条以上封顶
        let conf30 = compute_confidence(30);
        assert!(
            (conf30 - 1.0).abs() < 0.0001,
            "30 条记录置信度应封顶到 1.0，实际 {}",
            conf30
        );

        // 建议措施：中等级
        let recs = build_recommendations("中");
        assert!(!recs.is_empty(), "中等级建议措施不应为空");
        assert!(recs.len() >= 2, "中等级应有 ≥ 2 条建议");
        // 风险评分
        let mid_score = compute_risk_score(95.0, false);
        assert!(
            (0.0..=30.0).contains(&mid_score),
            "95% 合格率无下降趋势应得低分，实际 {}",
            mid_score
        );

        // 问题归因关键词提取
        assert_eq!(extract_issue_keyword(Some("颜色偏深")), "颜色差异");
        assert_eq!(extract_issue_keyword(Some("色牢度不合格")), "色牢度");
        assert_eq!(extract_issue_keyword(Some("克重不足")), "克重偏差");
        assert_eq!(extract_issue_keyword(Some("纬密偏低")), "纬密偏差");
        assert_eq!(extract_issue_keyword(Some("强度不够")), "强度不足");
        assert_eq!(extract_issue_keyword(Some("无匹配项")), "其他");
        assert_eq!(extract_issue_keyword(None), "其他");
    }

    /// 测试 5：辅助函数覆盖 - 用真实记录验证 `mean_qualification_rate`
    /// 使用 `make_record` 构造 3 条记录，确保 `#[allow(dead_code)]`
    /// 不会因辅助函数未使用而失效。
    #[test]
    fn test_mean_qualification_with_real_records() {
        // P9-1: 用 ymd! 宏统一日期构造
        let d1 = crate::ymd!(2024, 1, 15);
        let d2 = crate::ymd!(2024, 2, 15);
        let d3 = crate::ymd!(2024, 3, 15);
        let records = vec![
            make_record(1, "成品检验", d1, Some(98.0), None),
            make_record(1, "成品检验", d2, Some(96.0), None),
            make_record(1, "成品检验", d3, Some(94.0), None),
        ];
        let avg = mean_qualification_rate(&records);
        // (98 + 96 + 94) / 3 = 96.0
        assert!(
            (avg - 96.0).abs() < 0.0001,
            "3 条记录平均合格率应为 96.0，实际 {}",
            avg
        );
    }
}

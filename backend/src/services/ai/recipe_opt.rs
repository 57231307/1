//! AI 染色工艺优化服务（ai/recipe_opt）
//!
//! 基于 `dye_recipe` 历史数据 + k-NN 相似度算法，向现场工艺员推荐
//! 染色参数（温度 / 时间 / pH / 浴比）。
//!
//! 算法概要：
//! 1. 取近 6 个月内、未删除的 `dye_recipe` 历史数据作为候选集
//! 2. 对每条历史配方，按 `color_no` / `fabric_type` / `dye_type` 三个维度
//!    计算相似度：
//!    - `color_no` 精确匹配得 1.0；前缀 3 位相同得 0.7；否则 0.0
//!    - `fabric_type` 完全相同 +0.2
//!    - `dye_type` 完全相同 +0.1
//! 3. 取相似度 Top K（默认 K=5），按相似度加权平均得到推荐参数
//! 4. 当有效历史数据 < 3 条时，回退到内置典型参数表
//!
//! 模块内拆出多个纯函数（`compute_similarity` / `weighted_average_params` /
//! `find_typical_params` / `build_candidates`），单元测试可直接调用，避免依赖数据库。

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::models::dye_recipe::{Entity as DyeRecipeEntity, Model as DyeRecipeModel};
use crate::utils::error::AppError;

use super::AiAnalysisService;

// =====================================================
// 输入 / 输出 DTO
// =====================================================

/// 工艺优化推荐请求
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeOptRequest {
    /// 色号（如 "BL-301"），必填
    pub color_no: String,
    /// 布类（棉 / 涤纶 / 丝绸 / 羊毛 等），必填
    pub fabric_type: String,
    /// 颜色名称（可选，仅用于展示与日志）
    pub color_name: Option<String>,
    /// 染料类型（活性 / 分散 / 酸性 / 还原 等，可选）
    pub dye_type: Option<String>,
    /// k-NN 近邻数（可选，默认 5；传 0 时强制走退化路径）
    pub k: Option<usize>,
}

/// 工艺推荐主参数
#[derive(Debug, Clone, Serialize)]
pub struct RecipeParams {
    /// 染色温度（°C）
    pub temperature: f64,
    /// 染色时间（分钟）
    pub time_minutes: i32,
    /// 染浴 pH 值
    pub ph_value: f64,
    /// 浴比
    pub liquor_ratio: f64,
}

/// 相似候选案例（命中 TopK 后的前 10 条）
#[derive(Debug, Serialize)]
pub struct RecipeCandidate {
    pub recipe_no: String,
    pub color_no: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub temperature: Option<f64>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<f64>,
    pub liquor_ratio: Option<f64>,
    /// 相似度（0.0 - 1.0 归一化值）
    pub similarity: f64,
}

/// 工艺优化推荐响应
#[derive(Debug, Serialize)]
pub struct RecipeOptResponse {
    /// 推荐参数
    pub recommended_params: RecipeParams,
    /// 命中的相似历史配方数量
    pub similar_cases: usize,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 来源标识："knn" | "fallback"
    pub source: String,
    /// 人类可读原因说明
    pub reason: String,
    /// 候选案例（最多 10 条）
    pub candidates: Vec<RecipeCandidate>,
}

// =====================================================
// 内部纯函数（不依赖数据库，可直接单测）
// =====================================================

/// 相似度评分最大理论值（颜色 1.0 + 布类 0.2 + 染料 0.1 = 1.3）
pub(crate) const MAX_SIMILARITY: f64 = 1.3;
/// 典型参数回退的温度默认值（°C）
pub(crate) const TYPICAL_TEMPERATURE: f64 = 80.0;
/// 典型参数回退的时间默认值（分钟）
pub(crate) const TYPICAL_TIME_MINUTES: i32 = 45;
/// 典型参数回退的 pH 默认值
pub(crate) const TYPICAL_PH: f64 = 6.0;
/// 典型参数回退的浴比默认值
pub(crate) const TYPICAL_LIQUOR_RATIO: f64 = 8.0;

/// 计算两条配方的相似度（0.0 - 1.3）
///
/// 评分规则：
/// - `color_no` 精确（大小写不敏感）相等 → 1.0
/// - `color_no` 前缀 3 位相同（忽略分隔符）→ 0.7
/// - 否则 → 0.0
/// - `fabric_type` 精确相等 → +0.2
/// - `dye_type` 精确相等 → +0.1
pub(crate) fn compute_similarity(
    target_color: &str,
    target_fabric: &str,
    target_dye: Option<&str>,
    candidate: &DyeRecipeModel,
) -> f64 {
    let color_score = color_similarity(target_color, candidate.color_no.as_deref().unwrap_or(""));

    // 没有任何颜色信号视为完全无关
    if color_score <= 0.0 {
        return 0.0;
    }

    let mut score = color_score;
    if let Some(c_fabric) = &candidate.fabric_type {
        if !target_fabric.is_empty() && c_fabric.eq_ignore_ascii_case(target_fabric) {
            score += 0.2;
        }
    }
    if let (Some(t_dye), Some(c_dye)) = (target_dye, candidate.dye_type.as_deref()) {
        if !t_dye.is_empty() && c_dye.eq_ignore_ascii_case(t_dye) {
            score += 0.1;
        }
    }
    score
}

/// 颜色号相似度（仅依赖 color_no 字符串）
///
/// 标准化时去除常见分隔符 `-` `_` `/` ` `，便于"BL301" 与 "BL-301" 模糊匹配。
fn color_similarity(target: &str, candidate: &str) -> f64 {
    if target.is_empty() || candidate.is_empty() {
        return 0.0;
    }
    let t_norm = normalize_color(target);
    let c_norm = normalize_color(candidate);
    if t_norm == c_norm {
        return 1.0;
    }
    if t_norm.len() >= 3 && c_norm.len() >= 3 && t_norm[..3] == c_norm[..3] {
        return 0.7;
    }
    0.0
}

/// 标准化色号：转大写、移除分隔符
fn normalize_color(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// 内部加权聚合结果
#[derive(Debug, Clone, Default)]
pub(crate) struct AggregatedParams {
    pub temperature: f64,
    pub time_minutes: f64,
    pub ph_value: f64,
    pub liquor_ratio: f64,
    pub total_weight: f64,
}

/// 按相似度加权聚合多条命中配方的参数
pub(crate) fn weighted_average_params(
    hits: &[(f64, &DyeRecipeModel)],
) -> Option<AggregatedParams> {
    if hits.is_empty() {
        return None;
    }

    let mut temp_sum = 0.0_f64;
    let mut time_sum = 0.0_f64;
    let mut ph_sum = 0.0_f64;
    let mut liquor_sum = 0.0_f64;
    let mut weight_sum = 0.0_f64;

    for (score, model) in hits {
        let w = *score;
        if w <= 0.0 {
            continue;
        }
        if let Some(t) = model.temperature {
            temp_sum += t.to_f64().unwrap_or(0.0) * w;
        }
        if let Some(t) = model.time_minutes {
            time_sum += (t as f64) * w;
        }
        if let Some(p) = model.ph_value {
            ph_sum += p.to_f64().unwrap_or(0.0) * w;
        }
        if let Some(l) = model.liquor_ratio {
            liquor_sum += l.to_f64().unwrap_or(0.0) * w;
        }
        weight_sum += w;
    }

    if weight_sum <= 0.0 {
        return None;
    }

    Some(AggregatedParams {
        temperature: temp_sum / weight_sum,
        time_minutes: time_sum / weight_sum,
        ph_value: ph_sum / weight_sum,
        liquor_ratio: liquor_sum / weight_sum,
        total_weight: weight_sum,
    })
}

/// 内置典型参数表（退化兜底，固定 4 字段）
///
/// 典型值（兜底，参考规格）：
/// - 温度：80°C ± 10°C → 默认 80
/// - 时间：45min ± 15min → 默认 45
/// - pH：6.0 ± 1.0 → 默认 6.0
/// - 浴比：1:8 ± 2 → 默认 8.0
pub(crate) fn find_typical_params() -> AggregatedParams {
    AggregatedParams {
        temperature: TYPICAL_TEMPERATURE,
        time_minutes: TYPICAL_TIME_MINUTES as f64,
        ph_value: TYPICAL_PH,
        liquor_ratio: TYPICAL_LIQUOR_RATIO,
        total_weight: 0.0,
    }
}

/// 计算最终置信度（0.0 - 1.0 归一化）
///
/// - k-NN 命中：min(命中条数 / K, 1.0) * 平均相似度归一化
/// - 退化路径：固定 0.6
pub(crate) fn compute_confidence(hits: &[(f64, &DyeRecipeModel)], k: usize) -> f64 {
    if hits.is_empty() {
        return 0.6;
    }
    let n = hits.len() as f64;
    let k = k.max(1) as f64;
    let coverage = (n / k).min(1.0);
    let avg_score = hits.iter().map(|(s, _)| *s).sum::<f64>() / n;
    // 归一化相似度（最大理论值 1.3）
    let normalized = (avg_score / MAX_SIMILARITY).clamp(0.0, 1.0);
    (coverage * normalized * 100.0).round() / 100.0
}

/// 将候选集合转换为响应中 `candidates` 字段
///
/// 取相似度 > 0 的前 10 条，并把原始分数归一化到 0.0-1.0。
pub(crate) fn build_candidates(
    scored: &[(f64, &DyeRecipeModel)],
    max_n: usize,
) -> Vec<RecipeCandidate> {
    scored
        .iter()
        .filter(|(s, _)| *s > 0.0)
        .take(max_n)
        .map(|(score, m)| RecipeCandidate {
            recipe_no: m.recipe_no.clone(),
            color_no: m.color_no.clone(),
            color_name: m.color_name.clone(),
            fabric_type: m.fabric_type.clone(),
            dye_type: m.dye_type.clone(),
            temperature: m.temperature.and_then(|d| d.to_f64()),
            time_minutes: m.time_minutes,
            ph_value: m.ph_value.and_then(|d| d.to_f64()),
            liquor_ratio: m.liquor_ratio.and_then(|d| d.to_f64()),
            similarity: ((*score / MAX_SIMILARITY) * 100.0).round() / 100.0,
        })
        .collect()
}

/// 保留 1 位小数
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// 判断是否需要走 k-NN 路径（命中条数 ≥ 3 才走 k-NN，否则退化）
pub(crate) fn should_use_knn(hit_count: usize) -> bool {
    hit_count >= 3
}

// =====================================================
// Service 实现
// =====================================================

impl AiAnalysisService {
    /// 染色工艺参数智能推荐
    ///
    /// 优先使用 k-NN 历史匹配（取 TopK，按相似度加权平均）；
    /// 命中 < 3 条或 k=0 时回退到典型参数表。
    pub async fn optimize_recipe(
        &self,
        request: RecipeOptRequest,
    ) -> Result<RecipeOptResponse, AppError> {
        // k 默认 5；k=0 时强制走退化路径
        let k = request.k.unwrap_or(5);

        // k=0 → 强制退化
        if k == 0 {
            let typical = find_typical_params();
            return Ok(RecipeOptResponse {
                recommended_params: RecipeParams {
                    temperature: typical.temperature,
                    time_minutes: typical.time_minutes as i32,
                    ph_value: typical.ph_value,
                    liquor_ratio: typical.liquor_ratio,
                },
                similar_cases: 0,
                confidence: 0.6,
                source: "fallback".to_string(),
                reason: "k=0，已强制走典型参数表".to_string(),
                candidates: Vec::new(),
            });
        }

        // 查询最近 6 个月、未删除的染色配方作为候选集
        let six_months_ago = chrono::Utc::now() - chrono::Duration::days(180);
        let six_months_ago_dt = six_months_ago.naive_utc();

        let candidates = DyeRecipeEntity::find()
            .filter(crate::models::dye_recipe::Column::IsDeleted.eq(false))
            .filter(
                crate::models::dye_recipe::Column::UpdatedAt
                    .gte(six_months_ago_dt),
            )
            .all(&*self.db)
            .await?;

        // 计算相似度并排序
        let mut scored: Vec<(f64, &DyeRecipeModel)> = candidates
            .iter()
            .map(|c| {
                (
                    compute_similarity(
                        &request.color_no,
                        &request.fabric_type,
                        request.dye_type.as_deref(),
                        c,
                    ),
                    c,
                )
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 取 TopK
        let top: Vec<(f64, &DyeRecipeModel)> = scored.iter().take(k).copied().collect();

        // 候选案例（取前 10 条），无论走哪条路径都返回，便于 UI 展示
        let resp_candidates = build_candidates(&scored, 10);

        if should_use_knn(top.len()) {
            // 走 k-NN 路径
            let agg = weighted_average_params(&top).ok_or_else(|| {
                AppError::internal("工艺推荐：k-NN 加权聚合失败")
            })?;
            let confidence = compute_confidence(&top, k);

            Ok(RecipeOptResponse {
                recommended_params: RecipeParams {
                    temperature: round1(agg.temperature),
                    time_minutes: agg.time_minutes.round() as i32,
                    ph_value: round1(agg.ph_value),
                    liquor_ratio: round1(agg.liquor_ratio),
                },
                similar_cases: top.len(),
                confidence,
                source: "knn".to_string(),
                reason: format!(
                    "基于 {} 条相似历史配方（k={}）的加权平均推荐",
                    top.len(),
                    k
                ),
                candidates: resp_candidates,
            })
        } else {
            // 退化：典型参数表（兜底）
            let typical = find_typical_params();

            Ok(RecipeOptResponse {
                recommended_params: RecipeParams {
                    temperature: typical.temperature,
                    time_minutes: typical.time_minutes as i32,
                    ph_value: typical.ph_value,
                    liquor_ratio: typical.liquor_ratio,
                },
                similar_cases: top.len(),
                confidence: 0.6,
                source: "fallback".to_string(),
                reason: format!(
                    "命中相似案例 {} 条（< 3），已回退到典型参数表（温度{}°C ±10、时间{}min ±15、pH{} ±1、浴比1:{} ±2）",
                    top.len(),
                    TYPICAL_TEMPERATURE,
                    TYPICAL_TIME_MINUTES,
                    TYPICAL_PH,
                    TYPICAL_LIQUOR_RATIO
                ),
                candidates: resp_candidates,
            })
        }
    }
}

// =====================================================
// 单元测试（不依赖数据库，覆盖纯函数）
// =====================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dye_recipe::AuxiliariesItem;
    use rust_decimal_macros::dec;

    /// 构造一条 `DyeRecipeModel` 测试夹具
    #[allow(clippy::too_many_arguments)]
    fn make_recipe(
        recipe_no: &str,
        color_no: &str,
        fabric_type: &str,
        dye_type: &str,
        temperature: f64,
        time_minutes: i32,
        ph: f64,
        liquor: f64,
    ) -> DyeRecipeModel {
        DyeRecipeModel {
            id: 0,
            recipe_no: recipe_no.to_string(),
            recipe_name: None,
            color_no: Some(color_no.to_string()),
            formula: None,
            temperature: Some(Decimal::try_from(temperature).unwrap_or(Decimal::ZERO)),
            time_minutes: Some(time_minutes),
            status: Some("active".to_string()),
            is_deleted: Some(false),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            color_code: None,
            color_name: Some("蓝色".to_string()),
            fabric_type: Some(fabric_type.to_string()),
            dye_type: Some(dye_type.to_string()),
            chemical_formula: None,
            ph_value: Some(Decimal::try_from(ph).unwrap_or(Decimal::ZERO)),
            liquor_ratio: Some(Decimal::try_from(liquor).unwrap_or(Decimal::ZERO)),
            auxiliaries: Some(vec![AuxiliariesItem {
                name: "助剂A".to_string(),
                amount: dec!(1.5),
                unit: "g/L".to_string(),
            }]),
            version: Some(1),
            parent_recipe_id: None,
            approved_by: None,
            approved_at: None,
            remarks: None,
            created_by: None,
        }
    }

    /// 测试 1：典型参数退化路径
    /// 当数据库无匹配（或命中 < 3 条）时，返回内置典型参数表
    /// 温度 80°C ± 10、时间 45min ± 15、pH 6.0 ± 1、浴比 1:8 ± 2
    #[test]
    fn test_typical_params_fallback() {
        let typical = find_typical_params();

        // 温度：80°C（±10）
        assert!(
            (typical.temperature - 80.0).abs() < 0.001,
            "典型温度应为 80.0，实际 {}",
            typical.temperature
        );
        assert!((typical.temperature - 80.0).abs() <= 10.0);

        // 时间：45min（±15）
        assert_eq!(typical.time_minutes as i32, 45);

        // pH：6.0（±1）
        assert!(
            (typical.ph_value - 6.0).abs() < 0.001,
            "典型 pH 应为 6.0，实际 {}",
            typical.ph_value
        );
        assert!((typical.ph_value - 6.0).abs() <= 1.0);

        // 浴比：1:8（±2）
        assert!(
            (typical.liquor_ratio - 8.0).abs() < 0.001,
            "典型浴比应为 8.0，实际 {}",
            typical.liquor_ratio
        );
        assert!((typical.liquor_ratio - 8.0).abs() <= 2.0);

        // 退化路径置信度固定 0.6
        let empty: Vec<(f64, &DyeRecipeModel)> = vec![];
        let conf = compute_confidence(&empty, 5);
        assert!((conf - 0.6).abs() < 0.001, "退化置信度应为 0.6，实际 {}", conf);

        // should_use_knn 边界
        assert!(!should_use_knn(0));
        assert!(!should_use_knn(2));
        assert!(should_use_knn(3));
        assert!(should_use_knn(5));
    }

    /// 测试 2：颜色完全匹配时使用 k-NN 加权平均
    /// 5 条完全匹配的配方 → 加权平均 = 各参数算术平均
    #[test]
    fn test_color_match_knn() {
        // 5 条全匹配：颜色 BL-301 + 棉 + 活性染料 → 相似度 1.3
        let history: Vec<DyeRecipeModel> = (0..5)
            .map(|i| {
                make_recipe(
                    &format!("R-BL301-{}", i),
                    "BL-301",
                    "棉",
                    "活性染料",
                    60.0 + i as f64,  // 60, 61, 62, 63, 64
                    40 + i as i32 * 2, // 40, 42, 44, 46, 48
                    6.0 + (i as f64) * 0.1, // 6.0, 6.1, 6.2, 6.3, 6.4
                    10.0,
                )
            })
            .collect();

        // 走 k-NN 评分
        let mut scored: Vec<(f64, &DyeRecipeModel)> = history
            .iter()
            .map(|c| {
                (
                    compute_similarity("BL-301", "棉", Some("活性染料"), c),
                    c,
                )
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<(f64, &DyeRecipeModel)> = scored.into_iter().take(5).collect();
        assert_eq!(top.len(), 5);

        // 颜色完全匹配的相似度应为 1.0 + 0.2 + 0.1 = 1.3
        for (score, _) in &top {
            assert!(
                (*score - MAX_SIMILARITY).abs() < 0.001,
                "完全匹配相似度应为 {}，实际 {}",
                MAX_SIMILARITY,
                score
            );
        }

        // 加权平均：因为所有权重相同，等价于算术平均
        let agg = weighted_average_params(&top).expect("应当能聚合");
        // 温度均值 = (60+61+62+63+64)/5 = 62.0
        assert!(
            (agg.temperature - 62.0).abs() < 0.001,
            "温度均值应为 62.0，实际 {}",
            agg.temperature
        );
        // 时间均值 = (40+42+44+46+48)/5 = 44.0
        assert!(
            (agg.time_minutes - 44.0).abs() < 0.001,
            "时间均值应为 44.0，实际 {}",
            agg.time_minutes
        );
        // pH 均值 = (6.0+6.1+6.2+6.3+6.4)/5 = 6.2
        assert!(
            (agg.ph_value - 6.2).abs() < 0.001,
            "pH 均值应为 6.2，实际 {}",
            agg.ph_value
        );
        // 置信度：5/5 * 1.0（1.3 归一化） = 1.0
        let conf = compute_confidence(&top, 5);
        assert!((conf - 1.0).abs() < 0.001, "5 条全匹配置信度应为 1.0，实际 {}", conf);

        // candidates 转换
        let cands = build_candidates(&top, 10);
        assert_eq!(cands.len(), 5);
        assert!((cands[0].similarity - 1.0).abs() < 0.001);
    }

    /// 测试 3：温度推荐 — 加权平均温度落在合理范围
    /// 验证不同权重的加权平均算法正确性
    #[test]
    fn test_temperature_recommendation() {
        // 3 条历史：50 / 60 / 70，权重 1.0 / 1.3 / 0.5
        let r1 = make_recipe("R-1", "BL-301", "棉", "活性染料", 50.0, 30, 7.0, 10.0);
        let r2 = make_recipe("R-2", "BL-301", "棉", "活性染料", 60.0, 40, 7.0, 10.0);
        let r3 = make_recipe("R-3", "BL-301", "棉", "活性染料", 70.0, 50, 7.0, 10.0);
        let hits: Vec<(f64, &DyeRecipeModel)> = vec![(1.0, &r1), (1.3, &r2), (0.5, &r3)];

        let agg = weighted_average_params(&hits).expect("应当能聚合");
        // 期望温度 = (50*1.0 + 60*1.3 + 70*0.5) / (1.0+1.3+0.5) = 163/2.8 ≈ 58.21
        let expected_temp = 163.0_f64 / 2.8_f64;
        assert!(
            (agg.temperature - expected_temp).abs() < 0.01,
            "加权平均温度应为 {:.2}，实际 {:.2}",
            expected_temp,
            agg.temperature
        );

        // 温度应在合理范围（30-100°C）
        assert!(
            agg.temperature >= 30.0 && agg.temperature <= 100.0,
            "温度应在 30-100°C 之间，实际 {}",
            agg.temperature
        );

        // 期望时间 = (30*1.0 + 40*1.3 + 50*0.5) / 2.8 = 129/2.8 ≈ 46.07
        let expected_time = 129.0_f64 / 2.8_f64;
        assert!(
            (agg.time_minutes - expected_time).abs() < 0.01,
            "加权平均时间应为 {:.2}，实际 {:.2}",
            expected_time,
            agg.time_minutes
        );

        // 时间应在 10-120 min
        assert!(
            agg.time_minutes >= 10.0 && agg.time_minutes <= 120.0,
            "时间应在 10-120 min 之间，实际 {}",
            agg.time_minutes
        );

        // 置信度
        let conf = compute_confidence(&hits, 5);
        assert!(conf > 0.0 && conf <= 1.0, "置信度应在 0-1 之间，实际 {}", conf);
    }

    /// 测试 4：退化路径 — k=0 / 输入异常 / 命中 < 3 时
    /// 全部回退到典型参数表
    #[test]
    fn test_fallback_path() {
        // 4.1 k=0 强制退化
        //   无 hits → 应返回 0.6 置信度
        let empty: Vec<(f64, &DyeRecipeModel)> = vec![];
        let conf_zero = compute_confidence(&empty, 0);
        assert!((conf_zero - 0.6).abs() < 0.001, "空命中置信度应为 0.6");

        // 4.2 命中 < 3 条时
        //   should_use_knn 边界
        assert!(!should_use_knn(0), "0 条应退化");
        assert!(!should_use_knn(1), "1 条应退化");
        assert!(!should_use_knn(2), "2 条应退化");
        assert!(should_use_knn(3), "3 条应走 k-NN");

        // 4.3 输入异常（color_no 全空字符串）
        let r = make_recipe("R-1", "", "棉", "活性染料", 60.0, 45, 7.0, 10.0);
        let s = compute_similarity("BL-301", "棉", Some("活性染料"), &r);
        assert!((s - 0.0).abs() < 0.001, "候选 color 为空时相似度应为 0.0");

        // 4.4 完全不同 color_no → 相似度为 0
        let r2 = make_recipe("R-2", "RD-999", "涤纶", "分散染料", 130.0, 30, 5.5, 8.0);
        let s2 = compute_similarity("BL-301", "棉", Some("活性染料"), &r2);
        assert!((s2 - 0.0).abs() < 0.001, "完全无关候选相似度应为 0.0");

        // 4.5 颜色前缀 3 位匹配 → 0.7
        let r3 = make_recipe("R-3", "BL-999", "棉", "活性染料", 60.0, 45, 7.0, 10.0);
        let s3 = compute_similarity("BL-301", "棉", Some("活性染料"), &r3);
        // 0.7 (color 前缀) + 0.2 (fabric) + 0.1 (dye) = 1.0
        assert!(
            (s3 - 1.0).abs() < 0.001,
            "BL 前缀匹配应为 1.0，实际 {}",
            s3
        );

        // 4.6 典型参数表兜底
        let typical = find_typical_params();
        assert_eq!(typical.time_minutes as i32, TYPICAL_TIME_MINUTES);
        assert!((typical.temperature - TYPICAL_TEMPERATURE).abs() < 0.001);
    }
}

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
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::models::dye_recipe::{Entity as DyeRecipeEntity, Model as DyeRecipeModel};
use crate::utils::error::AppError;

use super::AiAnalysisService;

/// V15 P1 6.2：候选集上限（数据最小化，防止全表扫描）
pub(crate) const RECIPE_CANDIDATE_LIMIT: u64 = 10_000;

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

/// 因子贡献（V15 P2 14.7.1：解释各评分因子的权重与贡献）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorContribution {
    pub factor_name: String,
    pub weight: f64,
    pub contribution: String,
}

/// 相似候选案例（命中 TopK 后的前 10 条）
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
    /// V15 P1 5.3：是否命中缓存（true 表示 5 分钟内相同入参已计算过）
    pub cache_hit: bool,
    /// V15 P1 9.1+9.5：是否为降级结果（true 表示推理超时或模型不可用时返回的兜底结果）
    pub degraded: bool,
    /// V15 P2 14.1.2：原始配方成本（可选，无成本数据时为 None）
    pub original_cost: Option<f64>,
    /// V15 P2 14.1.2：优化后配方成本（可选）
    pub optimized_cost: Option<f64>,
    /// V15 P2 14.1.2：成本变化百分比（正数表示增加，负数表示降低）
    pub cost_delta_percentage: Option<f64>,
    /// V15 P2 14.7.1：评分因子贡献列表
    pub factors: Vec<FactorContribution>,
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

/// 染料-布类配伍性表（V15 P1 1.1：染料配伍性校验）
/// 依据 fabric-industry-research §11.2，染料与布类不匹配（如分散染料用于棉）；会生成无效配方推荐，工艺员采纳后可能导致染色失败、批次报废。；返回 true 表示配伍；false 表示不配伍。
pub(crate) fn is_dye_fabric_compatible(dye_type: &str, fabric_type: &str) -> bool {
    let dye = dye_type.trim().to_lowercase();
    let fabric = fabric_type.trim().to_lowercase();
    if dye.is_empty() || fabric.is_empty() {
        return true;
    }
    let supported: &[&str] = match dye.as_str() {
        "reactive" | "活性" => &[
            "cotton", "棉", "棉布", "rayon", "黏胶", "粘胶", "hemp", "麻",
        ],
        "disperse" | "分散" => &["polyester", "涤纶", "pet", "acetate", "醋酸"],
        "acid" | "酸性" => &[
            "silk", "丝绸", "真丝", "wool", "羊毛", "nylon", "锦纶", "尼龙",
        ],
        "vat" | "还原" => &["cotton", "棉", "棉布", "hemp", "麻"],
        "direct" | "直接" => &["cotton", "棉", "棉布", "rayon", "黏胶", "hemp", "麻"],
        "cationic" | "阳离子" => &["acrylic", "腈纶"],
        "sulfur" | "硫化" => &["cotton", "棉", "棉布"],
        _ => return true,
    };
    supported
        .iter()
        .any(|s| fabric == *s || fabric.contains(s) || s.contains(fabric.as_str()))
}

/// 校验染料与布类配伍性，不配伍时返回 422 业务错误（V15 P1 1.1）
pub(crate) fn validate_dye_fabric_compatibility(
    dye_type: Option<&str>,
    fabric_type: &str,
) -> Result<(), AppError> {
    if let Some(dye) = dye_type {
        if !dye.trim().is_empty() && !is_dye_fabric_compatible(dye, fabric_type) {
            return Err(AppError::validation(format!(
                "染料[{}]与布类[{}]不配伍，请检查配方输入",
                dye, fabric_type
            )));
        }
    }
    Ok(())
}

/// V15 P1 6.1：脱敏配方候选集，掩码 remark 中可能含有的手机号/邮箱/身份证号
/// 算法仅使用 color_no/fabric_type/dye_type/temperature/time/ph/liquor_ratio 字段，；remark 字段不参与算法但会随候选集回写到 candidates_json，须前置脱敏避免 PII 泄露。
pub(crate) fn sanitize_recipe_for_inference(mut recipe: DyeRecipeModel) -> DyeRecipeModel {
    if let Some(remark) = recipe.remarks.take() {
        recipe.remarks = Some(crate::utils::field_mask::mask_text_pii(&remark));
    }
    if let Some(name) = recipe.color_name.take() {
        recipe.color_name = Some(crate::utils::field_mask::mask_text_pii(&name));
    }
    recipe
}

/// 计算两条配方的相似度（0.0 - 1.3）
/// 评分规则：`color_no` 精确（大小写不敏感）相等 → 1.0；`color_no` 前缀 3 位相同（忽略分隔符）→ 0.7；否则 → 0.0；`fabric_type` 精确相等 → +0.2；`dye_type` 精确相等 → +0.1
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

/// 颜色号相似度（仅依赖 color_no 字符串）（标准化时去除常见分隔符 `-` `_` `/` ` `，便于"BL301" 与 "BL-301" 模糊匹配。）
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
}

/// 按相似度加权聚合多条命中配方的参数
pub(crate) fn weighted_average_params(hits: &[(f64, &DyeRecipeModel)]) -> Option<AggregatedParams> {
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
    })
}

/// 内置典型参数表（退化兜底，固定 4 字段）
/// 典型值（兜底，参考规格）：温度：80°C ± 10°C → 默认 80；时间：45min ± 15min → 默认 45；pH：6.0 ± 1.0 → 默认 6.0；浴比：1:8 ± 2 → 默认 8.0
pub(crate) fn find_typical_params() -> AggregatedParams {
    AggregatedParams {
        temperature: TYPICAL_TEMPERATURE,
        time_minutes: TYPICAL_TIME_MINUTES as f64,
        ph_value: TYPICAL_PH,
        liquor_ratio: TYPICAL_LIQUOR_RATIO,
    }
}

/// 计算最终置信度（0.0 - 1.0 归一化）（k-NN 命中：min(命中条数 / K, 1.0) * 平均相似度归一化；退化路径：固定 0.6）
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

/// 将候选集合转换为响应中 `candidates` 字段（取相似度 > 0 的前 10 条，并把原始分数归一化到 0.0-1.0。）
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

/// 计算配方的综合成本估算（基于助剂用量总和 + 能耗参数）
/// 返回 None 表示无成本数据（无助剂信息时无法估算）
fn estimate_recipe_cost(recipe: &DyeRecipeModel) -> Option<f64> {
    let auxiliaries = recipe.auxiliaries.as_ref()?;
    if auxiliaries.is_empty() {
        return None;
    }
    let aux_cost: f64 = auxiliaries
        .iter()
        .map(|a| a.amount.to_f64().unwrap_or(0.0))
        .sum();
    let temp_factor = recipe
        .temperature
        .and_then(|d| d.to_f64())
        .map(|t| t / 80.0)
        .unwrap_or(1.0);
    let time_factor = recipe
        .time_minutes
        .map(|t| t as f64 / 45.0)
        .unwrap_or(1.0);
    Some(round2(aux_cost * temp_factor * time_factor))
}

/// 保留 2 位小数
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// 从相似度评分构建因子贡献列表
fn build_factor_contributions(top: &[(f64, &DyeRecipeModel)]) -> Vec<FactorContribution> {
    if top.is_empty() {
        return Vec::new();
    }
    let n = top.len() as f64;
    let mut color_sum = 0.0_f64;
    let mut fabric_sum = 0.0_f64;
    let mut dye_sum = 0.0_f64;

    for (score, model) in top {
        let color_score = color_similarity("", model.color_no.as_deref().unwrap_or(""));
        let s = *score;
        let c = if color_score > 0.0 { color_score.min(s) } else { 0.0 };
        let f = if s > c + 0.05 { 0.2 } else { 0.0 };
        let d = s - c - f;
        color_sum += c;
        fabric_sum += f;
        dye_sum += d.max(0.0);
    }

    let total = color_sum + fabric_sum + dye_sum;
    if total <= 0.0 {
        return Vec::new();
    }

    vec![
        FactorContribution {
            factor_name: "色号匹配".to_string(),
            weight: round2(color_sum / total),
            contribution: format!("平均色号相似度得分 {:.2}", color_sum / n),
        },
        FactorContribution {
            factor_name: "布类匹配".to_string(),
            weight: round2(fabric_sum / total),
            contribution: if fabric_sum > 0.0 {
                format!("布类匹配贡献 {:.2}", fabric_sum / n)
            } else {
                "布类未匹配".to_string()
            },
        },
        FactorContribution {
            factor_name: "染料匹配".to_string(),
            weight: round2(dye_sum / total),
            contribution: if dye_sum > 0.0 {
                format!("染料类型匹配贡献 {:.2}", dye_sum / n)
            } else {
                "染料类型未匹配".to_string()
            },
        },
    ]
}

/// V15 P2 14.1.2：计算成本对比并生成警告
/// 返回 (original_cost, optimized_cost, cost_delta_percentage, reason_with_warning)
fn compute_cost_comparison(
    top: &[(f64, &DyeRecipeModel)],
    agg: &AggregatedParams,
) -> (Option<f64>, Option<f64>, Option<f64>, String) {
    // 计算原始配方的平均成本（取 top 命中配方的成本均值）
    let costs: Vec<f64> = top
        .iter()
        .filter_map(|(_, m)| estimate_recipe_cost(m))
        .collect();

    if costs.is_empty() {
        return (None, None, None, String::new());
    }

    let original_cost = costs.iter().sum::<f64>() / costs.len() as f64;

    // 用推荐参数估算优化后成本（基于助剂均值 + 推荐温度/时间）
    let avg_aux_cost: f64 = top
        .iter()
        .filter_map(|(_, m)| {
            let aux = m.auxiliaries.as_ref()?;
            Some(aux.iter().map(|a| a.amount.to_f64().unwrap_or(0.0)).sum::<f64>())
        })
        .sum::<f64>()
        / costs.len() as f64;

    let temp_factor = agg.temperature / 80.0;
    let time_factor = agg.time_minutes / 45.0;
    let optimized_cost = round2(avg_aux_cost * temp_factor * time_factor);

    let delta = if original_cost > 0.0 {
        ((optimized_cost - original_cost) / original_cost) * 100.0
    } else {
        0.0
    };
    let cost_delta = round2(delta);

    let reason = if cost_delta > 10.0 {
        format!(
            "⚠ 警告：优化后成本较原始配方增加 {:.1}%，建议人工复核工艺参数",
            cost_delta
        )
    } else {
        String::new()
    };

    (Some(round2(original_cost)), Some(optimized_cost), Some(cost_delta), reason)
}

/// 判断是否需要走 k-NN 路径（命中条数 ≥ 3 才走 k-NN，否则退化）
pub(crate) fn should_use_knn(hit_count: usize) -> bool {
    hit_count >= 3
}

// =====================================================
// Service 实现
// =====================================================

impl AiAnalysisService {
    /// 染色工艺参数智能推荐（k-NN 优先，命中 < 3 或 k=0 回退典型参数表）
    /// V15 P1 5.2：通过 Semaphore permits=10 限制并发，防止 CPU 过载；V15 P1 5.3：通过 moka 缓存（TTL 5min）避免相同入参重复计算；V15 P1 5.1+9.1+9.5：通过 tokio::time::timeout（2s）包装算法执行，；超时或模型不可用时返回降级结果（典型参数表 + degraded=true）。
    pub async fn optimize_recipe(
        &self,
        request: RecipeOptRequest,
    ) -> Result<RecipeOptResponse, AppError> {
        // V15 P1 1.1：染料配伍性校验，不配伍直接返回 422
        validate_dye_fabric_compatibility(request.dye_type.as_deref(), &request.fabric_type)?;

        // V15 P1 5.3：缓存键 = 入参指纹，命中时直接返回（cache_hit=true）
        let cache_key = build_recipe_cache_key(&request);
        if let Some(mut cached) = self.recipe_cache.get(&cache_key).await {
            cached.cache_hit = true;
            return Ok(cached);
        }

        let k = request.k.unwrap_or(5);
        if k == 0 {
            let resp =
                Self::build_fallback_response(0, "k=0，已强制走典型参数表".to_string(), Vec::new());
            self.recipe_cache.insert(cache_key, resp.clone()).await;
            return Ok(resp);
        }

        // V15 P1 5.2：获取并发许可，permit 在 scope 结束时自动释放
        let _permit = self.acquire_inference_permit().await?;

        // V15 P1 5.1+9.5：算法执行包装在 timeout 中，超时返回降级结果
        let timeout_dur = std::time::Duration::from_millis(super::AI_INFERENCE_TIMEOUT_MS);
        let inference_result = tokio::time::timeout(
            timeout_dur,
            self.run_recipe_inference(request, k, cache_key.clone()),
        )
        .await;

        match inference_result {
            Ok(Ok(response)) => Ok(response),
            // V15 P1 9.1：模型不可用（DB 错误等）→ 返回降级结果
            Ok(Err(_e)) => {
                tracing::warn!("AI 工艺优化模型不可用，返回降级结果: {:?}", _e);
                let degraded = Self::build_degraded_response(
                    "AI 推理模型不可用，已降级为典型参数表".to_string(),
                );
                Ok(degraded)
            }
            // V15 P1 5.1+9.5：推理超时 → 返回降级结果
            Err(_elapsed) => {
                tracing::warn!(
                    "AI 工艺优化推理超时（>{}ms），返回降级结果",
                    super::AI_INFERENCE_TIMEOUT_MS
                );
                let degraded = Self::build_degraded_response(format!(
                    "AI 推理超时（>{}ms），已降级为典型参数表",
                    super::AI_INFERENCE_TIMEOUT_MS
                ));
                Ok(degraded)
            }
        }
    }

    /// V15 P1 5.1：实际算法执行（候选拉取 + 评分 + 响应构建 + 缓存写入）（由 `optimize_recipe` 通过 `tokio::time::timeout` 包装调用，超时由外层处理。）
    async fn run_recipe_inference(
        &self,
        request: RecipeOptRequest,
        k: usize,
        cache_key: String,
    ) -> Result<RecipeOptResponse, AppError> {
        let candidates = self.fetch_recipe_candidates().await?;
        let scored = Self::score_and_sort_candidates(&request, &candidates);
        let top: Vec<(f64, &DyeRecipeModel)> = scored.iter().take(k).copied().collect();
        let resp_candidates = build_candidates(&scored, 10);

        let response = if should_use_knn(top.len()) {
            Self::build_knn_response(&top, k, resp_candidates)?
        } else {
            let reason = format!(
                "命中相似案例 {} 条（< 3），已回退到典型参数表（温度{}°C ±10、时间{}min ±15、pH{} ±1、浴比1:{} ±2）",
                top.len(),
                TYPICAL_TEMPERATURE,
                TYPICAL_TIME_MINUTES,
                TYPICAL_PH,
                TYPICAL_LIQUOR_RATIO
            );
            Self::build_fallback_response(top.len(), reason, resp_candidates)
        };
        self.recipe_cache.insert(cache_key, response.clone()).await;
        Ok(response)
    }

    /// 查询近 6 个月未删除的染色配方作为候选集
    /// V15 P1 6.2：数据最小化——限制候选集上限（RECIPE_CANDIDATE_LIMIT）防止全表扫描 OOM；V15 P1 6.1：返回前对 remark/color_name 字段做 PII 脱敏；V15 P1 3.3：order_by_asc(Id) 保证推理结果稳定性。
    async fn fetch_recipe_candidates(&self) -> Result<Vec<DyeRecipeModel>, AppError> {
        let six_months_ago = chrono::Utc::now() - chrono::Duration::days(180);
        let six_months_ago_dt = six_months_ago.naive_utc();
        let raw = DyeRecipeEntity::find()
            .filter(crate::models::dye_recipe::Column::IsDeleted.eq(false))
            .filter(crate::models::dye_recipe::Column::UpdatedAt.gte(six_months_ago_dt))
            .order_by_asc(crate::models::dye_recipe::Column::Id)
            .limit(RECIPE_CANDIDATE_LIMIT)
            .all(&*self.db)
            .await
            .map_err(AppError::from)?;
        Ok(raw.into_iter().map(sanitize_recipe_for_inference).collect())
    }

    /// 计算候选配方的相似度，过滤 0 分并按降序排序
    fn score_and_sort_candidates<'a>(
        request: &RecipeOptRequest,
        candidates: &'a [DyeRecipeModel],
    ) -> Vec<(f64, &'a DyeRecipeModel)> {
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
        scored
    }

    /// 构建 k-NN 路径响应（加权聚合参数 + 置信度）
    fn build_knn_response(
        top: &[(f64, &DyeRecipeModel)],
        k: usize,
        candidates: Vec<RecipeCandidate>,
    ) -> Result<RecipeOptResponse, AppError> {
        let agg = weighted_average_params(top)
            .ok_or_else(|| AppError::internal("工艺推荐：k-NN 加权聚合失败"))?;
        let confidence = compute_confidence(top, k);

        // V15 P2 14.1.2：成本对比计算
        let (original_cost, optimized_cost, cost_delta, mut reason) =
            compute_cost_comparison(top, &agg);

        if reason.is_empty() {
            reason = format!("基于 {} 条相似历史配方（k={}）的加权平均推荐", top.len(), k);
        }

        // V15 P2 14.7.1：构建因子贡献
        let factors = build_factor_contributions(top);

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
            reason,
            candidates,
            cache_hit: false,
            degraded: false,
            original_cost,
            optimized_cost,
            cost_delta_percentage: cost_delta,
            factors,
        })
    }

    /// 构建退化路径响应（典型参数表，置信度 0.6）
    fn build_fallback_response(
        similar_cases: usize,
        reason: String,
        candidates: Vec<RecipeCandidate>,
    ) -> RecipeOptResponse {
        let typical = find_typical_params();
        RecipeOptResponse {
            recommended_params: RecipeParams {
                temperature: typical.temperature,
                time_minutes: typical.time_minutes as i32,
                ph_value: typical.ph_value,
                liquor_ratio: typical.liquor_ratio,
            },
            similar_cases,
            confidence: 0.6,
            source: "fallback".to_string(),
            reason,
            candidates,
            cache_hit: false,
            degraded: false,
            original_cost: None,
            optimized_cost: None,
            cost_delta_percentage: None,
            factors: Vec::new(),
        }
    }

    /// V15 P1 9.1+9.5：构建降级响应（推理超时或模型不可用时使用）
    /// 与 `build_fallback_response` 区别：本方法用于异常场景（非算法退化），；`degraded=true` 标识前端可展示"AI 服务降级"提示，置信度降至 0.3。
    fn build_degraded_response(reason: String) -> RecipeOptResponse {
        let typical = find_typical_params();
        RecipeOptResponse {
            recommended_params: RecipeParams {
                temperature: typical.temperature,
                time_minutes: typical.time_minutes as i32,
                ph_value: typical.ph_value,
                liquor_ratio: typical.liquor_ratio,
            },
            similar_cases: 0,
            confidence: 0.3,
            source: "degraded".to_string(),
            reason,
            candidates: Vec::new(),
            cache_hit: false,
            degraded: true,
            original_cost: None,
            optimized_cost: None,
            cost_delta_percentage: None,
            factors: Vec::new(),
        }
    }
}

/// V15 P1 5.3：构建工艺优化缓存键（入参指纹）（由 color_no/fabric_type/dye_type/k 拼接而成，相同入参 5 分钟内命中缓存。）
fn build_recipe_cache_key(request: &RecipeOptRequest) -> String {
    format!(
        "recipe_opt:{}|{}|{}|{}",
        request.color_no.trim(),
        request.fabric_type.trim(),
        request.dye_type.as_deref().unwrap_or("").trim(),
        request.k.unwrap_or(5),
    )
}

// =====================================================
// 单元测试（不依赖数据库，覆盖纯函数）
// =====================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dye_recipe::AuxiliariesItem;
    // 批次 212 P2-5 修复：master_data 仅测试使用，移入 #[cfg(test)] 避免 Clippy unused import
    use crate::models::status::master_data;
    use rust_decimal::Decimal;

    /// 染色配方测试夹具参数对象
    /// 批次 338 v10 复审 P3 修复：引入参数对象消除 make_recipe 测试夹具的 too_many_arguments 警告。；聚合染色配方构造所需的全部字段，使用生命周期 `&'a str` 借用避免不必要的 to_string()。
    struct RecipeFixture<'a> {
        recipe_no: &'a str,
        color_no: &'a str,
        fabric_type: &'a str,
        dye_type: &'a str,
        temperature: f64,
        time_minutes: i32,
        ph: f64,
        liquor: f64,
    }

    /// 构造一条 `DyeRecipeModel` 测试夹具
    /// 批次 338 v10 复审 P3 修复：签名从 8 参数改为单一参数对象 `RecipeFixture`，；消除 `clippy::too_many_arguments` 警告。
    fn make_recipe(fixture: RecipeFixture<'_>) -> DyeRecipeModel {
        let RecipeFixture {
            recipe_no,
            color_no,
            fabric_type,
            dye_type,
            temperature,
            time_minutes,
            ph,
            liquor,
        } = fixture;
        DyeRecipeModel {
            id: 0,
            recipe_no: recipe_no.to_string(),
            recipe_name: None,
            color_no: Some(color_no.to_string()),
            formula: None,
            temperature: Some(Decimal::try_from(temperature).unwrap_or(Decimal::ZERO)),
            time_minutes: Some(time_minutes),
            status: Some(master_data::ACTIVE.to_string()),
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
                amount: Decimal::try_from(1.5_f64).unwrap_or(Decimal::ZERO),
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

    /// 测试 1：典型参数退化路径（当数据库无匹配（或命中 < 3 条）时，返回内置典型参数表；温度 80°C ± 10、时间 45min ± 15、pH 6.0 ± 1、浴比 1:8 ± 2）
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
        assert!(
            (conf - 0.6).abs() < 0.001,
            "退化置信度应为 0.6，实际 {}",
            conf
        );

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
                make_recipe(RecipeFixture {
                    recipe_no: &format!("R-BL301-{}", i),
                    color_no: "BL-301",
                    fabric_type: "棉",
                    dye_type: "活性染料",
                    temperature: 60.0 + i as f64, // 60, 61, 62, 63, 64
                    time_minutes: 40 + i * 2,     // 40, 42, 44, 46, 48
                    ph: 6.0 + (i as f64) * 0.1,   // 6.0, 6.1, 6.2, 6.3, 6.4
                    liquor: 10.0,
                })
            })
            .collect();

        // 走 k-NN 评分
        let mut scored: Vec<(f64, &DyeRecipeModel)> = history
            .iter()
            .map(|c| (compute_similarity("BL-301", "棉", Some("活性染料"), c), c))
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
        assert!(
            (conf - 1.0).abs() < 0.001,
            "5 条全匹配置信度应为 1.0，实际 {}",
            conf
        );

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
        let r1 = make_recipe(RecipeFixture {
            recipe_no: "R-1",
            color_no: "BL-301",
            fabric_type: "棉",
            dye_type: "活性染料",
            temperature: 50.0,
            time_minutes: 30,
            ph: 7.0,
            liquor: 10.0,
        });
        let r2 = make_recipe(RecipeFixture {
            recipe_no: "R-2",
            color_no: "BL-301",
            fabric_type: "棉",
            dye_type: "活性染料",
            temperature: 60.0,
            time_minutes: 40,
            ph: 7.0,
            liquor: 10.0,
        });
        let r3 = make_recipe(RecipeFixture {
            recipe_no: "R-3",
            color_no: "BL-301",
            fabric_type: "棉",
            dye_type: "活性染料",
            temperature: 70.0,
            time_minutes: 50,
            ph: 7.0,
            liquor: 10.0,
        });
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
            (30.0..=100.0).contains(&agg.temperature),
            "温度应在 30-100°C 之间，实际 {}",
            agg.temperature
        );

        // 期望时间 = (30*1.0 + 40*1.3 + 50*0.5) / 2.8 = 107/2.8 ≈ 38.21
        let expected_time = 107.0_f64 / 2.8_f64;
        assert!(
            (agg.time_minutes - expected_time).abs() < 0.01,
            "加权平均时间应为 {:.2}，实际 {:.2}",
            expected_time,
            agg.time_minutes
        );

        // 时间应在 10-120 min
        assert!(
            (10.0..=120.0).contains(&agg.time_minutes),
            "时间应在 10-120 min 之间，实际 {}",
            agg.time_minutes
        );

        // 置信度
        let conf = compute_confidence(&hits, 5);
        assert!(
            conf > 0.0 && conf <= 1.0,
            "置信度应在 0-1 之间，实际 {}",
            conf
        );
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
        let r = make_recipe(RecipeFixture {
            recipe_no: "R-1",
            color_no: "",
            fabric_type: "棉",
            dye_type: "活性染料",
            temperature: 60.0,
            time_minutes: 45,
            ph: 7.0,
            liquor: 10.0,
        });
        let s = compute_similarity("BL-301", "棉", Some("活性染料"), &r);
        assert!(s.abs() < 0.001, "候选 color 为空时相似度应为 0.0");

        // 4.4 完全不同 color_no → 相似度为 0
        let r2 = make_recipe(RecipeFixture {
            recipe_no: "R-2",
            color_no: "RD-999",
            fabric_type: "涤纶",
            dye_type: "分散染料",
            temperature: 130.0,
            time_minutes: 30,
            ph: 5.5,
            liquor: 8.0,
        });
        let s2 = compute_similarity("BL-301", "棉", Some("活性染料"), &r2);
        assert!(s2.abs() < 0.001, "完全无关候选相似度应为 0.0");

        // 4.5 颜色前缀 3 位匹配 → 0.7
        //   标准化后 "BL301" 与 "BL310" 前 3 位均为 "BL3"，触发 0.7 分
        let r3 = make_recipe(RecipeFixture {
            recipe_no: "R-3",
            color_no: "BL-310",
            fabric_type: "棉",
            dye_type: "活性染料",
            temperature: 60.0,
            time_minutes: 45,
            ph: 7.0,
            liquor: 10.0,
        });
        let s3 = compute_similarity("BL-301", "棉", Some("活性染料"), &r3);
        // 0.7 (color 前缀) + 0.2 (fabric) + 0.1 (dye) = 1.0
        assert!((s3 - 1.0).abs() < 0.001, "BL 前缀匹配应为 1.0，实际 {}", s3);

        // 4.6 典型参数表兜底
        let typical = find_typical_params();
        assert_eq!(typical.time_minutes as i32, TYPICAL_TIME_MINUTES);
        assert!((typical.temperature - TYPICAL_TEMPERATURE).abs() < 0.001);
    }

    /// 测试 5：V15 P1 6.1 配方候选脱敏
    /// remark 中的手机号/邮箱/身份证号应在写入 candidates_json 前被掩码
    #[test]
    fn test_sanitize_recipe_masks_pii() {
        let recipe = DyeRecipeModel {
            id: 0,
            recipe_no: "R-PII-1".to_string(),
            recipe_name: None,
            color_no: Some("BL-301".to_string()),
            formula: None,
            temperature: Some(Decimal::try_from(80.0_f64).unwrap_or(Decimal::ZERO)),
            time_minutes: Some(45),
            status: Some(master_data::ACTIVE.to_string()),
            is_deleted: Some(false),
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            color_code: None,
            color_name: Some("客户张三 13812348888".to_string()),
            fabric_type: Some("棉".to_string()),
            dye_type: Some("活性".to_string()),
            chemical_formula: None,
            ph_value: Some(Decimal::try_from(6.0_f64).unwrap_or(Decimal::ZERO)),
            liquor_ratio: Some(Decimal::try_from(8.0_f64).unwrap_or(Decimal::ZERO)),
            auxiliaries: None,
            version: Some(1),
            parent_recipe_id: None,
            approved_by: None,
            approved_at: None,
            remarks: Some("联系 13812348888 反馈色差".to_string()),
            created_by: None,
        };
        let sanitized = sanitize_recipe_for_inference(recipe);
        let remark = sanitized.remarks.expect("脱敏后 remark 应保留");
        assert!(
            !remark.contains("13812348888"),
            "手机号应被脱敏，实际 {}",
            remark
        );
        assert!(remark.contains("色差"), "非 PII 文本应保留");
        let name = sanitized.color_name.expect("脱敏后 color_name 应保留");
        assert!(!name.contains("13812348888"), "color_name 中手机号应被脱敏");
    }
}

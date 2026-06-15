//! AI 染色工艺优化服务（ai/recipe_opt）
//!
//! 基于 `dye_recipe` 历史数据 + k-NN 相似度算法，向现场工艺员推荐
//! 染色参数（温度 / 时间 / pH / 浴比 / 染料类型 / 助剂）。
//!
//! 算法概要：
//! 1. 取近 6 个月内、未删除的 `dye_recipe` 历史数据作为候选集
//! 2. 对每条历史配方，按 `color_no` / `fabric_type` / `dye_type` 三个维度
//!    计算相似度：
//!    - `color_no` 精确匹配得 1.0；前缀 3 位相同得 0.7；否则 0.0
//!    - `fabric_type` 完全相同 +0.2
//!    - `dye_type` 完全相同 +0.1
//! 3. 取相似度 Top 5，按相似度加权平均得到推荐参数
//! 4. 当有效历史数据 < 3 条时，回退到内置典型参数表
//!
//! 模块内拆出多个纯函数（`compute_similarity` / `weighted_average_params` /
//! `find_typical_params`），单元测试可直接调用，避免依赖数据库。

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
    /// 色号（如 "BL-301"）
    pub color_no: String,
    /// 布类（棉 / 涤纶 / 丝绸 / 羊毛 等）
    pub fabric_type: String,
    /// 染料类型（活性 / 分散 / 酸性 / 还原 等，可选）
    pub dye_type: Option<String>,
    /// 颜色名称（可选，仅用于展示与日志）
    pub color_name: Option<String>,
}

/// 单条助剂 DTO
#[derive(Debug, Clone, Serialize)]
pub struct AuxiliaryDto {
    /// 助剂名称
    pub name: String,
    /// 用量
    pub amount: String,
    /// 单位（如 g/L、% owf）
    pub unit: String,
}

/// 推荐参数主 DTO
#[derive(Debug, Clone, Serialize)]
pub struct RecommendedParams {
    /// 染色温度（°C）
    pub temperature: f64,
    /// 染色时间（分钟）
    pub time_minutes: i32,
    /// 染浴 pH 值
    pub ph_value: f64,
    /// 浴比
    pub liquor_ratio: f64,
    /// 染料类型
    pub dye_type: String,
    /// 助剂清单
    pub auxiliaries: Vec<AuxiliaryDto>,
}

/// 工艺优化推荐响应
#[derive(Debug, Clone, Serialize)]
pub struct RecipeOptResponse {
    /// 推荐参数
    pub recommended_params: RecommendedParams,
    /// 命中的相似历史配方数量
    pub similar_cases: i32,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 来源说明（"k-NN 历史匹配" / "典型参数表"）
    pub source: String,
}

// =====================================================
// 相似度 / 聚合 / 典型参数表（纯函数，可直接单测）
// =====================================================

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
        if !target_fabric.is_empty()
            && c_fabric.eq_ignore_ascii_case(target_fabric)
        {
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
    pub dye_type: String,
    pub auxiliaries: Vec<AuxiliaryDto>,
    pub total_weight: f64,
}

/// 按相似度加权聚合多条命中配方的参数
///
/// 返回的 `AggregatedParams.auxiliaries` 取第一条命中的助剂列表（k-NN
/// 场景下助剂通常高度一致；多源差异聚合留待后续版本处理）。
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
    let mut first_aux: Vec<AuxiliaryDto> = Vec::new();
    let mut picked_dye_type: String = String::new();

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
        if picked_dye_type.is_empty() {
            picked_dye_type = model.dye_type.clone().unwrap_or_default();
        }
        if first_aux.is_empty() {
            if let Some(auxs) = &model.auxiliaries {
                first_aux = auxs
                    .iter()
                    .map(|a| AuxiliaryDto {
                        name: a.name.clone(),
                        amount: a.amount.to_string(),
                        unit: a.unit.clone(),
                    })
                    .collect();
            }
        }
    }

    if weight_sum <= 0.0 {
        return None;
    }

    Some(AggregatedParams {
        temperature: temp_sum / weight_sum,
        time_minutes: time_sum / weight_sum,
        ph_value: ph_sum / weight_sum,
        liquor_ratio: liquor_sum / weight_sum,
        dye_type: picked_dye_type,
        auxiliaries: first_aux,
        total_weight: weight_sum,
    })
}

/// 内置典型参数表（按 fabric_type 关键字匹配）
///
/// 当历史命中 < 3 条时，退化到此表，保证新色号也能给出合理推荐。
pub(crate) fn find_typical_params(fabric_type: &str) -> Option<AggregatedParams> {
    let ft = fabric_type.trim();
    let key = match ft {
        s if s.contains("棉") || s.eq_ignore_ascii_case("cotton") => "cotton",
        s if s.contains("涤") || s.eq_ignore_ascii_case("polyester") => "polyester",
        s if s.contains("丝") || s.eq_ignore_ascii_case("silk") => "silk",
        s if s.contains("羊毛") || s.contains("毛")
            || s.eq_ignore_ascii_case("wool")
            || s.eq_ignore_ascii_case("cashmere") => "wool",
        _ => return None,
    };

    // 典型配方数据：温度(°C) / 时间(min) / pH / 浴比 / 染料类型 / 默认助剂
    let (temp, time, ph, liquor, dye, auxs): (f64, i32, f64, f64, &str, Vec<AuxiliaryDto>) =
        match key {
            "cotton" => (
                60.0,
                45,
                7.0,
                10.0,
                "活性染料",
                vec![AuxiliaryDto {
                    name: "元明粉".to_string(),
                    amount: "50".to_string(),
                    unit: "g/L".to_string(),
                }],
            ),
            "polyester" => (
                130.0,
                30,
                5.5,
                8.0,
                "分散染料",
                vec![AuxiliaryDto {
                    name: "分散剂".to_string(),
                    amount: "1.0".to_string(),
                    unit: "g/L".to_string(),
                }],
            ),
            "silk" => (
                90.0,
                40,
                6.0,
                20.0,
                "酸性染料",
                vec![AuxiliaryDto {
                    name: "匀染剂".to_string(),
                    amount: "0.5".to_string(),
                    unit: "% owf".to_string(),
                }],
            ),
            "wool" => (
                80.0,
                60,
                4.5,
                15.0,
                "酸性染料",
                vec![AuxiliaryDto {
                    name: "醋酸".to_string(),
                    amount: "2.0".to_string(),
                    unit: "% owf".to_string(),
                }],
            ),
            _ => unreachable!(),
        };

    Some(AggregatedParams {
        temperature: temp,
        time_minutes: time as f64,
        ph_value: ph,
        liquor_ratio: liquor,
        dye_type: dye.to_string(),
        auxiliaries: auxs,
        total_weight: 0.0,
    })
}

/// 计算最终置信度
///
/// - k-NN 命中：min(命中条数 / 5, 1.0) * 平均相似度
/// - 退化路径：固定 0.6
pub(crate) fn compute_confidence(hits: &[(f64, &DyeRecipeModel)]) -> f64 {
    if hits.is_empty() {
        return 0.6;
    }
    let n = hits.len() as f64;
    let coverage = (n / 5.0).min(1.0);
    let avg_score = hits.iter().map(|(s, _)| *s).sum::<f64>() / n;
    // 归一化相似度（最大理论值 1.3）
    let normalized = (avg_score / 1.3).clamp(0.0, 1.0);
    (coverage * normalized * 100.0).round() / 100.0
}

// =====================================================
// Service 实现
// =====================================================

impl AiAnalysisService {
    /// 染色工艺参数智能推荐
    ///
    /// 优先使用 k-NN 历史匹配；命中 < 3 条时回退到典型参数表。
    pub async fn optimize_recipe(
        &self,
        request: RecipeOptRequest,
    ) -> Result<RecipeOptResponse, AppError> {
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

        // 取 Top 5
        let top: Vec<(f64, &DyeRecipeModel)> = scored.into_iter().take(5).collect();

        if top.len() >= 3 {
            // 走 k-NN 路径
            let agg = weighted_average_params(&top).ok_or_else(|| {
                AppError::internal("工艺推荐：k-NN 加权聚合失败")
            })?;
            let confidence = compute_confidence(&top);

            Ok(RecipeOptResponse {
                recommended_params: RecommendedParams {
                    temperature: round1(agg.temperature),
                    time_minutes: agg.time_minutes.round() as i32,
                    ph_value: round1(agg.ph_value),
                    liquor_ratio: round1(agg.liquor_ratio),
                    dye_type: agg.dye_type,
                    auxiliaries: agg.auxiliaries,
                },
                similar_cases: top.len() as i32,
                confidence,
                source: "k-NN 历史匹配".to_string(),
            })
        } else {
            // 退化：典型参数表
            let typical = find_typical_params(&request.fabric_type).ok_or_else(|| {
                AppError::validation(format!(
                    "工艺推荐：布类 '{}' 缺少典型参数表，无法推荐",
                    request.fabric_type
                ))
            })?;

            Ok(RecipeOptResponse {
                recommended_params: RecommendedParams {
                    temperature: typical.temperature,
                    time_minutes: typical.time_minutes as i32,
                    ph_value: typical.ph_value,
                    liquor_ratio: typical.liquor_ratio,
                    dye_type: typical.dye_type,
                    auxiliaries: typical.auxiliaries,
                },
                similar_cases: 0,
                confidence: 0.6,
                source: "典型参数表".to_string(),
            })
        }
    }
}

/// 保留 1 位小数
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
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
            recipe_no: "R-TEST".to_string(),
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
            color_name: None,
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

    /// 测试 1：典型棉配方 — 入参棉 + 蓝色，断言推荐温度 60±2、pH 7.0±0.2
    #[test]
    fn test_typical_cotton_recipe() {
        // 历史库准备 5 条棉 + 蓝/活性
        let history: Vec<DyeRecipeModel> = (0..5)
            .map(|i| {
                make_recipe(
                    "BL-301",
                    "棉",
                    "活性染料",
                    58.0 + i as f64,
                    40 + i as i32 * 2,
                    6.8 + (i as f64) * 0.1,
                    10.0,
                )
            })
            .collect();

        // 走 k-NN：颜色精确匹配 + 棉 + 活性 → 全部命中
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

        let agg = weighted_average_params(&top).expect("应当能聚合");
        // 温度均值 = (58+59+60+61+62)/5 = 60.0
        assert!(
            (agg.temperature - 60.0).abs() < 0.001,
            "温度均值应为 60.0，实际 {}",
            agg.temperature
        );
        // pH 均值 = (6.8+6.9+7.0+7.1+7.2)/5 = 7.0
        assert!(
            (agg.ph_value - 7.0).abs() < 0.001,
            "pH 均值应为 7.0，实际 {}",
            agg.ph_value
        );
        // 断言规格 60±2 / 7.0±0.2
        assert!((agg.temperature - 60.0).abs() <= 2.0);
        assert!((agg.ph_value - 7.0).abs() <= 0.2);
    }

    /// 测试 2：颜色完全匹配时相似度为 1.0（基础分）
    #[test]
    fn test_color_match_similarity() {
        let r = make_recipe("BL-301", "棉", "活性染料", 60.0, 45, 7.0, 10.0);
        let s = compute_similarity("BL-301", "棉", Some("活性染料"), &r);
        // 1.0 (color) + 0.2 (fabric) + 0.1 (dye) = 1.3
        assert!(
            (s - 1.3).abs() < 0.001,
            "完全匹配相似度应为 1.3，实际 {}",
            s
        );
    }

    /// 测试 3：温度推荐 — 多条历史数据加权平均
    #[test]
    fn test_temperature_recommendation() {
        // 构造 3 条历史，温度分别为 50 / 60 / 70，权重由相似度分配
        let r1 = make_recipe("BL-301", "棉", "活性染料", 50.0, 30, 7.0, 10.0);
        let r2 = make_recipe("BL-301", "棉", "活性染料", 60.0, 40, 7.0, 10.0);
        let r3 = make_recipe("BL-301", "棉", "活性染料", 70.0, 50, 7.0, 10.0);
        // 手动指定权重：1.0, 1.3, 0.5（模拟 k-NN 评分）
        let hits: Vec<(f64, &DyeRecipeModel)> =
            vec![(1.0, &r1), (1.3, &r2), (0.5, &r3)];

        let agg = weighted_average_params(&hits).expect("应当能聚合");
        // 期望：(50*1.0 + 60*1.3 + 70*0.5) / (1.0+1.3+0.5)
        //     = (50 + 78 + 35) / 2.8 = 163/2.8 = 58.214...
        let expected = 163.0_f64 / 2.8_f64;
        assert!(
            (agg.temperature - expected).abs() < 0.01,
            "加权平均温度应为 {:.2}，实际 {:.2}",
            expected,
            agg.temperature
        );
        // 置信度
        let conf = compute_confidence(&hits);
        assert!(conf > 0.0 && conf <= 1.0, "置信度应在 0~1 之间");
    }

    /// 测试 4：退化路径 — 颜色无历史时返回典型参数表
    #[test]
    fn test_fallback_typical_params() {
        // 没有任何历史命中，走典型参数表
        let typical = find_typical_params("棉").expect("棉必须有典型参数");
        assert!((typical.temperature - 60.0).abs() < 0.001);
        assert_eq!(typical.time_minutes as i32, 45);
        assert!((typical.ph_value - 7.0).abs() < 0.001);
        assert!((typical.liquor_ratio - 10.0).abs() < 0.001);
        assert_eq!(typical.dye_type, "活性染料");

        // 涤纶
        let poly = find_typical_params("涤纶").expect("涤纶必须有典型参数");
        assert!((poly.temperature - 130.0).abs() < 0.001);
        assert_eq!(poly.dye_type, "分散染料");

        // 丝绸
        let silk = find_typical_params("丝绸").expect("丝绸必须有典型参数");
        assert!((silk.temperature - 90.0).abs() < 0.001);

        // 羊毛
        let wool = find_typical_params("羊毛").expect("羊毛必须有典型参数");
        assert!((wool.temperature - 80.0).abs() < 0.001);

        // 未知布类 → None
        assert!(find_typical_params("未知纤维").is_none());

        // 退化场景下 source 文本
        let empty_hits: Vec<(f64, &DyeRecipeModel)> = vec![];
        let conf_empty = compute_confidence(&empty_hits);
        assert!((conf_empty - 0.6).abs() < 0.001, "退化置信度应为 0.6");
    }
}

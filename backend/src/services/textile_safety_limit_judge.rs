//! 纺织品安全限值判定器（GB 18401 三态判定核心，限值一律查 textile_limit_rule 行）
//!
//! 设计红线（裁定原文）：判定按"项目乘安全类别乘产品形态乘条件标志"四维矩阵取行，三态
//! 为考核、放宽、不适用；判定表可维护，判定器不硬编码任何限值数字。本文件里出现的唯一
//! 数字是等级半级编码（词表 `textile_fastness_grade` 同源）与比较器的机械语义，不含限值。
//!
//! 特异度分层（矩阵行如何压中优先级的成文口径，`select_candidates` 是唯一实现处）：
//! - 候选行 = 该项目全部限值行中，safety_category 命中"申报类别或通配 any"、
//!   product_form 命中"申报形态或通配 any"、condition_tag 命中"none 或申报豁免标签"的行。
//! - 分层分 = 带条件标签行记 4 分、精确形态行记 1 分、精确类别行记 1 分；
//!   只取最高分层的全部行（同层可多行，如可分解致癌芳香胺的 forbidden 与 lte 20 并存，
//!   判定取合取：任一行不满足即 fail）。
//! - 带条件行压过无条件行：中间产品带 wet_processing 时 pH 命中放宽行（4.0~10.5 档在
//!   行数据里），不带该标签的中间产品 pH 只有放宽行不匹配、final 行不匹配，候选为空，
//!   `resolve_verdict` 显式报"矩阵无行"，绝不静默套用放宽或收紧到错误档位。
//! - 命中 na 行（ comparator=na，不考核）时直接产出不适用结论，法定第三态不得折算成合格。
//!
//! 消费方：`textile_safety_report_service.rs` 判逐项目实测结果、
//! `textile_safety_submission_service.rs` 判童装加考 measurable 行与汇总整单结论。

use crate::models::textile_limit_rule::{
    ActiveModel as LimitActiveModel, Column as LimitColumn, Entity as LimitRule,
    Model as LimitRuleModel,
};
use crate::models::textile_test_item::Model as TestItemModel;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;

/// 判定维度输入：一次判定所需的申报要素与实测承载（结构体而非长参数列表，调用点可读）。
pub struct JudgeDims<'a> {
    /// 申报安全类别稳定码（GB18401_A、GB18401_B、GB18401_C）
    pub safety_category: &'a str,
    /// 申报产品形态（final、intermediate）
    pub product_form: &'a str,
    /// 申报豁免标签集（送检单 exemption_tags 逗号串拆出的 token 列表，可为空）
    pub condition_tags: &'a [String],
}

/// 实测值承载（与结果表列一一对应，判定器不感知存储，只消费值与 flag）。
#[derive(Clone, Debug, Default)]
pub struct MeasuredInput {
    /// 数值与等级型实测值（等级型已换算为半级编码数值）
    pub value: Option<Decimal>,
    /// 感官与布尔证据型 flag（none、detected）原文
    pub text: Option<String>,
}

/// 候选限值行的判定所需快照（从 `textile_limit_rule::Model` 抽取，纯函数不依赖 ORM 行）。
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateRule {
    pub safety_category: String,
    pub product_form: String,
    pub condition_tag: String,
    pub comparator: String,
    pub limit_low: Option<Decimal>,
    pub limit_high: Option<Decimal>,
}

impl From<&LimitRuleModel> for CandidateRule {
    fn from(r: &LimitRuleModel) -> Self {
        Self {
            safety_category: r.safety_category.clone(),
            product_form: r.product_form.clone(),
            condition_tag: r.condition_tag.clone(),
            comparator: r.comparator.clone(),
            limit_low: r.limit_low,
            limit_high: r.limit_high,
        }
    }
}

/// 单行判定结果（三态加"输入不足"：输入缺失不是不合格，是还不能判，必须显式区分）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOutcome {
    Pass,
    Fail,
    /// 本行声明该项目不适用
    NotApplicable,
}

impl RowOutcome {
    /// 判定结果到 `textile_verdict` 词表 token 的换算（唯一映射处，调用方不得手写字符串）。
    pub fn into_verdict_token(self) -> &'static str {
        use crate::models::status::textile_verdict as v;
        match self {
            RowOutcome::Pass => v::PASS,
            RowOutcome::Fail => v::FAIL,
            RowOutcome::NotApplicable => v::NOT_APPLICABLE,
        }
    }
}

/// 特异度分层分（判据见文件头，唯一实现处）：条件标签行 4 分、精确形态行 1 分、
/// 精确类别行 1 分；带条件行压过无条件行，精确压过通配。
pub fn tier_score(rule: &CandidateRule) -> u8 {
    use crate::models::status::textile_limit_scope as scope;
    let mut score = 0u8;
    if rule.condition_tag != scope::TAG_NONE {
        score += 4;
    }
    if rule.product_form != scope::FORM_ANY {
        score += 1;
    }
    if rule.safety_category != scope::CATEGORY_ANY {
        score += 1;
    }
    score
}

/// 从该项目全部限值行中筛出候选并按特异度取最高分层。
///
/// 入参 rules 为该 item_id 的所有限值行（调用方按 item 一次取回后可复用切片）。
/// 返回最高分层的全部行；无任何候选行时返回空 Vec，由调用方 `resolve_verdict` 报错点名。
pub fn select_candidates(rules: &[CandidateRule], dims: &JudgeDims<'_>) -> Vec<CandidateRule> {
    use crate::models::status::textile_limit_scope as scope;
    let candidates: Vec<CandidateRule> = rules
        .iter()
        .filter(|r| {
            let cat_ok = r.safety_category == dims.safety_category
                || r.safety_category == scope::CATEGORY_ANY;
            let form_ok = r.product_form == dims.product_form || r.product_form == scope::FORM_ANY;
            let tag_ok = r.condition_tag == scope::TAG_NONE
                || dims.condition_tags.iter().any(|t| t == &r.condition_tag);
            cat_ok && form_ok && tag_ok
        })
        .cloned()
        .collect();
    let best = candidates.iter().map(tier_score).max();
    match best {
        Some(score) => candidates
            .into_iter()
            .filter(|r| tier_score(r) == score)
            .collect(),
        None => Vec::new(),
    }
}

/// 单行比较器语义（机械实现，不含限值数字；限值一律来自行数据）。
fn judge_row(rule: &CandidateRule, measured: &MeasuredInput) -> Result<RowOutcome, AppError> {
    use crate::models::status::{textile_comparator as cmp, textile_flag_value as flag};
    match rule.comparator.as_str() {
        cmp::NOT_APPLICABLE => Ok(RowOutcome::NotApplicable),
        cmp::LTE => {
            let v = require_value(rule, measured)?;
            let high = rule.limit_high.ok_or_else(|| {
                AppError::business(format!(
                    "限值行 {} 声明 lte 却缺 limit_high，判定表数据不完整，请维护修正",
                    rule_debug(rule)
                ))
            })?;
            Ok(if v <= high {
                RowOutcome::Pass
            } else {
                RowOutcome::Fail
            })
        }
        cmp::GTE => {
            let v = require_value(rule, measured)?;
            let low = rule.limit_low.ok_or_else(|| {
                AppError::business(format!(
                    "限值行 {} 声明 gte 却缺 limit_low，判定表数据不完整，请维护修正",
                    rule_debug(rule)
                ))
            })?;
            Ok(if v >= low {
                RowOutcome::Pass
            } else {
                RowOutcome::Fail
            })
        }
        cmp::BETWEEN => {
            let v = require_value(rule, measured)?;
            let (Some(low), Some(high)) = (rule.limit_low, rule.limit_high) else {
                return Err(AppError::business(format!(
                    "限值行 {} 声明 between 却缺上下界，判定表数据不完整，请维护修正",
                    rule_debug(rule)
                )));
            };
            Ok(if v >= low && v <= high {
                RowOutcome::Pass
            } else {
                RowOutcome::Fail
            })
        }
        cmp::MUST_NONE => {
            // 感官与布尔证据型：flag 必须是词表内取值，其它文字（含空串）不是可判定输入
            match measured.text.as_deref() {
                Some(t) if t == flag::NONE => Ok(RowOutcome::Pass),
                Some(t) if t == flag::DETECTED => Ok(RowOutcome::Fail),
                _ => Err(input_incomplete(
                    rule,
                    "必须为无型项目要求实测 flag 为 none 或 detected",
                )),
            }
        }
        cmp::FORBIDDEN => match measured.text.as_deref() {
            Some(t) if t == flag::NONE => Ok(RowOutcome::Pass),
            Some(t) if t == flag::DETECTED => Ok(RowOutcome::Fail),
            _ => Err(input_incomplete(
                rule,
                "禁用型项目要求实测 flag 为 none 或 detected",
            )),
        },
        other => Err(AppError::business(format!(
            "限值行 {} 携带词表外比较器 {}，判定表数据被旁路写坏，请核实修正",
            rule_debug(rule),
            other
        ))),
    }
}

/// 合取判定（同分层多行全部满足才合格；任一行 na 即不适用压顶）。
///
/// 空分层不可调用本函数——候选为空的报错在 `resolve_verdict`（需要 item 上下文拼消息）。
pub fn judge_conjunction(
    rows: &[CandidateRule],
    measured: &MeasuredInput,
) -> Result<RowOutcome, AppError> {
    let mut outcome = RowOutcome::Pass;
    for rule in rows {
        match judge_row(rule, measured)? {
            RowOutcome::NotApplicable => return Ok(RowOutcome::NotApplicable),
            RowOutcome::Fail => outcome = RowOutcome::Fail,
            RowOutcome::Pass => {}
        }
    }
    Ok(outcome)
}

fn require_value(rule: &CandidateRule, measured: &MeasuredInput) -> Result<Decimal, AppError> {
    measured
        .value
        .ok_or_else(|| input_incomplete(rule, "数值与等级型项目必须录入实测数值后才能判定"))
}

fn input_incomplete(rule: &CandidateRule, why: &str) -> AppError {
    AppError::validation_displayable(format!(
        "限值行 {} 无法判定：{}（判定结果复位为待判定而非猜测结论）",
        rule_debug(rule),
        why
    ))
}

fn rule_debug(rule: &CandidateRule) -> String {
    format!(
        "(类别 {}, 形态 {}, 条件 {}, 比较器 {})",
        rule.safety_category, rule.product_form, rule.condition_tag, rule.comparator
    )
}

/// 判定器对外主入口：按四维取矩阵行并合取判定，返回 `textile_verdict` 词表值。
///
/// 入参：item 为项目主数据行（提供编码与名称拼错误上下文）；rules_for_item 为该项目的
/// 全部限值行；dims 为送检单申报要素；measured 为结果行实测承载。
/// 传给谁：报告服务写结果行 verdict、送检服务写童装核查 verdict。
/// 三态映射：Pass 到 pass、Fail 到 fail、NotApplicable 到 na；不返回 pending——
/// 待判定是"没判"的状态，由调用方在输入缺失或尚未录入时直接落词表常量，不经过本函数。
/// 失败面：候选为空或输入不足或判定表数据不完整一律 Err，fail-visible，不静默兜底。
pub async fn resolve_verdict(
    db: &Arc<DatabaseConnection>,
    item: &TestItemModel,
    dims: &JudgeDims<'_>,
    measured: &MeasuredInput,
) -> Result<String, AppError> {
    let rules = fetch_rules_for_item(db, item.id).await?;
    let rules_snapshot: Vec<CandidateRule> = rules.iter().map(CandidateRule::from).collect();
    let selected = select_candidates(&rules_snapshot, dims);
    if selected.is_empty() {
        return Err(AppError::business(format!(
            "项目 {}（{}）在安全类别 {}、产品形态 {}、条件标签 {:?} 组合下无限值行，判定矩阵未覆盖该组合，请在限值判定表维护后重试，判定器拒绝凭空造限值",
            item.item_code,
            item.item_name,
            dims.safety_category,
            dims.product_form,
            dims.condition_tags
        )));
    }
    let outcome = judge_conjunction(&selected, measured)?;
    Ok(match outcome {
        RowOutcome::Pass => crate::models::status::textile_verdict::PASS.to_string(),
        RowOutcome::Fail => crate::models::status::textile_verdict::FAIL.to_string(),
        RowOutcome::NotApplicable => {
            crate::models::status::textile_verdict::NOT_APPLICABLE.to_string()
        }
    })
}

/// 按 item_id 一次取回该项目的全部限值行（调用方对同项目多行判定可复用结果）。
pub async fn fetch_rules_for_item(
    db: &Arc<DatabaseConnection>,
    item_id: i32,
) -> Result<Vec<LimitRuleModel>, AppError> {
    LimitRule::find()
        .filter(LimitColumn::ItemId.eq(item_id))
        .all(db.as_ref())
        .await
        .map_err(AppError::from)
}

/// 判定表新行的构造（功能：把一条限值行的全部业务列与创建更新时间装配为待插入行；
/// 调用方：主数据服务的新增限值行通道；入参已由其完成比较器与维度词表校验；
/// 存什么：项目、安全类别、产品形态、条件标签、比较器、上下限、条款引用与备注；
/// 存哪里：纺织品安全限值规则表）。
pub fn build_limit_active(
    item_id: i32,
    safety_category: &str,
    product_form: &str,
    condition_tag: &str,
    comparator: &str,
    limit_low: Option<Decimal>,
    limit_high: Option<Decimal>,
    clause_ref: &str,
    remark: &str,
) -> LimitActiveModel {
    let now = chrono::Utc::now();
    LimitActiveModel {
        item_id: ActiveValue::Set(item_id),
        safety_category: ActiveValue::Set(safety_category.to_string()),
        product_form: ActiveValue::Set(product_form.to_string()),
        condition_tag: ActiveValue::Set(condition_tag.to_string()),
        comparator: ActiveValue::Set(comparator.to_string()),
        limit_low: ActiveValue::Set(limit_low),
        limit_high: ActiveValue::Set(limit_high),
        clause_ref: ActiveValue::Set(clause_ref.to_string()),
        remark: ActiveValue::Set(remark.to_string()),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn rule(
        cat: &str,
        form: &str,
        tag: &str,
        cmp: &str,
        low: Option<&str>,
        high: Option<&str>,
    ) -> CandidateRule {
        CandidateRule {
            safety_category: cat.to_string(),
            product_form: form.to_string(),
            condition_tag: tag.to_string(),
            comparator: cmp.to_string(),
            limit_low: low.map(dec),
            limit_high: high.map(dec),
        }
    }

    fn dims<'a>(cat: &'a str, form: &'a str, tags: &'a [String]) -> JudgeDims<'a> {
        JudgeDims {
            safety_category: cat,
            product_form: form,
            condition_tags: tags,
        }
    }

    /// 甲醛 A 类 25 mg/kg 对 lte 20 必判 fail；15 必判 pass（钉比较器方向）。
    #[test]
    fn lte_both_directions() {
        let rows = vec![rule("GB18401_A", "any", "none", "lte", None, Some("20"))];
        let tags: Vec<String> = vec![];
        let fail = judge_conjunction(
            &select_candidates(&rows, &dims("GB18401_A", "final", &tags)),
            &MeasuredInput {
                value: Some(dec("25")),
                text: None,
            },
        )
        .unwrap();
        assert_eq!(fail, RowOutcome::Fail);
        let pass = judge_conjunction(
            &select_candidates(&rows, &dims("GB18401_A", "final", &tags)),
            &MeasuredInput {
                value: Some(dec("15")),
                text: None,
            },
        )
        .unwrap();
        assert_eq!(pass, RowOutcome::Pass);
    }

    /// pH 区间闭区间边界值 7.5 合格、7.6 不合格（钉 between 含边界）。
    #[test]
    fn between_inclusive_bounds() {
        let rows = vec![rule(
            "GB18401_A",
            "final",
            "none",
            "between",
            Some("4.0"),
            Some("7.5"),
        )];
        let tags: Vec<String> = vec![];
        let sel = select_candidates(&rows, &dims("GB18401_A", "final", &tags));
        assert_eq!(sel.len(), 1);
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("7.5")),
                    text: None
                }
            )
            .unwrap(),
            RowOutcome::Pass
        );
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("7.6")),
                    text: None
                }
            )
            .unwrap(),
            RowOutcome::Fail
        );
    }

    /// 中间产品加 wet_processing 时放宽行（4.0~10.5）特异度压过 final 行（不匹配），
    /// pH 10.0 判 pass；不带该标签的中间产品 pH 候选为空，绝不静默套用放宽档。
    #[test]
    fn ph_relaxation_requires_matching_condition() {
        let rows = vec![
            rule(
                "GB18401_A",
                "final",
                "none",
                "between",
                Some("4.0"),
                Some("7.5"),
            ),
            rule(
                "any",
                "intermediate",
                "wet_processing",
                "between",
                Some("4.0"),
                Some("10.5"),
            ),
        ];
        let tags = vec!["wet_processing".to_string()];
        let sel = select_candidates(&rows, &dims("GB18401_A", "intermediate", &tags));
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].limit_high, Some(dec("10.5")));
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("10.0")),
                    text: None
                }
            )
            .unwrap(),
            RowOutcome::Pass
        );
        let no_tags: Vec<String> = vec![];
        assert!(select_candidates(&rows, &dims("GB18401_A", "intermediate", &no_tags)).is_empty());
    }

    /// 色牢度豁免标签命中时 na 行（分层 4 分）压过等级下限行，判定为不适用而非合格。
    #[test]
    fn fastness_exemption_wins_over_grade_floor() {
        let rows = vec![
            rule("GB18401_A", "any", "none", "gte", Some("3.5"), None),
            rule("any", "any", "plain_or_bleached", "na", None, None),
        ];
        let tags = vec!["plain_or_bleached".to_string()];
        let sel = select_candidates(&rows, &dims("GB18401_A", "final", &tags));
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].comparator, "na");
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("2.0")),
                    text: None
                }
            )
            .unwrap(),
            RowOutcome::NotApplicable
        );
        // 无豁免标签时回到下限行，2.0 低于 3.5 判 fail
        let no_tags: Vec<String> = vec![];
        let sel2 = select_candidates(&rows, &dims("GB18401_A", "final", &no_tags));
        assert_eq!(
            judge_conjunction(
                &sel2,
                &MeasuredInput {
                    value: Some(dec("2.0")),
                    text: None
                }
            )
            .unwrap(),
            RowOutcome::Fail
        );
    }

    /// 耐唾液 B 类法定不考核：na 行（类别精确）是唯一候选，产出 na 而非 pending 或 pass。
    #[test]
    fn saliva_b_is_statutory_na() {
        let rows = vec![rule("GB18401_B", "any", "none", "na", None, None)];
        let tags: Vec<String> = vec![];
        let sel = select_candidates(&rows, &dims("GB18401_B", "final", &tags));
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: None,
                    text: None
                }
            )
            .unwrap(),
            RowOutcome::NotApplicable
        );
    }

    /// 芳香胺 forbidden 与 lte 20 同层并存，判定取合取：数值 15 但 flag detected 仍 fail；
    /// 数值 25 且 flag none 也 fail；两者都满足才 pass。
    #[test]
    fn azo_conjunction_needs_both_rows_satisfied() {
        let rows = vec![
            rule("any", "any", "none", "forbidden", None, None),
            rule("any", "any", "none", "lte", None, Some("20")),
        ];
        let tags: Vec<String> = vec![];
        let sel = select_candidates(&rows, &dims("GB18401_A", "final", &tags));
        assert_eq!(sel.len(), 2);
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("15")),
                    text: Some("detected".into())
                }
            )
            .unwrap(),
            RowOutcome::Fail
        );
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("25")),
                    text: Some("none".into())
                }
            )
            .unwrap(),
            RowOutcome::Fail
        );
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: Some(dec("15")),
                    text: Some("none".into())
                }
            )
            .unwrap(),
            RowOutcome::Pass
        );
    }

    /// 输入不足不是不合格：lte 行缺数值、must_none 行缺 flag 都返回 Err 而非 Fail 或 Pass。
    #[test]
    fn incomplete_input_is_error_not_verdict() {
        let rows = vec![rule("any", "any", "none", "lte", None, Some("90"))];
        let tags: Vec<String> = vec![];
        let sel = select_candidates(&rows, &dims("GB18401_A", "final", &tags));
        assert!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: None,
                    text: None
                }
            )
            .is_err()
        );
        let odor = vec![rule("any", "any", "none", "must_none", None, None)];
        let sel = select_candidates(&odor, &dims("GB18401_C", "final", &tags));
        assert!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: None,
                    text: Some("鱼腥味".into())
                }
            )
            .is_err()
        );
        assert_eq!(
            judge_conjunction(
                &sel,
                &MeasuredInput {
                    value: None,
                    text: Some("none".into())
                }
            )
            .unwrap(),
            RowOutcome::Pass
        );
    }

    /// 通配行与精确行并存时精确压上（甲醛 A 精确 20 与通配假设 300 并存，实测 25 对精确行 fail）。
    #[test]
    fn exact_category_beats_wildcard() {
        let rows = vec![
            rule("any", "any", "none", "lte", None, Some("300")),
            rule("GB18401_A", "any", "none", "lte", None, Some("20")),
        ];
        let tags: Vec<String> = vec![];
        let sel = select_candidates(&rows, &dims("GB18401_A", "final", &tags));
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].limit_high, Some(dec("20")));
    }
}

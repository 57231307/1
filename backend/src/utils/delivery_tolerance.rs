//! 交货数量容差（超收 / 短量允收）解析
//!
//! 语义：合同/订单「约定交货数量」与「实际交付数量」之间允许偏差的**允收区间**，
//! 行业惯例来自 UCP600 第 31 条 c 款——对**非按包装单位计量**的货物（纺织面料按米、
//! 公斤计量即属此类）默认允许 ±5%；按包装/计件单位（件、套、双…）交付的成品无此
//! 惯例，默认 0%（须精确）；合同数量含「约 (about)」时放宽至 ±10%。
//!
//! ⚠️ **与 `crate::utils::dual_unit_converter` 的 0.5% 严格区分**：
//! - `dual_unit_converter::validate_dual_unit` 的 `0.005`（0.5%）是「米 ↔ 公斤」
//!   录入自洽性校验（同一行的两个计量单位换算后差异是否过大，属**数据录入质量**），
//!   单位是比例，硬编码 0.005，语义是「同一数量两种表达是否一致」。
//! - 本模块的 5%/0%/10% 是「实际交付量 vs 约定量」的**允收区间**，单位为百分比数值，
//!   语义是「交付差多少算正常」。
//! 二者量纲、语义、使用点均不同，**禁止复用同一常量 / 同一配置项**；本模块独立命名，
//! 与 dual_unit 无任何引用关系。
//!
//! 解析优先级：**行显式值 > 品类默认 > 全局默认**（详见 [`resolve_tolerance_pct`]）。
//! 「约 → 10%」在当前数据模型无「约」标志列的情况下，由业务在行显式值上直接写入 10.00
//! 实现；行显式值优先级最高，天然覆盖此规则。

use rust_decimal::Decimal;

/// 行级交付容差百分比（`quantity_tolerance_pct`）合法区间**下界**：0。
///
/// 单一真源——`so`、`po` 各自的 `validate_quantity_tolerance_pct`、以及销售合同
/// 创建入参校验均引用本常量，杜绝字面量在多处漂移。语义仍是闭区间 `0 <= v <= 100`，
/// `None`（不覆盖）由各调用方跳过。`Decimal::ZERO` 是库内既有 const，故本项可为 `const`。
pub const TOLERANCE_PCT_MIN: Decimal = Decimal::ZERO;

/// 行级交付容差百分比合法区间**上界**：100。
///
/// 与 [`TOLERANCE_PCT_MIN`] 同为单一真源。`Decimal::new(100, 0)` 非 `const fn`，
/// 无法在 `const` 上下文就地构造；改用库内 `const fn` [`Decimal::from_parts`]，
/// 其参数 `(lo=100, mid=0, hi=0, negative=false, scale=0)` 与 `Decimal::new(100, 0)`
/// 的内存表示逐位等价（即数值 100、标度 0），故取值完全一致。
pub const TOLERANCE_PCT_MAX: Decimal = Decimal::from_parts(100, 0, 0, false, 0);

/// 面料 / 按量计价（米、公斤等非包装计量）默认允收百分比：±5%
/// 本纺织 ERP 以面料按量计价为行业常态，故该项同时作为全局默认。
pub fn fabric_default_pct() -> Decimal {
    Decimal::new(500, 2) // 5.00
}

/// 计件成品（按件/套/双等包装单位交付）默认允收百分比：0%（须精确交付）
pub fn piece_default_pct() -> Decimal {
    Decimal::ZERO // 0.00
}

/// 全局默认允收百分比（无任何更具体信号时的兜底品类）。
/// 本 ERP 面料按量计价为主，全局默认取面料 5%（Q2 决策：行业推荐 B 全局默认 ±5%）。
pub fn global_default_pct() -> Decimal {
    fabric_default_pct()
}

/// 按量计价（面料按米/公斤等计量）单位词表——命中即品类默认 5%。
const BY_MEASURE_UNITS: &[&str] = &[
    "米",
    "公尺",
    "m",
    "千米",
    "km",
    "码",
    "yard",
    "yd",
    "公斤",
    "千克",
    "kg",
    "克",
    "g",
    "磅",
    "lb",
    "升",
    "l",
    "吨",
    "t",
    "平方米",
    "㎡",
];

/// 计件 / 包装单位词表——命中即品类默认 0%。
const BY_COUNT_UNITS: &[&str] = &[
    "件", "个", "套", "双", "打", "dozen", "支", "张", "片", "条", "包", "箱", "pc", "pcs", "ea",
    "set", "ctn", "roll", "卷",
];

fn unit_matches(units: &[&str], unit: &str) -> bool {
    let normalized = unit.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }
    units
        .iter()
        .any(|candidate| candidate.trim().to_lowercase() == normalized)
}

/// 依据计量单位判定品类默认容差百分比；未知/空单位返回 `None`（交由全局默认兜底）。
pub fn category_default_pct(unit: Option<&str>) -> Option<Decimal> {
    let u = unit?.trim();
    if u.is_empty() {
        return None;
    }
    if unit_matches(BY_MEASURE_UNITS, u) {
        Some(fabric_default_pct())
    } else if unit_matches(BY_COUNT_UNITS, u) {
        Some(piece_default_pct())
    } else {
        None
    }
}

/// 解析行有效允收容差百分比。
///
/// 优先级：`行显式值 > 品类默认（按计量单位）> 全局默认（面料 5%）`。
/// - `row_explicit`：订单行 / 合同行上显式写入的 `quantity_tolerance_pct`（NULL 传 `None`）。
/// - `unit`：该行计量单位（面料为 米/公斤，成品为 件/套），用于品类默认判定；无法取得传 `None`。
pub fn resolve_tolerance_pct(row_explicit: Option<Decimal>, unit: Option<&str>) -> Decimal {
    if let Some(explicit) = row_explicit {
        return explicit;
    }
    if let Some(category) = category_default_pct(unit) {
        return category;
    }
    global_default_pct()
}

/// 由容差百分比推导约定数量 `ordered` 的允收上下界 `(下界, 上界)`。
///
/// `pct` 为百分比数值（如 5.00 表示 ±5%）：下界 = ordered×(1−pct/100)，上界 = ordered×(1+pct/100)。
pub fn tolerance_bounds(ordered: Decimal, pct: Decimal) -> (Decimal, Decimal) {
    let factor = pct / Decimal::from(100_i32);
    let lower = ordered * (Decimal::ONE - factor);
    let upper = ordered * (Decimal::ONE + factor);
    (lower, upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn pct(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn row_explicit_wins_over_category_and_global() {
        // 行显式 3.00 覆盖面料默认 5.00
        assert_eq!(
            resolve_tolerance_pct(Some(pct("3.00")), Some("米")),
            pct("3.00")
        );
    }

    #[test]
    fn fabric_unit_resolves_to_five_percent() {
        assert_eq!(resolve_tolerance_pct(None, Some("米")), pct("5.00"));
        assert_eq!(resolve_tolerance_pct(None, Some("KG")), pct("5.00"));
        assert_eq!(resolve_tolerance_pct(None, Some(" 公斤 ")), pct("5.00"));
    }

    #[test]
    fn piece_unit_resolves_to_zero_percent() {
        assert_eq!(resolve_tolerance_pct(None, Some("件")), pct("0.00"));
        assert_eq!(resolve_tolerance_pct(None, Some("PCS")), pct("0.00"));
    }

    #[test]
    fn unknown_unit_falls_back_to_global_fabric_default() {
        assert_eq!(resolve_tolerance_pct(None, Some("未知单位")), pct("5.00"));
        assert_eq!(resolve_tolerance_pct(None, None), pct("5.00"));
        assert_eq!(resolve_tolerance_pct(None, Some("   ")), pct("5.00"));
    }

    #[test]
    fn bounds_symmetric_for_five_percent() {
        let (lower, upper) = tolerance_bounds(pct("1000"), pct("5.00"));
        assert_eq!(lower, pct("950"));
        assert_eq!(upper, pct("1050"));
    }

    #[test]
    fn zero_tolerance_bounds_equal_ordered() {
        let (lower, upper) = tolerance_bounds(pct("100"), pct("0.00"));
        assert_eq!(lower, pct("100"));
        assert_eq!(upper, pct("100"));
    }
}

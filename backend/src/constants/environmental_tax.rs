//! 环保税——污染当量值（法定不可调值）唯一来源
//!
//! 依据：《中华人民共和国环境保护税法》附表《环境保护税税目税额表》中
//! 印染行业适用水污染物/大气污染物项目的第一类污染当量值，以及固废（污泥）口径。
//!
//! **分源原则**
//! - 本模块只承载**法定不可调值**（污染当量值），任何地方不得重新定义或改数
//!   （与状态词表"写入方即权威"同一同源范式；`services/environmental_tax_service.rs`
//!   只引用本模块，不得内联数字字面量）。
//! - **地方可变值**（适用税额，每污染当量 1.2–12 元、由省级在法定幅度内确定）
//!   属部署配置，见 `crate::config::settings::AppSettings::env_tax_rate_per_equivalent`，
//!   绝不在本模块给出默认值。
//!
//! 单位口径：
//! 排放量统一以 kg 计入公式（污泥按吨计征，法定当量值 1 吨在kg口径下即 1000）。
//! 未在本表登记的污染物**必须显式拒绝计税**（`statutory_pollution_equivalent` 返回
//! `None`，调用方报错），禁止按 1kg 或任何估算值兜底继续计算。

use rust_decimal::Decimal;

// 说明：rust_decimal 1.42.1 的 `Decimal::new(weight, scale)` 不是 const fn，
// 常量场景改用等价的 const 构造 `Decimal::from_parts(lo, mid, hi, negative, scale)`
// （96 位无符号有效数字 + 小数位数），数值与各常量 doc 标注的 `Decimal::new(w, s)` 相等。

/// COD（化学需氧量）污染当量值：1kg（法定不可调；= Decimal::new(1, 0)）
pub const COD_POLLUTION_EQUIVALENT_KG: Decimal = Decimal::from_parts(1, 0, 0, false, 0);

/// 氨氮污染当量值：0.5kg（法定不可调；= Decimal::new(5, 1)）
pub const AMMONIA_NITROGEN_POLLUTION_EQUIVALENT_KG: Decimal =
    Decimal::from_parts(5, 0, 0, false, 1);

/// VOCs（挥发性有机物）污染当量值：0.5kg（法定不可调；= Decimal::new(5, 1)）
pub const VOCS_POLLUTION_EQUIVALENT_KG: Decimal = Decimal::from_parts(5, 0, 0, false, 1);

/// 污泥（固废）污染当量值：1 吨；本仓计算口径排放量以 kg 计，故取 1000（法定不可调；= Decimal::new(1000, 0)）
pub const SLUDGE_POLLUTION_EQUIVALENT_KG: Decimal = Decimal::from_parts(1000, 0, 0, false, 0);

/// 按污染物名称查法定污染当量值（kg 口径）。
///
/// 返回 `None` = 该污染物**未在法定登记表内**——调用方必须显式拒绝计税
/// （校验错误），禁止回退任何默认当量值继续算出看似正常的税额。
/// 名称匹配含大小写别名，取值只来自本模块常量。
pub fn statutory_pollution_equivalent(pollutant_name: &str) -> Option<Decimal> {
    match pollutant_name {
        "COD" | "cod" => Some(COD_POLLUTION_EQUIVALENT_KG),
        "氨氮" | "NH3-N" => Some(AMMONIA_NITROGEN_POLLUTION_EQUIVALENT_KG),
        "VOCs" | "vocs" => Some(VOCS_POLLUTION_EQUIVALENT_KG),
        "污泥" | "sludge" => Some(SLUDGE_POLLUTION_EQUIVALENT_KG),
        _ => None,
    }
}

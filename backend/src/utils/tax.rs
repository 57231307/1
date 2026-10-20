//! 增值税税额计算内核——税率单位口径的唯一真源
//!
//! 单位口径：全仓税率一律以**百分数**表达、传输与存储（`13` 表示 13%，`0` 表示
//! 免税或无进项税），与落库列 `purchase_order_item.tax_percent`、
//! `sales_order_item.tax_percent`、`sales_quotations.tax_rate` 同单位。
//! 传入小数 `0.13` 会被按 0.13% 计算，税额量级错 100 倍，因此所有税额/含税价
//! 计算只能经本模块出，禁止各域再写一套 `× 0.13` 或 `÷ 100` 的散装公式。
//!
//! 本模块**不含任何税率档位**：档位值（13/9/10/0…）只能由调用方从真实来源取得
//! （人工录入值、上游单据已存值、部署配置），取不到必须由调用方点名报错，
//! 禁止在内核或调用方回落任何默认档位。
//!
//! 入参约定：`tax_percent` 为百分数口径税率；`amount` 为不含税金额；
//! 返回值供各单据服务直接写入税额列。

use rust_decimal::Decimal;

/// 百分之一（税率换算因子）。
/// `Decimal::from` 非 const fn，常量场景用等价的 const 构造 `from_parts`
/// （与 `constants::environmental_tax` 同一口径）。
const PERCENT: Decimal = Decimal::from_parts(100, 0, 0, false, 0);

/// 税额 = 不含税金额 × 税率% ÷ 100，按 2 位小数收敛。
pub fn tax_amount(amount: Decimal, tax_percent: Decimal) -> Decimal {
    (amount * tax_percent / PERCENT).round_dp(2)
}

/// 含税单价 = 单价 × (1 + 税率% ÷ 100)，按 2 位小数收敛。
pub fn unit_price_with_tax(unit_price: Decimal, tax_percent: Decimal) -> Decimal {
    (unit_price * (Decimal::ONE + tax_percent / PERCENT)).round_dp(2)
}

/// 基数 × 税率% ÷ 100，不做舍入（供凭证类链路按列精度自行收敛）。
pub fn percent_of(base: Decimal, tax_percent: Decimal) -> Decimal {
    base * tax_percent / PERCENT
}

/// 从单据上已记录的税额与对应不含税基反推适用税率（百分数）。
///
/// 入参：`tax_amount` 为该单据已落库的税额、`base` 为同单据对应不含税基。
/// `base <= 0` 时无法反推返回 `None`，调用方必须显式处理（点名报错或按"该基无税"
/// 的算术事实留 0），禁止回落到任何档位字面量。
pub fn derive_tax_percent(tax_amount: Decimal, base: Decimal) -> Option<Decimal> {
    if base <= Decimal::ZERO {
        return None;
    }
    Some(tax_amount / base * PERCENT)
}

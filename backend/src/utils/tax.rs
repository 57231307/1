//! 增值税税额计算内核——税率单位口径的唯一真源
//!
//! 单位口径限定在**价税与进项税域**。该域税率一律以**百分数**表达、传输与存储
//! （13 表示 13%，0 表示免税或无进项税），点名覆盖 purchase_order_item.tax_percent、
//! sales_order_item.tax_percent、purchase_return_item.tax_percent、
//! sales_quotations.tax_rate 与本模块全部入参 tax_percent。
//! 传入小数 0.13 会被按 0.13% 计算，税额量级错 100 倍，因此所有税额与含税价计算
//! 只能经本模块出，禁止各域再写一套乘 0.13 或除 100 的散装公式。
//!
//! 例外是**出口退税域**，该域是**小数**口径且自身自洽，不经过本模块。
//! export_refund_declarations.refund_rate 为 DECIMAL(6,4)，services/export_refund_service.rs
//! 的免抵退计算是出口销售额乘 refund_rate 直乘、不除 100，前端
//! views/tax-rebates/index.vue 的输入上界是 1、展示时按百分数放大。
//! 若据"全仓百分数"把 13 当退税率传进该域，退税额会放大 100 倍，禁止按本模块口径改写它。
//!
//! 缺税率来源的口径是**分层**的，两侧各只有一条合法路径，任何一侧都不得越界。
//! - **人工建单与人工改行**必须提交税率，缺值由调用方点名拒绝。0 是业务声明值，
//!   不能当"没录"的缺省，见 services/po/order_ops/crud.rs 的 calculate_item_amounts。
//! - **自动补货链路**无人工录入环节也无税率来源，由调用方**显式声明无进项税**
//!   （取 NO_INPUT_TAX_PERCENT）落账并留日志，见 services/po/price.rs 两条建单入口。
//! 本模块**不含任何税率档位**。档位值只能由调用方从真实来源取得（人工录入值、
//! 上游单据已存值、部署配置），任何链路都禁止回落默认档位。
//!
//! 入参约定，tax_percent 为百分数口径税率，amount 为不含税金额，
//! 返回值供各单据服务直接写入税额列。

use rust_decimal::Decimal;

/// 百分之一（税率换算因子）。
/// Decimal::from 非 const fn，常量场景用等价的 const 构造 from_parts
/// （与 constants 模块的 environmental_tax 同一口径）。
const PERCENT: Decimal = Decimal::from_parts(100, 0, 0, false, 0);

/// 无进项税税率（0）。
///
/// 语义是"业务上本就无进项税（免税、来料加工、出口零税率、无进项票）"，属模块文档承认的
/// 合法口径值，**不是**税率档位。档位是 13、9、10 这类会凭空放大税额的数值，
/// 禁止用任何档位替代本常量做回落。
/// 调用方只有**自动补货链路**。缺料预警与库存预警两条建单入口无人工录入环节、
/// 也无税率来源，在此**显式声明**无进项税（services/po/price.rs），产出的 Decimal
/// 写入 purchase_order_item 的 tax_percent 列（NOT NULL），并作为税额内核入参算出
/// tax_amount。**人工建单缺 tax_rate 不得使用本常量代填**，由调用方点名拒绝，
/// 否则漏录与免税在进项台账里无法区分。
pub const NO_INPUT_TAX_PERCENT: Decimal = Decimal::from_parts(0, 0, 0, false, 0);

/// 税率可提交的上界，等于落库列域而非业务档位。
///
/// 依据是 purchase_order_item.tax_percent 为 DECIMAL(7,4)，即百分数口径的 0 至
/// 999.9999，超出即列溢出，由 insert 抛数据库错误。
/// 调用方是建单行校验（services/po/order_ops/crud.rs 的 calculate_item_amounts），
/// 用途是在算金额之前把越界入参点名拒绝，让 400 归位到入参非法而不是 500 落库崩溃。
/// 本常量不落库，也不参与税额计算。
pub const TAX_PERCENT_MAX: Decimal = Decimal::from_parts(9_999_999, 0, 0, false, 4);

/// 税额 = 不含税金额 乘 税率% 除 100，按 2 位小数收敛。
pub fn tax_amount(amount: Decimal, tax_percent: Decimal) -> Decimal {
    (amount * tax_percent / PERCENT).round_dp(2)
}

/// 含税单价 = 单价 乘 (1 + 税率% 除 100)，按 2 位小数收敛。
pub fn unit_price_with_tax(unit_price: Decimal, tax_percent: Decimal) -> Decimal {
    (unit_price * (Decimal::ONE + tax_percent / PERCENT)).round_dp(2)
}

/// 基数 乘 税率% 除 100，不做舍入（供凭证类链路按列精度自行收敛）。
pub fn percent_of(base: Decimal, tax_percent: Decimal) -> Decimal {
    base * tax_percent / PERCENT
}

/// 从单据上已记录的税额与对应不含税基反推适用税率（百分数）。
///
/// 入参，tax_amount 为该单据已落库的税额、base 为同单据对应不含税基。
/// base <= 0 时无法反推返回 None，调用方必须显式处理（点名报错或按"该基无税"
/// 的算术事实留 0），禁止回落到任何档位字面量。
pub fn derive_tax_percent(tax_amount: Decimal, base: Decimal) -> Option<Decimal> {
    if base <= Decimal::ZERO {
        return None;
    }
    Some(tax_amount / base * PERCENT)
}

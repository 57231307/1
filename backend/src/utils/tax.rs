//! 增值税税额计算内核——税率单位口径与价税换算方向的唯一真源
//!
//! 单位口径限定在**价税与进项税域**。该域税率一律以**百分数**表达、传输与存储
//! （13 表示 13%，0 表示免税或无进项税），点名覆盖 purchase_order_item.tax_percent、
//! sales_order_item.tax_percent、purchase_return_item.tax_percent、
//! sales_quotations.tax_rate 与本模块全部入参 tax_percent。
//! 传入小数 0.13 会被按 0.13% 计算，税额量级错 100 倍，因此所有税额与含税价计算
//! 只能经本模块出，禁止各域再写一套乘 0.13 或除 100 的散装公式。
//!
//! **换算方向是本模块的第二条硬口径，与单位口径同等强度**。含税金额（含税单价）是
//! 录入事实或上游单据带入值时，价税分离只允许一个方向，即 净额 等于 含税 除以
//! (1 加 税率百分数) 并先收敛到分，税额 等于 含税 减 净额 取差额，
//! 并且恒等式 净额 加 税额 等于 含税 必须逐分成立（唯一入口 split_from_inclusive）。
//! 被禁止的方向，用 净额 乘 税率 造税额、用 净额 乘 (1 加 税率) 造含税、
//! 用 1.13 与 0.13 这类写死的档位代替单据自身税率，
//! 以及在已经拿到含税事实的链路里再乘一次回去推含税。
//! 理由两条，价税分离在财税口径上本来只用除法还原不含税销售额；
//! 净额与税额各自独立舍入会造出 1 分差，票面与台账随之不钩稽。
//! 0 税率与免税走同一条除法（除数为 1，退化成等值），本模块不为 0 开任何例外分支，
//! 因此不存在"免税腿用乘法、含税腿用除法"的双标空间。
//!
//! 唯一的反向入口是 split_from_net，它只服务**净额是事实**的链路，税额按净额正算、
//! 含税取净额加税额，三值钩稽由构造函数保证，调用方不得自己拼加减。
//! 逐条判定为"净额是事实"的真实入口清单（新增正向入口必须进这份清单并被方向锁测复核）
//! - 采购订单行建单与改行，services/po/order_ops/crud.rs 的 calculate_item_amounts，
//!   录入列是净额单价 unit_price，税率是人工提交的进项税率；
//! - 采购收货重算与自动补货建单，services/po/receipt.rs 与 services/po/price.rs，
//!   沿用同一行的净额事实，后者显式声明无进项税；
//! - 销售订单行与销售合同行，services/so/amounts.rs 的 calculate_line_amounts，
//!   销售侧表单只有单价与小计两列，净额是录入事实，含税单价由报价带入另一列；
//! - 发货行销项税额，services/so/delivery_ops/ship.rs，基数是实发数量乘订单净额单价，
//!   出库环节不存在按发货量重新录入的含税事实；
//! - 销售退货行与采购退货行，services/sales_return_service.rs 与
//!   services/purchase_return_service.rs，退货按原单净额单价乘退货数量立账；
//! - 委外非正常损耗的进项税转出，services/outsourcing_ops/receipt.rs，
//!   税率由该单已记进项税额与费基反推，转出额按比例正算，不是价税分离；
//! - 报价推荐价，services/quotation_pricing_service.rs，来源是色号基础价档（净额），
//!   产出的是含税单价**建议值**，不是单据成交后的含税事实。
//!
//! 三值各自独立来源、只需核对不需换算的入口用 assert_three_way_reconcile，
//! 它不产生任何新数值，只判定 净额 加 税额 等于 含税 且 含税不小于净额，
//! 破口即点名拒绝（财务发票票面三值与报价转订单的行级三值走这条）。
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
//! 入参约定，tax_percent 为百分数口径税率，inclusive 为含税金额或含税单价，
//! net 为不含税净额，出参 TaxSplit 供各单据服务分别写入净额列、税额列与含税总额列。

use rust_decimal::Decimal;
use std::fmt;

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

/// 价税三值收敛到分的唯一位数。
///
/// 功能，钉住一切税额与含税换算只做一次舍入并且舍到位。
/// 调用方，本模块全部换算函数与需要核对独立录入三值的调用点。
/// 入参，无。传给谁，随出参一起落各单据的净额列、税额列与含税总额列。
/// 存什么，位数常量 2。存哪里，只作编译期常量，不落库。
///
/// 依据，采购行与报价行的税额列与含税列是 DECIMAL(18,2)，销售订单行经迁移放宽到
/// DECIMAL(18,4)，但录入面与出参一律按分（2 位）表达。换算内部若按 4 位舍，
/// 写进 2 位列时数据库会再舍一次，两次舍入就能造出 1 分差，三值钩稽随之破口，
/// 因此这里坚持 2 位而不是取列 scale 的最大值。
pub const MONEY_SCALE: u32 = 2;

/// 价税三值，含税事实、净额与税额。
///
/// 功能，把一次换算产出的三个数绑成一个不可拆的整体，杜绝调用方各舍各的。
/// 调用方，split_from_inclusive 与 split_from_net 与 assert_three_way_reconcile 的出参。
/// 入参，由上述构造函数填入。传给谁，各单据服务的金额写入腿。
/// 存什么，inclusive 存含税列（total_amount 与 amount_with_tax），net 存净额列
/// （subtotal 与 amount），tax 存税额列（tax_amount）。存哪里，由调用方落对应单据表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxSplit {
    /// 含税金额（含税单价）事实，已收敛到分，原样落含税列
    pub inclusive: Decimal,
    /// 不含税净额，已收敛到分
    pub net: Decimal,
    /// 税额，已收敛到分，恒满足 净额 加 税额 等于 含税
    pub tax: Decimal,
}

/// 价税换算的入参非法形态，一律由调用方点名拒绝并把原文外显，不静默也不兜底。
///
/// 功能，把"缺税率""负税率""越列域""含税小于净额""三值不钩稽"五类非法输入
/// 与"换算本身"分开表达，让调用方能按形态出不同文案。
/// 调用方，本模块换算函数的返回值；各单据服务把它转成 400 或业务拒绝。
/// 入参，各变体带实际提交值。传给谁，错误映射层。存什么与存哪里，不落库。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxDirectionError {
    /// 税率缺键。未录与 0 税率是两回事，0 必须由上游显式提交
    MissingTaxPercent,
    /// 税率为负
    NegativeTaxPercent {
        /// 实际提交值
        tax_percent: Decimal,
    },
    /// 税率越出落库列域
    TaxPercentOverMax {
        /// 实际提交值
        tax_percent: Decimal,
        /// 列域上界
        max: Decimal,
    },
    /// 含税金额为负
    NegativeInclusiveAmount {
        /// 实际提交值
        inclusive: Decimal,
    },
    /// 净额为负
    NegativeNetAmount {
        /// 实际提交值
        net: Decimal,
    },
    /// 含税小于净额，价税三值互相矛盾
    InclusiveBelowNet {
        /// 净额侧
        net: Decimal,
        /// 含税侧
        inclusive: Decimal,
    },
    /// 净额加税额不等于含税
    NotReconciled {
        /// 净额侧
        net: Decimal,
        /// 税额侧
        tax: Decimal,
        /// 含税侧
        inclusive: Decimal,
        /// 差额，已收敛到分
        gap: Decimal,
    },
}

impl fmt::Display for TaxDirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaxDirectionError::MissingTaxPercent => write!(
                f,
                "缺少税率 tax_percent，须按百分数口径提交（13 表示 13%，无税显式提交 0），系统不设默认税率档位"
            ),
            TaxDirectionError::NegativeTaxPercent { tax_percent } => write!(
                f,
                "税率 tax_percent 不得为负，实际 {tax_percent}，须按百分数口径提交（13 表示 13%）"
            ),
            TaxDirectionError::TaxPercentOverMax { tax_percent, max } => write!(
                f,
                "税率 tax_percent 越出落库列域，实际 {tax_percent}，上界 {max}（百分数口径，超出即列溢出）"
            ),
            TaxDirectionError::NegativeInclusiveAmount { inclusive } => write!(
                f,
                "含税金额不得为负，实际 {inclusive}，红冲与退货请按单据的负向数量或单价表达"
            ),
            TaxDirectionError::NegativeNetAmount { net } => {
                write!(f, "不含税净额不得为负，实际 {net}")
            }
            TaxDirectionError::InclusiveBelowNet { net, inclusive } => write!(
                f,
                "含税金额小于不含税净额，价税三值互相矛盾，净额 {net} 含税 {inclusive}，税率非负时含税不可能低于净额"
            ),
            TaxDirectionError::NotReconciled {
                net,
                tax,
                inclusive,
                gap,
            } => write!(
                f,
                "价税三值不钩稽，净额 {net} 加 税额 {tax} 与含税 {inclusive} 差 {gap}，须逐分相等"
            ),
        }
    }
}

impl std::error::Error for TaxDirectionError {}

/// 校验税率取值并给出除法因子 1 加 税率百分数。
///
/// 功能，把"缺键 负数 越列域"三类非法税率在换算之前拦下。
/// 调用方，本模块 split_from_inclusive。入参 tax_percent 是百分数口径税率的可选值。
/// 传给谁，返回值进换算函数作除数。存什么与存哪里，不落库。
fn validated_factor(tax_percent: Option<Decimal>) -> Result<Decimal, TaxDirectionError> {
    let percent = tax_percent.ok_or(TaxDirectionError::MissingTaxPercent)?;
    validate_percent(percent)?;
    Ok(Decimal::ONE + percent / PERCENT)
}

/// 校验税率落在 0 到落库列域上界之间。
///
/// 功能，防止负税率与越界税率进入任何一侧的换算。
/// 调用方，本模块 validated_factor 与 split_from_net。
/// 入参，tax_percent 为百分数口径税率。传给谁，返回给上层换算函数决定是否继续。
/// 存什么与存哪里，不落库。
fn validate_percent(tax_percent: Decimal) -> Result<(), TaxDirectionError> {
    if tax_percent < Decimal::ZERO {
        return Err(TaxDirectionError::NegativeTaxPercent { tax_percent });
    }
    if tax_percent > TAX_PERCENT_MAX {
        return Err(TaxDirectionError::TaxPercentOverMax {
            tax_percent,
            max: TAX_PERCENT_MAX,
        });
    }
    Ok(())
}

/// 含税事实进，净额与税额出，价税分离的唯一合法方向。
///
/// 功能，把含税金额按单据自身税率做除法反算，税额取差额，三值钩稽由构造保证。
/// 调用方，报价单表头合计（services/quotation_ops 的 calc 与 update）与任何
/// "含税是录入事实或上游单据带入值"的写入腿。
/// 入参，inclusive_raw 为含税金额或含税单价（允许带超过 2 位的原始乘积，本函数
/// 先收敛到分），tax_percent 为百分数口径税率的可选项，缺键即点名拒绝。
/// 传给谁，出参 TaxSplit 由调用方分别写入净额列、税额列与含税总额列。
/// 存什么存哪里，本函数不落库，只出三个已收敛到分的定点小数。
///
/// 舍入口径，含税先收敛到分，除法结果收敛到分，税额取两者之差，
/// 两个分位精确值相减天然落在分上，因此全程只做两次同精度收敛、没有第三次舍入。
/// 0 税率走同一条除法（因子为 1，退化成等值且税额为 0），函数内没有税率分支。
pub fn split_from_inclusive(
    inclusive_raw: Decimal,
    tax_percent: Option<Decimal>,
) -> Result<TaxSplit, TaxDirectionError> {
    let factor = validated_factor(tax_percent)?;
    let inclusive = inclusive_raw.round_dp(MONEY_SCALE);
    if inclusive < Decimal::ZERO {
        return Err(TaxDirectionError::NegativeInclusiveAmount { inclusive });
    }
    let net = (inclusive / factor).round_dp(MONEY_SCALE);
    let tax = inclusive - net;
    Ok(TaxSplit {
        inclusive,
        net,
        tax,
    })
}

/// 含税事实反算不含税净额，口径与 split_from_inclusive 完全同源。
///
/// 功能，只需要净额的展示与校验腿用它，避免为拿一个数而拆整个结构体。
/// 调用方，各服务在写入前的净额取值腿与用例断言。
/// 入参，inclusive_raw 为含税金额或含税单价、tax_percent 为百分数口径税率可选项。
/// 传给谁，返回的净额由调用方落净额列或参与比较。存什么存哪里，不落库。
pub fn net_from_inclusive(
    inclusive_raw: Decimal,
    tax_percent: Option<Decimal>,
) -> Result<Decimal, TaxDirectionError> {
    Ok(split_from_inclusive(inclusive_raw, tax_percent)?.net)
}

/// 含税事实反算税额（含税减反算净额取差额），口径与 split_from_inclusive 完全同源。
///
/// 功能，只需要税额的台账腿用它，税额永远是与净额的差额而不是第二次独立舍入。
/// 调用方，各服务在写入前的税额取值腿与用例断言。
/// 入参，inclusive_raw 为含税金额或含税单价、tax_percent 为百分数口径税率可选项。
/// 传给谁，返回的税额由调用方落 tax_amount 列。存什么存哪里，不落库。
pub fn tax_from_inclusive(
    inclusive_raw: Decimal,
    tax_percent: Option<Decimal>,
) -> Result<Decimal, TaxDirectionError> {
    Ok(split_from_inclusive(inclusive_raw, tax_percent)?.tax)
}

/// 净额是事实时的正算入口，税额按净额正算、含税取净额加税额。
///
/// 功能，把"净额 乘 税率 造税额"与"净额 加 税额 造含税"收进唯一一处，
/// 让所有正向入口共享同一次舍入与同一条钩稽，调用方不得自己拼。
/// 调用方，模块文档清单里逐条判为"净额是事实"的真实入口（采购订单行、采购收货重算、
/// 自动补货建单、销售订单行与销售合同行、发货行销项税、销售退货行与采购退货行）。
/// 入参，net_raw 为不含税净额或不含税单价（允许带超过 2 位的原始乘积）、
/// tax_percent 为百分数口径税率可选项，缺键即点名拒绝。
/// 传给谁，出参 TaxSplit 由调用方分别写入净额列、税额列与含税总额列。
/// 存什么存哪里，本函数不落库。
///
/// 禁止把它用在"含税才是事实"的链路：那种链路里本函数造的含税会与录入含税脱钩，
/// 台账上表现为一笔单据两个含税数，任何一侧调整都无人报警。
pub fn split_from_net(
    net_raw: Decimal,
    tax_percent: Option<Decimal>,
) -> Result<TaxSplit, TaxDirectionError> {
    let percent = tax_percent.ok_or(TaxDirectionError::MissingTaxPercent)?;
    validate_percent(percent)?;
    let net = net_raw.round_dp(MONEY_SCALE);
    if net < Decimal::ZERO {
        return Err(TaxDirectionError::NegativeNetAmount { net });
    }
    let tax = (net * percent / PERCENT).round_dp(MONEY_SCALE);
    let inclusive = net + tax;
    Ok(TaxSplit {
        inclusive,
        net,
        tax,
    })
}

/// 三值各自独立来源时的钩稽判定，不产生任何新数值。
///
/// 功能，只核对 净额 加 税额 等于 含税 与 含税不小于净额，破口即点名拒绝，
/// 给"票面三值都由人工或上游单据带入"的链路一条不改取值的守门路径。
/// 调用方，财务发票建单（handlers/finance_invoice_handler.rs 的 create_finance_invoice）
/// 与报价转订单的行级三值（services/quotation_convert_service.rs 的
/// copy_quotation_items_to_order）。
/// 入参，三个原始定点小数，本函数按 MONEY_SCALE 各收敛一次再比较。
/// 传给谁，成功时出参 TaxSplit 供调用方直接落库；失败时由调用方把 Display 原文外显。
/// 存什么存哪里，不落库，落库由调用方按净额列、税额列、含税列分别写入。
///
/// 容差用存储精度（分位精确相等，收敛后差额必须为 0），不用浮点 epsilon。
pub fn assert_three_way_reconcile(
    net_raw: Decimal,
    tax_raw: Decimal,
    inclusive_raw: Decimal,
) -> Result<TaxSplit, TaxDirectionError> {
    let net = net_raw.round_dp(MONEY_SCALE);
    let tax = tax_raw.round_dp(MONEY_SCALE);
    let inclusive = inclusive_raw.round_dp(MONEY_SCALE);
    if inclusive < net {
        return Err(TaxDirectionError::InclusiveBelowNet { net, inclusive });
    }
    let gap = (net + tax - inclusive).round_dp(MONEY_SCALE);
    if gap != Decimal::ZERO {
        return Err(TaxDirectionError::NotReconciled {
            net,
            tax,
            inclusive,
            gap,
        });
    }
    Ok(TaxSplit {
        inclusive,
        net,
        tax,
    })
}

/// 税额 = 不含税金额 乘 税率% 除 100，按 2 位小数收敛。
///
/// 方向口径，这是"净额是事实"的正向腿，只允许模块文档清单里的入口调用；
/// 含税才是事实的链路一律走 split_from_inclusive，禁止用本函数造含税。
/// 调用方与入参沿用原口径，amount 为不含税金额，tax_percent 为百分数口径税率，
/// 返回值供各单据服务写入税额列。需要同时拿到净额与含税时用 split_from_net，
/// 它保证三值钩稽而不是让调用方自己拼加法。
pub fn tax_amount(amount: Decimal, tax_percent: Decimal) -> Decimal {
    (amount * tax_percent / PERCENT).round_dp(2)
}

/// 含税单价 = 单价 乘 (1 + 税率% 除 100)，按 2 位小数收敛。
///
/// 方向口径，全仓唯一一次"由净价档生成含税建议"的正向腿，用途限报价推荐价
/// （services/quotation_pricing_service.rs 的 calculate，来源是色号基础价档，
/// 出参是给人看的建议值）。含税一经成交就是单据事实，此后任何链路不得再用它，
/// 需要反算时用 net_from_inclusive。
pub fn unit_price_with_tax(unit_price: Decimal, tax_percent: Decimal) -> Decimal {
    (unit_price * (Decimal::ONE + tax_percent / PERCENT)).round_dp(2)
}

/// 基数 乘 税率% 除 100，不做舍入（供凭证类链路按列精度自行收敛）。
///
/// 方向口径，正向腿，基数是已核对过的不含税事实（委外进项税转出的费基与比例），
/// 不产出含税金额，因此不触及"禁止乘法造含税"。
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// 夹具金额字面量，解析失败立即点名，不允许静默变成 0
    fn dec(literal: &str) -> Decimal {
        Decimal::from_str(literal)
            .unwrap_or_else(|e| panic!("测试内金额字面量必须可解析，实际 {literal}，错误 {e}"))
    }

    /// 三值逐分钩稽的公共断言，容差取存储精度（分位精确相等），不用浮点 epsilon。
    fn assert_reconciles(
        split: &TaxSplit,
        expected_net: &str,
        expected_tax: &str,
        expected_inclusive: &str,
    ) {
        assert_eq!(
            split.net,
            dec(expected_net),
            "净额必须等于含税除以因子后收敛到分的值，期望 {expected_net}，实际 {}",
            split.net
        );
        assert_eq!(
            split.tax,
            dec(expected_tax),
            "税额必须等于含税减净额的差额，期望 {expected_tax}，实际 {}",
            split.tax
        );
        assert_eq!(
            split.inclusive,
            dec(expected_inclusive),
            "含税必须等于收敛到分后的录入事实，期望 {expected_inclusive}，实际 {}",
            split.inclusive
        );
        assert_eq!(
            split.net + split.tax,
            split.inclusive,
            "三值钩稽破口，净额 {} 加税额 {} 必须逐分等于含税 {}",
            split.net,
            split.tax,
            split.inclusive
        );
        assert_eq!(
            (split.net + split.tax - split.inclusive).round_dp(MONEY_SCALE),
            Decimal::ZERO,
            "钩稽差额必须为 0 分，实际净额 {} 税额 {} 含税 {}",
            split.net,
            split.tax,
            split.inclusive
        );
    }

    // ===== 除法方向（含税是录入事实）=====

    /// 正向，含税 1130 元按 13 个点除法反算得净额 1000 与税额 130，三值逐分钩稽。
    #[test]
    fn split_from_inclusive_divides_the_recorded_gross_amount() {
        let split = split_from_inclusive(dec("1130"), Some(dec("13")))
            .expect("含税 1130 与税率 13 是合法夹具，不该被拒绝");
        assert_reconciles(&split, "1000", "130", "1130");
    }

    /// 正向，含税 1000 元按 13 个点反算，净额落在 884.96，税额取差额 115.04。
    #[test]
    fn split_from_inclusive_keeps_the_cent_exact_when_the_division_repeats() {
        let split = split_from_inclusive(dec("1000"), Some(dec("13"))).expect("合法夹具不该被拒绝");
        assert_reconciles(&split, "884.96", "115.04", "1000");
    }

    /// 正向，9 个点与 13.5 个点两档各算各的，函数内不持有任何档位。
    #[test]
    fn split_from_inclusive_uses_the_submitted_rate_not_any_default_tier() {
        assert_reconciles(
            &split_from_inclusive(dec("1090"), Some(dec("9"))).expect("9 个点夹具应通过"),
            "1000",
            "90",
            "1090",
        );
        assert_reconciles(
            &split_from_inclusive(dec("1000"), Some(dec("13.5"))).expect("13.5 个点夹具应通过"),
            "881.06",
            "118.94",
            "1000",
        );
        assert_reconciles(
            &split_from_inclusive(dec("1234567.89"), Some(dec("13"))).expect("大额夹具应通过"),
            "1092537.96",
            "142029.93",
            "1234567.89",
        );
    }

    /// 税率 0，免税腿走同一条除法并退化成等值，税额为 0，不开任何例外分支。
    #[test]
    fn zero_rate_inclusive_leg_degrades_to_identity() {
        let split =
            split_from_inclusive(dec("1000"), Some(Decimal::ZERO)).expect("0 税率是合法声明值");
        assert_reconciles(&split, "1000", "0", "1000");
    }

    /// 税率 100，因子为 2，含税 600 元劈成净额 300 与税额 300。
    #[test]
    fn hundred_percent_rate_splits_the_gross_amount_in_half() {
        let split = split_from_inclusive(dec("600"), Some(dec("100"))).expect("100 税率是合法夹具");
        assert_reconciles(&split, "300", "300", "600");
    }

    /// 空税率，缺键点名拒绝，绝不退化成 0 税率也绝不取任何档位。
    #[test]
    fn missing_rate_is_named_instead_of_defaulted() {
        assert_eq!(
            split_from_inclusive(dec("1130"), None),
            Err(TaxDirectionError::MissingTaxPercent),
            "含税腿缺税率必须出 MissingTaxPercent，不能按 0 税率算成净额等于含税"
        );
        assert_eq!(
            net_from_inclusive(dec("1130"), None),
            Err(TaxDirectionError::MissingTaxPercent),
            "只取净额的腿同样必须点名拒绝缺税率"
        );
        assert_eq!(
            tax_from_inclusive(dec("1130"), None),
            Err(TaxDirectionError::MissingTaxPercent),
            "只取税额的腿同样必须点名拒绝缺税率"
        );
        assert_eq!(
            split_from_net(dec("1000"), None),
            Err(TaxDirectionError::MissingTaxPercent),
            "正向腿缺税率也必须点名拒绝"
        );
        assert!(
            TaxDirectionError::MissingTaxPercent
                .to_string()
                .contains("tax_percent"),
            "缺税率文案必须带字段名，前端才可能定位到录入项"
        );
    }

    /// 负税率与越列域税率逐条点名，拒绝文案带实际提交值。
    #[test]
    fn illegal_rates_are_rejected_per_form() {
        assert_eq!(
            split_from_inclusive(dec("1130"), Some(dec("-13"))),
            Err(TaxDirectionError::NegativeTaxPercent {
                tax_percent: dec("-13")
            }),
            "负税率必须出 NegativeTaxPercent 并回显实际值"
        );
        assert_eq!(
            split_from_net(dec("1000"), Some(dec("-0.01"))),
            Err(TaxDirectionError::NegativeTaxPercent {
                tax_percent: dec("-0.01")
            }),
            "正向腿的负税率同样拒绝"
        );
        let over = TAX_PERCENT_MAX + dec("0.01");
        assert_eq!(
            split_from_inclusive(dec("1130"), Some(over)),
            Err(TaxDirectionError::TaxPercentOverMax {
                tax_percent: over,
                max: TAX_PERCENT_MAX,
            }),
            "越出落库列域的税率必须在换算之前拒绝"
        );
        assert!(
            split_from_inclusive(dec("1130"), Some(TAX_PERCENT_MAX)).is_ok(),
            "列域上界本身是合法值，拒绝必须只针对越界"
        );
    }

    /// 含税为负点名拒绝，红冲不由本模块用负金额表达。
    #[test]
    fn negative_inclusive_amount_is_rejected() {
        assert_eq!(
            split_from_inclusive(dec("-1130"), Some(dec("13"))),
            Err(TaxDirectionError::NegativeInclusiveAmount {
                inclusive: dec("-1130")
            }),
            "含税为负必须出 NegativeInclusiveAmount"
        );
        assert_eq!(
            split_from_net(dec("-1"), Some(dec("13"))),
            Err(TaxDirectionError::NegativeNetAmount { net: dec("-1") }),
            "净额为负必须出 NegativeNetAmount"
        );
    }

    /// 只舍一次，含税原始值先收敛到分再做除法，原始尾差不参与第二次舍入。
    #[test]
    fn rounding_happens_once_at_money_scale() {
        let split = split_from_inclusive(dec("1130.004"), Some(dec("13"))).expect("尾差夹具应通过");
        assert_reconciles(&split, "1000", "130", "1130");
        let upper = split_from_inclusive(dec("1129.996"), Some(dec("13"))).expect("尾差夹具应通过");
        assert_reconciles(&upper, "1000", "130", "1130");
        assert_eq!(
            MONEY_SCALE, 2,
            "存储口径位数必须是 2 分，改动位数要同步全部单据列"
        );
    }

    /// 税额取差额而不是第二次独立舍入，含税 0.13 元这条上独立舍入会把钩稽差造出来。
    #[test]
    fn tax_is_the_difference_not_an_independent_rounding() {
        let split = split_from_inclusive(dec("0.13"), Some(dec("13"))).expect("分位夹具应通过");
        assert_reconciles(&split, "0.12", "0.01", "0.13");
        // 独立舍入的税额是 0.02，净额加它会得 0.14，比含税多 1 分，票面就钩稽不上
        assert_eq!(
            split.tax,
            Decimal::ZERO + dec("0.01"),
            "税额必须是含税减净额的差额，不得取净额乘税率的独立舍入值"
        );
    }

    /// 一批真实量级下净额永不超过含税，税额非负，三值逐分相等。
    #[test]
    fn inclusive_split_never_yields_net_above_gross() {
        let rates = ["0", "1", "3", "6", "9", "13", "13.56", "100", "999.9999"];
        let gross_amounts = [
            "0.01",
            "0.04",
            "1",
            "99.99",
            "1000",
            "250000.55",
            "8888888.88",
        ];
        for rate in rates {
            for gross in gross_amounts {
                let split = split_from_inclusive(dec(gross), Some(dec(rate)))
                    .unwrap_or_else(|e| panic!("合法夹具 {gross} 与税率 {rate} 被拒绝，错误 {e}"));
                assert!(
                    split.net <= split.inclusive,
                    "税率非负时净额不得超过含税，实际税率 {rate} 含税 {gross} 净额 {}",
                    split.net
                );
                assert!(
                    split.tax >= Decimal::ZERO,
                    "税额不得为负，实际税率 {rate} 含税 {gross} 税额 {}",
                    split.tax
                );
                assert_eq!(
                    split.net + split.tax,
                    split.inclusive,
                    "三值钩稽破口，税率 {rate} 含税 {gross}"
                );
            }
        }
    }

    // ===== 正向（净额是事实）=====

    /// 正向，净额 1000 元按 13 个点得税额 130 与含税 1130，钩稽由构造保证。
    #[test]
    fn split_from_net_builds_the_three_values_for_net_fact_entries() {
        let split = split_from_net(dec("1000"), Some(dec("13"))).expect("净额是事实的正向夹具");
        assert_reconciles(&split, "1000", "130", "1130");
    }

    /// 正向，净额 884.96 元按 13 个点得税额 115.04 与含税 1000，与除法腿互为逆运算。
    #[test]
    fn split_from_net_is_the_inverse_of_the_division_leg() {
        let forward = split_from_net(dec("884.96"), Some(dec("13"))).expect("正向夹具应通过");
        assert_reconciles(&forward, "884.96", "115.04", "1000");
        let back =
            split_from_inclusive(forward.inclusive, Some(dec("13"))).expect("逆向夹具应通过");
        assert_eq!(
            back.net, forward.net,
            "净额事实正算出的含税再除法反算必须回到同一净额，实际 {} 与 {}",
            back.net, forward.net
        );
    }

    /// 正向，净额与含税单价两档在 0 税率与 100 税率下的取值。
    #[test]
    fn split_from_net_handles_zero_and_hundred_percent_rates() {
        assert_reconciles(
            &split_from_net(dec("1000"), Some(Decimal::ZERO)).expect("0 税率是合法声明值"),
            "1000",
            "0",
            "1000",
        );
        assert_reconciles(
            &split_from_net(dec("300"), Some(dec("100"))).expect("100 税率是合法夹具"),
            "300",
            "300",
            "600",
        );
    }

    // ===== 三值钩稽判定（不产生新数值）=====

    /// 票面三值逐分相等时原样放行，不改任何取值。
    #[test]
    fn reconcile_accepts_exact_ticket_three_values_without_changing_them() {
        let split = assert_three_way_reconcile(dec("1000.00"), dec("130.00"), dec("1130.00"))
            .expect("逐分钩稽的票面三值必须放行");
        assert_reconciles(&split, "1000", "130", "1130");
    }

    /// 含税小于净额，价税三值互相矛盾，点名拒绝并回显两侧。
    #[test]
    fn inclusive_below_net_is_rejected() {
        assert_eq!(
            assert_three_way_reconcile(dec("1000"), dec("130"), dec("900")),
            Err(TaxDirectionError::InclusiveBelowNet {
                net: dec("1000"),
                inclusive: dec("900"),
            }),
            "含税低于净额必须出 InclusiveBelowNet，不得静默把净额当含税"
        );
        assert_eq!(
            assert_three_way_reconcile(dec("1000"), dec("0"), dec("999.99")),
            Err(TaxDirectionError::InclusiveBelowNet {
                net: dec("1000"),
                inclusive: dec("999.99"),
            }),
            "含税比净额小 1 分同样算矛盾，容差不得放宽到分以下"
        );
    }

    /// 净额加税额不等于含税，1 分差也点名拒绝，拒绝文案带差额。
    #[test]
    fn three_way_gap_is_rejected_with_the_exact_gap() {
        let err = assert_three_way_reconcile(dec("1000"), dec("130"), dec("1131"))
            .expect_err("差 1 分的票面三值必须被拒绝");
        assert_eq!(
            err,
            TaxDirectionError::NotReconciled {
                net: dec("1000"),
                tax: dec("130"),
                inclusive: dec("1131"),
                gap: dec("-1"),
            },
            "钩稽破口必须出 NotReconciled 并把三值与差额一并回显"
        );
        assert_eq!(
            assert_three_way_reconcile(dec("1000"), dec("130.01"), dec("1130")),
            Err(TaxDirectionError::NotReconciled {
                net: dec("1000"),
                tax: dec("130.01"),
                inclusive: dec("1130"),
                gap: dec("0.01"),
            }),
            "税额多 1 分是同一形态，必须同判"
        );
        let text = err.to_string();
        assert!(
            text.contains("1131") && text.contains("1000"),
            "拒绝文案必须带含税与净额两个实际值，实际文案 {text}"
        );
    }

    /// 三值判定也守税率之外的位数口径，超过分的原始尾差先收敛再比。
    #[test]
    fn reconcile_compares_after_converging_to_money_scale() {
        let split = assert_three_way_reconcile(dec("1000.004"), dec("130.003"), dec("1130.002"))
            .expect("收敛到分后逐分相等的三值必须放行");
        assert_reconciles(&split, "1000", "130", "1130");
    }

    // ===== 既有正向函数的回归锁 =====

    /// 正向税额函数与 split_from_net 的税额必须同值，防止两条正向腿再度分裂。
    #[test]
    fn forward_tax_amount_matches_the_kernel_split() {
        for net in ["1000", "884.96", "25.5", "0.01", "1234567.89"] {
            for rate in ["0", "6", "9", "13"] {
                let via_helper = tax_amount(dec(net), dec(rate));
                let via_split = split_from_net(dec(net), Some(dec(rate)))
                    .unwrap_or_else(|e| panic!("夹具 {net} 与 {rate} 必须合法，错误 {e}"))
                    .tax;
                assert_eq!(
                    via_helper, via_split,
                    "税额两条正向算法必须逐分一致，净额 {net} 税率 {rate}，实际 {via_helper} 与 {via_split}"
                );
            }
        }
    }

    /// 含税单价建议腿与 split_from_net 的含税除数量口径同向，档位只从入参取。
    #[test]
    fn unit_price_with_tax_uses_only_the_submitted_rate() {
        assert_eq!(
            unit_price_with_tax(dec("100"), dec("13")),
            dec("113"),
            "100 元净价按 13 个点得含税单价 113"
        );
        assert_eq!(
            unit_price_with_tax(dec("100"), Decimal::ZERO),
            dec("100"),
            "0 税率下含税单价等于净价，函数内不得有任何档位"
        );
        assert_eq!(
            unit_price_with_tax(dec("7.777"), dec("13")),
            dec("8.79"),
            "含税单价必须收敛到分，实际 7.777 乘 1.13 等于 8.78801，落分是 8.79"
        );
    }

    /// 不做舍入的比例腿保留原始精度，由调用方按列精度自行收敛。
    #[test]
    fn percent_of_does_not_round() {
        assert_eq!(
            percent_of(dec("1000"), dec("13")),
            dec("130"),
            "1000 的 13 个点是 130"
        );
        assert_ne!(
            percent_of(dec("0.1"), dec("13")),
            Decimal::ZERO,
            "不做舍入的比例腿不得把 0.013 洗成 0"
        );
    }

    /// 税率反推，基为 0 或负时返回 None，调用方必须显式处理而不是落档位。
    #[test]
    fn derive_tax_percent_returns_none_for_a_non_positive_base() {
        assert_eq!(
            derive_tax_percent(dec("130"), dec("1000")),
            Some(dec("13")),
            "税额 130 与费基 1000 反推得 13 个点"
        );
        assert_eq!(
            derive_tax_percent(dec("130"), Decimal::ZERO),
            None,
            "费基为 0 无法反推，必须返回 None 而不是 0 或任何档位"
        );
        assert_eq!(
            derive_tax_percent(dec("130"), dec("-1000")),
            None,
            "费基为负同样无法反推"
        );
        assert_eq!(
            derive_tax_percent(Decimal::ZERO, dec("1000")),
            Some(Decimal::ZERO),
            "税额为 0 且费基为正时反推得 0，属免税或无票的算术事实"
        );
    }
}

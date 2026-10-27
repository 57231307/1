//! 增值税价税分离内核——税率单位口径与计算方向的唯一真源
//!
//! 单位口径限定在**价税与进项税域**。该域税率一律以**百分数**表达、传输与存储
//! （13 表示 13%，0 表示免税或无进项税），点名覆盖 purchase_order_item.tax_percent、
//! sales_order_item.tax_percent、purchase_return_item.tax_percent、
//! sales_quotations.tax_rate 与本模块全部入参 tax_percent。
//! 传入小数 0.13 会被按 0.13% 计算，税额量级错 100 倍，因此所有税额与含税价计算
//! 只能经本模块出，禁止各域再写一套乘 0.13 或除 100 的散装公式。
//!
//! **计算方向是本模块的第二条硬口径，与单位口径同等强度**：含税金额（或含税单价）
//! 是录入事实与上游单据带入值，价税分离只允许一个方向，即
//! `净额 = 含税 ÷ (1 + 税率% ÷ 100)` 先四舍五入到分，`税额 = 含税 − 净额` 取差额，
//! 并且恒等式 `净额 + 税额 == 含税` 必须逐分成立。
//! 被禁止的方向：`净额 × 税率` 造税额、`× (1 + 税率)` 造含税、`× 1.13` 这类硬编码档位。
//! 理由有两条：财税口径上价税分离本身只用除法还原不含税销售额
//! （《增值税暂行条例实施细则》：销售额 = 含税销售额 ÷ (1 + 税率)）；
//! 两处各自独立舍入会产生 1 分差，票面与台账不钩稽。
//! 0 税率与免税走同一函数（除法退化为等值），本模块不为 0 开任何例外分支，
//! 因此也不存在"免税腿用乘法、含税腿用除法"的双标空间。
//!
//! 例外是**出口退税域**，该域是**小数**口径且自身自洽，不经过本模块，也不套本模块的
//! 方向口径（该域是退税率直乘，不是价税分离）。
//! export_refund_declarations.refund_rate 为 DECIMAL(6,4)，services/export_refund_service.rs
//! 的免抵退计算是出口销售额乘 refund_rate 直乘、不除 100，前端
//! views/tax-rebates/index.vue 的输入上界是 1、展示时按百分数放大。
//! 若据"全仓百分数"把 13 当退税率传进该域，退税额会放大 100 倍，禁止按本模块口径改写它。
//!
//! 另有一类**非增值税**的从量计征也不在本模块口径内：environmental_tax_service.rs 的
//! 环境保护税是「污染当量数 × 适用税额（元/当量）」，没有价税分离可言，
//! 不得按本模块改写，也不得据本模块的静态判定器判它违规（判定器按文件白名单放行该文件）。
//!
//! 缺税率来源的口径是**分层**的，两侧各只有一条合法路径，任何一侧都不得越界。
//! - **人工建单与人工改行**必须提交税率与含税单价，缺值由调用方点名拒绝。0 是业务声明值，
//!   不能当"没录"的缺省，见 services/po/order_ops/crud.rs 的 calculate_item_amounts。
//! - **自动补货链路**无人工录入环节也无税率来源，由调用方**显式声明无进项税**
//!   （取 NO_INPUT_TAX_PERCENT）落账并留日志，见 services/po/price.rs 两条建单入口。
//!   该链路录的是不含税采购成本价，且在 0 税率下含税与净额等值，故把成本价直接作为
//!   含税事实进本模块，除法退化为等值、税额为 0，不引入任何档位。
//! 本模块**不含任何税率档位**。档位值只能由调用方从真实来源取得（人工录入值、
//! 上游单据已存值、部署配置），任何链路都禁止回落默认档位。
//!
//! 入参约定，tax_percent 为百分数口径税率，inclusive 为含税金额或含税单价，
//! 出参 [`TaxSplit`] 供各单据服务直接写入净额列、税额列与含税总额列。

use rust_decimal::Decimal;

use crate::utils::error::AppError;

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
/// 写入 purchase_order_item 的 tax_percent 列（NOT NULL），并作为价税分离内核入参
/// 算出税额（0）与净额（等于成本价）。**人工建单缺 tax_rate 不得使用本常量代填**，
/// 由调用方点名拒绝，否则漏录与免税在进项台账里无法区分。
pub const NO_INPUT_TAX_PERCENT: Decimal = Decimal::from_parts(0, 0, 0, false, 0);

/// 税率可提交的上界，等于落库列域而非业务档位。
///
/// 依据是 purchase_order_item.tax_percent 为 DECIMAL(7,4)，即百分数口径的 0 至
/// 999.9999，超出即列溢出，由 insert 抛数据库错误。
/// 调用方是建单行校验（services/po/order_ops/crud.rs 的 calculate_item_amounts），
/// 用途是在算金额之前把越界入参点名拒绝，让 400 归位到入参非法而不是 500 落库崩溃。
/// 本常量不落库，也不参与税额计算。
pub const TAX_PERCENT_MAX: Decimal = Decimal::from_parts(9_999_999, 0, 0, false, 4);

/// 单据行或单据头的**价税三值**：含税事实、反算净额、差额税额。
///
/// 三者由 [`split_from_inclusive`] 一次性产出，构造即满足 `net + tax == inclusive`，
/// 调用方把三个字段分别写入净额列、税额列与含税总额列，不存在第二次舍入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxSplit {
    /// 含税金额（或含税单价）：录入事实或上游单据带入值，已收敛到分，原样落含税列
    pub inclusive: Decimal,
    /// 不含税净额：`inclusive ÷ (1 + tax_percent ÷ 100)`，四舍五入到分
    pub net: Decimal,
    /// 税额：`inclusive − net` 取差额，恒等式由构造保证
    pub tax: Decimal,
}

/// 含税金额（或含税单价）反算不含税净额，四舍五入到分。
///
/// 入参，inclusive_raw 是含税事实（允许带超过 2 位的原始乘积，本函数先收敛到分）、
/// tax_percent 是百分数口径税率。出参是净额，2 位小数。
/// 0 税率与免税走同一条除法（除数为 1，退化为等值），函数内没有税率分支。
/// 调用方是本模块 [`split_from_inclusive`] 与只需要净额的展示/校验腿
/// （前端同源算法见 frontend/src/utils/money.ts 的 netFromInclusive）。
pub fn net_from_inclusive(inclusive_raw: Decimal, tax_percent: Decimal) -> Decimal {
    (inclusive_raw.round_dp(2) / (Decimal::ONE + tax_percent / PERCENT)).round_dp(2)
}

/// 价税分离唯一合法方向：含税事实进，净额与税额出，恒等式由构造保证。
///
/// 入参，inclusive_raw 为含税金额（或含税单价）事实、tax_percent 为百分数口径税率。
/// 出参 [`TaxSplit`]，inclusive 是收敛到分后的含税事实、net 是除法反算的净额、
/// tax 是 `含税 − 净额` 的差额（两个 2 位小数之差天然 2 位，不做第二次舍入）。
/// 调用方是全部价税写入腿：采购订单行与收货重算（services/po 的 crud 与 receipt 与 price）、
/// 销售订单行（services/so/order_crud.rs）、发货行税额（services/so/delivery_ops/ship.rs）、
/// 采购退货行与销售退货行（services/purchase_return_service.rs 与 services/sales_return_service.rs）、
/// 报价行与报价头（services/quotation_ops 的 calc 与 crud 与 update、services/quotation_pricing_service.rs）。
/// 产出的三值分别写入各单据的净额列（subtotal/amount）、税额列（tax_amount）
/// 与含税总额列（total_amount/amount_with_tax），由调用方落库。
pub fn split_from_inclusive(inclusive_raw: Decimal, tax_percent: Decimal) -> TaxSplit {
    let inclusive = inclusive_raw.round_dp(2);
    let net = net_from_inclusive(inclusive, tax_percent);
    // MUTATION_ROUND_B 变异自证临时替换 撤销即回差额取法
    let tax = (net * tax_percent / PERCENT).round_dp(2);
    TaxSplit {
        inclusive,
        net,
        tax,
    }
}

/// 由含税单价与数量得到含税金额（含税事实的聚合腿，不是税额算式）。
///
/// 入参，unit_price_with_tax 是人工录入或上游带入的含税单价、quantity 是数量。
/// 出参是收敛到分的含税金额，传给 [`split_from_inclusive`] 做价税分离。
/// 本函数只含「单价 × 数量」这一种乘法，税率不参与，因此不构成造含税；
/// 任何调用方都不得在这里再乘 (1 + 税率)。
pub fn inclusive_amount(unit_price_with_tax: Decimal, quantity: Decimal) -> Decimal {
    (unit_price_with_tax * quantity).round_dp(2)
}

/// 单据行的价税五值（行级单源，见 [`line_split`]）。
#[derive(Debug, Clone, Copy)]
pub struct LineSplit {
    /// 不含税单价（由含税单价反算，落各行表的 unit_price 列）
    pub unit_price: Decimal,
    /// 未折扣净额（落 subtotal/amount 列）
    pub list_net: Decimal,
    /// 折后净额（与 tax 相加等于 deal_inclusive）
    pub deal_net: Decimal,
    /// 成交含税额（标价含税额扣净额折扣前的含税事实，落 total_amount/amount_with_tax 列）
    pub deal_inclusive: Decimal,
    /// 净额口径折扣额（由折扣前后两次反算取差，落 discount_amount 列）
    pub discount_net: Decimal,
    /// 税额（成交含税 − 折后净额，落 tax_amount 列）
    pub tax: Decimal,
}

/// 行级价税分离单源：含税单价 + 数量 + 折扣百分数 + 税率百分数 → 落库各列的五个值。
///
/// 入参，unit_price_with_tax 是录入的含税单价事实、quantity 是数量、
/// discount_percent 是行折扣百分数（可空 = 不打折）、tax_percent 是百分数口径税率。
/// 出参 [`LineSplit`]，其中折扣一律取**净额口径的差额**（折前净额 − 折后净额），
/// 这样各行表的 `subtotal − discount_amount + tax_amount == total_amount` 逐分成立，
/// 收货、应付、开票这些下游只读这三列的链路拿到的永远是钩稽过的数。
/// 传给谁：services/po/order_ops/crud.rs（采购订单行）、services/so/order_crud.rs（销售订单行）、
/// services/purchase_return_service.rs 与 services/sales_return_service.rs（两条退货行）。
/// 为什么必须在内核里：这四条腿历史上各写一套 `× 税率` 的正向公式，同一张单在
/// 建单、改单、退货三处算出三个税额；收敛成一枚函数后，方向口径只有一处可改。
pub fn line_split(
    unit_price_with_tax: Decimal,
    quantity: Decimal,
    discount_percent: Option<Decimal>,
    tax_percent: Decimal,
) -> LineSplit {
    let list_inclusive = inclusive_amount(unit_price_with_tax, quantity);
    let inclusive_discount = match discount_percent {
        Some(percent) => (list_inclusive * percent / PERCENT).round_dp(2),
        None => Decimal::ZERO,
    };
    let deal_inclusive = (list_inclusive - inclusive_discount).round_dp(2);
    let list_net = net_from_inclusive(list_inclusive, tax_percent);
    let deal_net = net_from_inclusive(deal_inclusive, tax_percent);
    LineSplit {
        unit_price: net_from_inclusive(unit_price_with_tax, tax_percent),
        list_net,
        deal_net,
        deal_inclusive,
        discount_net: (list_net - deal_net).round_dp(2),
        tax: (deal_inclusive - deal_net).round_dp(2),
    }
}

/// 取本行的含税单价事实：调用方提交了就采信提交值，没提交时只有 0 税率可以按算术事实
/// 把不含税单价原样当含税额（除法退化为等值，不是造数）。
///
/// 入参，submitted_inclusive 是请求里的含税单价（可缺）、fallback_net 是请求里的不含税单价、
/// tax_percent 是本行生效税率、field_label 是点名用的字段名（出参文案回显给用户怎么填）。
/// 出参是含税单价；税率非 0 而缺含税单价时返回 Err（business 语义，文案点名须按含税单价录入），
/// 系统绝不返回 `不含税单价 × (1 + 税率)` 造出来的价格。
/// 传给谁：services/po/order_ops/crud.rs 与 services/so/order_crud.rs 与两条退货服务的行金额入口。
pub fn require_inclusive_unit_price(
    submitted_inclusive: Option<Decimal>,
    fallback_net: Decimal,
    tax_percent: Decimal,
    field_label: &str,
) -> Result<Decimal, AppError> {
    match submitted_inclusive {
        Some(price) => Ok(price),
        None if tax_percent == Decimal::ZERO => Ok(fallback_net),
        None => Err(AppError::business_displayable(format!(
            "税率 {tax_percent}% 非 0 时缺少含税单价 {field_label}，须按含税单价录入后重试（系统不按税率替你算含税价）",
        ))),
    }
}

/// 编辑既有行但调用方没重新提交含税单价时，从已落库列**还原**含税单价（除法还原，不是造数）。
///
/// 入参，stored_total_inclusive 是该行已落库的成交含税额（total_amount/amount_with_tax 列）、
/// stored_quantity 是该行数量、stored_discount_amount 是该行已落库的净额口径折扣、
/// doc_label 是单据名（进文案让用户知道是哪张单）。
/// 出参是还原出的含税单价（按单价列精度 6 位收敛）；两种情况返回 Err 并要求重新提交：
/// - 该行有折扣：折扣列存的是净额口径，标价含税额还原不出来，硬算等于用折后价当标价再折一次；
/// - 该行数量为 0：除不动。
/// 传给谁：services/po/receipt.rs 的 update_order_item 与两条退货服务的改明细腿。
/// 为什么在本模块：这三条腿都只能靠「已落库含税额 ÷ 数量」还原，写在不同文件里会各舍各的。
pub fn recover_inclusive_unit_price(
    stored_total_inclusive: Decimal,
    stored_quantity: Decimal,
    stored_discount_amount: Decimal,
    doc_label: &str,
) -> Result<Decimal, AppError> {
    if stored_discount_amount != Decimal::ZERO {
        return Err(AppError::business_displayable(format!(
            "{doc_label}本行有折扣，已落库列还原不出标价含税单价，编辑本行须同时提交含税单价 unit_price_with_tax（系统不按税率倒造含税价）",
        )));
    }
    if stored_quantity <= Decimal::ZERO {
        return Err(AppError::business_displayable(format!(
            "{doc_label}本行数量为 0，无法由已落库含税额还原含税单价，请提交含税单价 unit_price_with_tax 后重试",
        )));
    }
    Ok((stored_total_inclusive / stored_quantity).round_dp(6))
}

/// 单价钩稽门：提交的不含税单价必须与内核反算值分位一致，出参是权威的反算单价。
///
/// 入参，submitted_net 是调用方提交的不含税单价（样布/赠品行允许显式 0）、
/// inclusive_unit_price 是 [`require_inclusive_unit_price`] 定妥的含税单价事实、
/// tax_percent 是本行税率、doc_label 是单据名（进文案供用户定位是哪张单）。
/// 出参是反算得到的不含税单价（调用方落 unit_price 列）；不符返回 Err，
/// 文案点名两个值与差值，防止调用方带着旧的乘法公式把两套口径塞进同一行。
/// 传给谁：与 [`require_inclusive_unit_price`] 同一批行金额入口。
pub fn ensure_unit_price_reconciles(
    submitted_net: Decimal,
    inclusive_unit_price: Decimal,
    tax_percent: Decimal,
    doc_label: &str,
) -> Result<Decimal, AppError> {
    let derived = net_from_inclusive(inclusive_unit_price, tax_percent);
    if unit_price_matches(submitted_net, derived) {
        return Ok(derived);
    }
    Err(AppError::business_displayable(format!(
        "{doc_label}价税不钩稽：提交不含税单价 {submitted_net}，按含税单价 {inclusive_unit_price} 与税率 {tax_percent}% 反算应为 {derived}，请只改含税单价（不含税单价由系统反算）",
    )))
}

/// 钩稽校验：`净额 + 税额` 必须逐分等于 `含税`。
///
/// 入参，net 是不含税净额、tax 是税额、inclusive 是含税金额，三值来自单据已落库列
/// 或调用方带入值（不是本模块刚算出的三值）。出参，等值即 Ok，不等返回
/// AppError::business（文案点名三值与实际差额，供人工核票）。
/// 调用方是「金额三值由外部提交进来、系统不重算」的写入腿，点名
/// services/ap_invoice_ops/crud.rs 与 services/finance_invoice_service.rs 的发票录入，
/// 以及各单据金额重算后的自检。用途是把 1 分差在入库之前拦成业务错误，
/// 而不是等票面或台账对不上才发现。
pub fn ensure_reconciled(net: Decimal, tax: Decimal, inclusive: Decimal) -> Result<(), AppError> {
    let sum = net + tax;
    if sum == inclusive {
        return Ok(());
    }
    Err(AppError::business(format!(
        "价税不钩稽：不含税 {net} 加税额 {tax} 等于 {sum}，与含税额 {inclusive} 相差 {}",
        inclusive - sum
    )))
}

/// 单价钩稽容差（0.01 元，一分）。
///
/// 用途：人工录入腿同时提交不含税单价与含税单价时，判定两者是否由同一个含税事实
/// 反算而来（两端各自 round_dp(2) 的次序差异最多一分）。
/// 调用方是 services/po/order_ops/crud.rs 的 calculate_item_amounts。
/// 这不是税率档位，也不参与任何金额生成，只当校验标尺。
pub const UNIT_PRICE_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 2);

/// 判定提交的净额单价与内核反算值是否分位一致（差在一分以内）。
///
/// 入参，submitted 是调用方提交的单价、derived 是 [`net_from_inclusive`] 的出参。
/// 出参 true 表示两端同源，false 表示调用方在按别的公式造数（例如乘了 1 + 税率），
/// 调用方必须点名拒绝，不得采信提交值。
pub fn unit_price_matches(submitted: Decimal, derived: Decimal) -> bool {
    (submitted - derived).abs() <= UNIT_PRICE_TOLERANCE
}

/// 从单据上已记录的税额与对应不含税基**观察**适用税率（百分数）。
///
/// 入参，tax_amount 为该单据已落库的税额、base 为同单据对应不含税基。
/// base <= 0 时无法观察返回 None，调用方必须显式处理（点名报错或按"该基无税"
/// 的算术事实留 0），禁止回落到任何档位字面量。
/// 本函数是**读侧观察**（由已存在的税额与基推出档位，用于核对与提示），
/// 不是税额算式：它的出参不参与生成任何待落库的税额或含税额，因此不违反
/// 「税额只能由含税反算差额得到」这条方向口径。
/// 调用方是委外加工进项税转出的核对腿（services/outsourcing_ops/receipt.rs）。
pub fn derive_tax_percent(tax_amount: Decimal, base: Decimal) -> Option<Decimal> {
    if base <= Decimal::ZERO {
        return None;
    }
    Some(tax_amount / base * PERCENT)
}

/// 把「按占比分摊已记税额」的算术收敛成一枚单源，供进项税转出这类**分摊**腿使用。
///
/// 入参，recorded_tax 是单据上已落库的税额事实、share_numerator 是本次分摊的分子基数、
/// share_denominator 是分母基数（同源口径的总基数）。出参是分摊后的税额，收敛到分。
/// 分母 <= 0 返回 None，由调用方点名处理，不返回 0 冒充算得。
/// 传给谁：调用方写入 outsourcing_voucher.tax_transfer_amount 列（损耗凭证的进项税转出额）。
/// 为什么需要它：转出额必须是**已记进项税额的一部分**，不得由费基乘税率重新造一枚税额，
/// 否则同一笔进项会因分摊口径不同而算出两个数，且转出额可能超过原票税额。
/// 本函数只含「已记税额 × 分子 ÷ 分母」，税率完全不进算式。
pub fn prorate_tax(
    recorded_tax: Decimal,
    share_numerator: Decimal,
    share_denominator: Decimal,
) -> Option<Decimal> {
    if share_denominator <= Decimal::ZERO {
        return None;
    }
    Some((recorded_tax * share_numerator / share_denominator).round_dp(2))
}

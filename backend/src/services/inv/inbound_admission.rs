//! 入库准入判定（全仓唯一实现）
//!
//! 调用方是采购入库建单与追加明细（`handlers::purchase_receipt_handler` 的 `create_receipt`
//! 与 `create_receipt_item`），以及库存直建入库两条端点（`handlers::inventory_stock_handler::create_stock`
//! 与 `handlers::inventory_stock_handler_fabric::create_stock_fabric`）。
//!
//! 存在理由是等级、数量、双计量公斤数、来源单号这四类判据属于纺织成品布入库的受理侧硬门槛，
//! 两条入库链路必须同口径，因此判定只写在本文件一次，调用方负责把行号等上下文拼进文案。
//! 色号、缸号、批次三维追溯判定不在本文件，唯一实现是同目录 [`fabric_class`]。本文件与它互补，
//! 各自只判自己那一族，不交叉复述规则。
//!
//! 取值来源是等级词表 [`inventory_stock_grade::ALL`]（`models::status::purchase_inventory`），
//! 公斤数换算走 `utils::dual_unit_converter::DualUnitConverter` 单入口，本文件不写第二个换算式。

use rust_decimal::Decimal;

use crate::models::status::purchase_inventory::inventory_stock_grade;
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;

/// 验布定级结论必填且必须落在库存等级词表内，返回 trim 后的等级值。
///
/// 入参 `raw` 是明细提交的等级原文，`None`、缺键与纯空白都按未采集处理；`field` 是文案里
/// 点名的字段说法，采购入库说"验布定级结论"，库存直建说"等级"，由调用方按各自界面口径给。
/// 传给谁走 `AppError::validation_displayable`（400，VALIDATION_ERROR 族，可外显），由调用方
/// 拼上行号后原样透传，不得改写成内部错误。
/// 存什么存哪里是 `purchase_receipt_item.grade` 与 `inventory_stocks.grade` 的写入值，
/// 两列都只允许 [`inventory_stock_grade::ALL`] 的三个值。
///
/// 不接受缺省的理由是未定级的成品布按最高等级入账会同时抬高售价与可用量口径，属把采集缺口
/// 伪装成质检结论。词表外的值（例如验布域的 A/B/C）落进库存等级列会让该行对所有按等级筛选
/// 与降级链路永久不可见。
pub fn require_stock_grade(raw: Option<&str>, field: &str) -> Result<String, AppError> {
    let value = raw.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return Err(AppError::validation_displayable(format!(
            "{field}未采集，成品布入库必须登记验布定级结论（允许值 {}）",
            inventory_stock_grade::ALL.join(",")
        )));
    }
    if !inventory_stock_grade::ALL.contains(&value) {
        return Err(AppError::validation_displayable(format!(
            "{field} \"{value}\" 不在库存等级取值域内（允许值 {}）",
            inventory_stock_grade::ALL.join(",")
        )));
    }
    Ok(value.to_string())
}

/// 入库数量必须严格大于零，入参 `quantity` 是主计量米数，`label` 是界面字段说法（如"入库米数"）。
///
/// 传给谁是由调用方拼行号后作为 `AppError::validation_displayable` 出参，在写库之前拒绝。
/// 存什么存哪里是校验通过的值写 `inventory_stocks.quantity_meters` 与流水表
/// `inventory_transactions.quantity_meters`。
///
/// 下限取严格大于零的理由是零与负数入账都不表示任何实物，还会把台账与流水同时污染成假增减，
/// 负数入库等价于凭空出库并绕过所有出库门控。
pub fn require_positive_quantity(quantity: Decimal, label: &str) -> Result<(), AppError> {
    if quantity <= Decimal::ZERO {
        return Err(AppError::validation_displayable(format!(
            "{label}必须大于 0，实收为 0 或负数不构成入库，请核实后重填"
        )));
    }
    Ok(())
}

/// 可选物理参数（克重与幅宽）一旦提交就必须为正，未提交则原样放行。
///
/// 入参 `value` 是提交的可选值，`label` 是界面字段说法（如"克重"）。
/// 传给谁是调用方透传，拒绝发生在写库之前。
/// 存什么存哪里是通过的值写 `inventory_stocks.gram_weight` 与 `inventory_stocks.width`，
/// 以及 `purchase_receipt_item` 同名列。这两列同时是公斤数换算的入参，非正值会让换算必然失败。
pub fn require_positive_measure_if_present(
    value: Option<Decimal>,
    label: &str,
) -> Result<(), AppError> {
    if let Some(v) = value
        && v <= Decimal::ZERO
    {
        return Err(AppError::validation_displayable(format!(
            "{label}必须大于 0，留空表示未实测，填了就必须是正数"
        )));
    }
    Ok(())
}

/// 入库辅计量公斤数的唯一解析入口，返回要落库的公斤数。
///
/// 入参 `quantity_meters` 是主计量米数（已由 [`require_positive_quantity`] 保证为正），
/// `gram_weight` 与 `width` 是实测克重与幅宽（可缺），`quantity_kg` 是显式提交的公斤数（可缺），
/// `label` 是调用方给的行上下文说法（如"入库单第 3 行（产品 500）"），用于拒绝时点名是哪一行。
/// 传给谁是调用方把返回值写入库存行与流水的 `quantity_kg` 列，并原样透传本函数的拒绝文案。
/// 存什么存哪里是 `inventory_stocks.quantity_kg`（NOT NULL）与
/// `inventory_transactions.quantity_kg`（NOT NULL）。
///
/// 口径（两条入库链路必须一致）分三条。其一，显式提交了大于零的公斤数即以实称量为准，
/// 因为实称重量是码单上的观测量，优先于按克重幅宽推算的理论重量。其二，未提交公斤数而克重与
/// 幅宽齐全时，走 `DualUnitConverter` 单入口推算，换算报错即如实外显，不回退到别的值。其三，
/// 两条来源都给不出公斤数即拒绝。缺省落 0 公斤会让双计量台账凭空少一列真实量，而成本核算与
/// 按公斤出库都建立在这个辅量上，静默补零属把采集缺口伪装成实测值。
pub fn resolve_inbound_quantity_kg(
    quantity_meters: Decimal,
    gram_weight: Option<Decimal>,
    width: Option<Decimal>,
    quantity_kg: Option<Decimal>,
    label: &str,
) -> Result<Decimal, AppError> {
    if let Some(kg) = quantity_kg
        && kg > Decimal::ZERO
    {
        return Ok(kg);
    }
    if let (Some(gram_weight), Some(width)) = (gram_weight, width) {
        return DualUnitConverter::meters_to_kg(quantity_meters, gram_weight, width).map_err(|e| {
            AppError::validation_displayable(format!("{label} 双计量单位换算失败，{e}"))
        });
    }
    Err(AppError::validation_displayable(format!(
        "{label} 公斤数无法确定，请录入实测公斤数，或同时录入克重与幅宽由系统换算（都必须大于 0）"
    )))
}

/// 来源单号必填，返回 trim 后的单号。入参 `raw` 是提交原文，`label` 是界面字段说法。
///
/// 传给谁是调用方把返回值写入库存流水 `inventory_transactions.source_bill_no`，同批还有
/// `source_bill_type` 与 `source_bill_id`，由调用方按所属链路如实给出。
/// 存什么存哪里只落流水表，不改库存台账结构。
///
/// 必填理由是入库无痕属本域最重的一类缺陷。没有来源单号，事后无法把库存行回指到验布码单、
/// 生产工单或采购收货单，批次追溯链在起点即断。
pub fn require_source_bill_no(raw: Option<&str>, label: &str) -> Result<String, AppError> {
    let value = raw.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return Err(AppError::validation_displayable(format!(
            "{label}未填写，入库必须登记来源单号（验布码单号、生产工单号或收货单号）"
        )));
    }
    Ok(value.to_string())
}

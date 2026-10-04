//! 委外加工 DTO 子模块（outsourcing_ops/types）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` 迁移 10 个 DTO struct。
//! 包含委外订单/发料明细/收回入库单/凭证的 Create/Update/Query 请求体。

use rust_decimal::Decimal;
use serde::Deserialize;

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时直接调 visit_none()，
/// 把"显式 null"塌成外层 `None`，与"键缺席"不可区分。本适配器先按内层 `Option<T>` 反序列化
/// 再包一层：键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 形态与 handlers/department_handler.rs 中同名私有适配器一致（跨域合并到共享 utils 超本波授权范围）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

// ============================================================================
// 委外加工订单 DTO
// ============================================================================

/// 创建委外订单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingOrderRequest {
    pub order_no: String,
    pub order_type: String,
    pub supplier_id: i32,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub issue_date: chrono::NaiveDate,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub issue_quantity: Decimal,
    pub issue_unit: Option<String>,
    pub material_cost: Decimal,
    /// 加工费：outsourcing_order.processing_fee NOT NULL DECIMAL(14,4)
    /// （v15/mod.rs:3247，无 DB 默认值）——类型层非 Option（NOT NULL 列不标可选，
    /// 显式 null 被 serde 按类型错误拒绝，绝不落 NULL）。
    /// `#[serde(default)]` 仅表达「建单缺省键 = 0 起步」：与既有
    /// build_order_active_model 的 Set(ZERO) 初始化完全同值，费用真实录入走
    /// draft 期 PUT（order.rs:484 结算语义「需在订单更新时填入」）或建单直传。
    #[serde(default)]
    pub processing_fee: Decimal,
    /// 运费：outsourcing_order.freight_fee NOT NULL DECIMAL(14,4)（v15/mod.rs:3248），
    /// 三态口径同 processing_fee
    #[serde(default)]
    pub freight_fee: Decimal,
    /// 进项税额：outsourcing_order.tax_amount NOT NULL DECIMAL(14,4)（v15/mod.rs:3249），
    /// 三态口径同 processing_fee；结算 FEE 凭证单独记录在 voucher.tax_amount
    #[serde(default)]
    pub tax_amount: Decimal,
    pub standard_loss_rate: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新委外订单请求（仅 draft 状态可更新）
///
/// 三态语义（RFC 7386，对齐 handlers/department_handler.rs 范式）：
/// 键缺席=保持原值、显式 null=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（order_type/supplier_id/issue_date/issue_quantity/issue_unit/material_cost/
/// processing_fee/freight_fee/tax_amount，v15 outsourcing_order DDL v15/mod.rs:3227-3262，
/// 三费列 :3247-3249）显式 null 由 service 入口在任何 DB 访问前拒绝。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingOrderRequest {
    /// 委外类型：NOT NULL——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub order_type: Option<Option<String>>,
    /// 供应商 ID：NOT NULL——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub supplier_id: Option<Option<i32>>,
    /// 关联生产订单 ID：DB 可空 INTEGER——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub production_order_id: Option<Option<i32>>,
    /// 关联缸号 ID：DB 可空 INTEGER——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub dye_batch_id: Option<Option<i32>>,
    /// 色号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub color_no: Option<Option<String>>,
    /// 缸号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub dye_lot_no: Option<Option<String>>,
    /// 发料日期：NOT NULL DATE——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub issue_date: Option<Option<chrono::NaiveDate>>,
    /// 预计收回日期：DB 可空 DATE——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub expected_return_date: Option<Option<chrono::NaiveDate>>,
    /// 发出数量：NOT NULL DECIMAL——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub issue_quantity: Option<Option<Decimal>>,
    /// 发出单位：NOT NULL VARCHAR——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub issue_unit: Option<Option<String>>,
    /// 发出材料成本：NOT NULL DECIMAL——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub material_cost: Option<Option<Decimal>>,
    /// 加工费：NOT NULL DECIMAL(14,4)（v15/mod.rs:3247）——显式 null 被 service 拒绝，
    /// 有值覆盖时联动重算 total_cost（settle 语义 order.rs:484：FEE 凭证=加工费+运费）
    #[serde(default, deserialize_with = "double_option")]
    pub processing_fee: Option<Option<Decimal>>,
    /// 运费：NOT NULL DECIMAL(14,4)（v15/mod.rs:3248）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub freight_fee: Option<Option<Decimal>>,
    /// 进项税额：NOT NULL DECIMAL(14,4)（v15/mod.rs:3249）——显式 null 被 service 拒绝，
    /// 结算时单独落 FEE 凭证 tax_amount 列（order.rs:525）
    #[serde(default, deserialize_with = "double_option")]
    pub tax_amount: Option<Option<Decimal>>,
    /// 标准损耗率：DB 可空 DECIMAL——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub standard_loss_rate: Option<Option<Decimal>>,
    /// 备注：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
}

/// 委外订单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OutsourcingOrderQuery {
    pub order_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
    pub issue_date_from: Option<chrono::NaiveDate>,
    pub issue_date_to: Option<chrono::NaiveDate>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 委外加工发料明细 DTO
// ============================================================================

/// 创建委外发料明细请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingOrderItemRequest {
    pub outsourcing_order_id: i32,
    pub product_id: i32,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub unit_cost: Decimal,
    pub remarks: Option<String>,
    /// 缺陷 2.1：关联胚布ID（精确到卷/匹级追溯）
    pub greige_fabric_id: Option<i32>,
    /// batch-18 P2-2：该缸号/匹号的加工费
    pub processing_fee: Option<Decimal>,
    /// batch-18 P2-2：该缸号/匹号的运费
    pub freight_fee: Option<Decimal>,
    /// 生产匹号（外发染色发料引用的生产匹，匹号领域）
    pub piece_no: Option<String>,
}

/// 更新委外发料明细请求
///
/// 三态语义（RFC 7386）：键缺席=保持原值、显式 null=清空（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（quantity/unit/unit_cost，v15 outsourcing_order_item 建表；
/// processing_fee/freight_fee，v15:2195-2196 ALTER NOT NULL DEFAULT 0）显式 null 由 service 拒绝。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingOrderItemRequest {
    /// 色号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub color_no: Option<Option<String>>,
    /// 缸号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub dye_lot_no: Option<Option<String>>,
    /// 匹号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub batch_no: Option<Option<String>>,
    /// 发出仓库 ID：DB 可空 INTEGER——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub warehouse_id: Option<Option<i32>>,
    /// 发出数量：NOT NULL DEFAULT 0——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub quantity: Option<Option<Decimal>>,
    /// 单位：NOT NULL DEFAULT 'kg'——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub unit: Option<Option<String>>,
    /// 单位成本：NOT NULL DEFAULT 0——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub unit_cost: Option<Option<Decimal>>,
    /// 备注：DB 可空 TEXT——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
    /// 加工费：NOT NULL DEFAULT 0（v15:2196）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub processing_fee: Option<Option<Decimal>>,
    /// 运费：NOT NULL DEFAULT 0（v15:2195）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub freight_fee: Option<Option<Decimal>>,
}

// ============================================================================
// 委外收回入库单 DTO
// ============================================================================

/// 创建委外收回入库单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingReceiptRequest {
    pub receipt_no: String,
    pub outsourcing_order_id: i32,
    pub receipt_date: chrono::NaiveDate,
    pub product_id: i32,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub return_quantity: Decimal,
    pub loss_quantity: Option<Decimal>,
    pub quality_status: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    // 收回匹实测值三列（m0075 建列，取值域门见 outsourcing_ops/receipt.rs 的 validate_measured_value）：
    // 入参可空、如实透传——有值必落，键缺席或显式 null 保持 NULL（= 未补录），不回落 products 主数据、
    // 不塞默认值；非空时必须 > 0。创建侧不强制必填，缺值由标签侧 fail-closed 逐列点名拒绝。
    /// 实测重量（千克，>0；DB 可空列 outsourcing_receipt.weight）
    pub weight: Option<Decimal>,
    /// 实测幅宽（cm，>0；DB 可空列 outsourcing_receipt.width）
    pub width: Option<Decimal>,
    /// 实测克重（g/m²，>0；DB 可空列 outsourcing_receipt.gram_weight）
    pub gram_weight: Option<Decimal>,
}

/// 更新委外收回入库单请求（仅 draft 状态可更新）
///
/// 三态语义（RFC 7386）：键缺席=保持原值、显式 null=清空（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（receipt_date/product_id/return_quantity/loss_quantity，
/// v15 outsourcing_receipt 建表）显式 null 由 service 拒绝。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingReceiptRequest {
    /// 收回日期：NOT NULL DATE——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub receipt_date: Option<Option<chrono::NaiveDate>>,
    /// 成品 ID：NOT NULL INTEGER——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub product_id: Option<Option<i32>>,
    /// 色号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub color_no: Option<Option<String>>,
    /// 缸号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub dye_lot_no: Option<Option<String>>,
    /// 匹号：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub batch_no: Option<Option<String>>,
    /// 入库仓库 ID：DB 可空 INTEGER——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub warehouse_id: Option<Option<i32>>,
    /// 收回数量：NOT NULL DEFAULT 0——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub return_quantity: Option<Option<Decimal>>,
    /// 损耗数量：NOT NULL DEFAULT 0（实体 Model 为非 Option Decimal）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub loss_quantity: Option<Option<Decimal>>,
    /// 质检结论：DB 可空 VARCHAR（词表 outsourcing_receipt_quality_status）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub quality_status: Option<Option<String>>,
    /// 等级：DB 可空 VARCHAR——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub grade: Option<Option<String>>,
    /// 备注：DB 可空 TEXT——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
    // ========== #220 收回匹实测值三列（m0075，DB 可空无默认）==========
    // 三态语义与其余可空列一致：键缺席=保持原值、显式 null=清空回"未补录"、有值=覆盖。
    // 清空只把匹行/收回单退回 NULL（标签继续 fail-closed 点名），绝不代表回落主数据。
    /// 实测重量（千克）：DB 可空 DECIMAL——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub weight: Option<Option<Decimal>>,
    /// 实测幅宽（cm）：DB 可空 DECIMAL——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub width: Option<Option<Decimal>>,
    /// 实测克重（g/m²）：DB 可空 DECIMAL——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub gram_weight: Option<Option<Decimal>>,
}

/// 委外收回入库单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OutsourcingReceiptQuery {
    pub outsourcing_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
    pub receipt_date_from: Option<chrono::NaiveDate>,
    pub receipt_date_to: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 委外加工会计分录凭证 DTO
// ============================================================================

/// 创建委外凭证请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingVoucherRequest {
    pub voucher_no: String,
    pub outsourcing_order_id: i32,
    pub voucher_type: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: Decimal,
    pub tax_amount: Option<Decimal>,
    pub voucher_date: chrono::NaiveDate,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 委外凭证查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OutsourcingVoucherQuery {
    pub outsourcing_order_id: Option<i32>,
    pub voucher_type: Option<String>,
    pub is_posted: Option<bool>,
    pub voucher_date_from: Option<chrono::NaiveDate>,
    pub voucher_date_to: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

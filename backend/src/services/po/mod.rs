//! 采购服务模块（po = purchase order）
//!
//! 由原 `services/purchase_order_service.rs`（1752 行）按业务子领域拆分而来。
//! 子模块：
//! - `order`    采购订单（核心 CRUD / 生命周期）
//! - `contract` 采购合同（审批工作流：提交、审批、拒绝）
//! - `receipt`  采购收货（含库存联动）
//! - `price`    采购价格（采购建议、预算占用）
//! - `purchase_return`采购退货（占位模块，待后续扩展）
//!
//! 兼容说明：原 `crate::services::po::order::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::po::*;` 重新导出以保持向后兼容。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::utils::error::AppError;

pub mod contract;
pub mod order;
pub mod order_ops;
pub mod price;
pub mod receipt;

// =====================================================
// 请求 DTO（与原 purchase_order_service.rs 末尾保持一致）
// =====================================================

/// 创建采购订单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    /// 供应商 ID
    #[validate(range(min = 1, message = "供应商ID必须大于0"))]
    pub supplier_id: i32,

    /// 订单日期
    pub order_date: NaiveDate,

    /// 预计交货日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 部门 ID
    pub department_id: Option<i32>,

    /// 币种
    #[validate(length(max = 10, message = "币种长度不能超过10个字符"))]
    pub currency: Option<String>,

    /// 汇率
    pub exchange_rate: Option<Decimal>,

    /// 付款条件
    #[validate(length(max = 200, message = "付款条件长度不能超过200个字符"))]
    pub payment_terms: Option<String>,

    /// 运输条款
    #[validate(length(max = 200, message = "运输条款长度不能超过200个字符"))]
    pub shipping_terms: Option<String>,

    /// 备注
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 来源销售订单 ID（转采购时传入，触发 SKU 对照翻译）
    pub source_sales_order_id: Option<i32>,

    /// 资质例外放行原因（可空）：供应商存在已过期法定许可类资质（阻断一切新建单）或
    /// 已过期一般资质且为本供应商首单（脱离草稿的历史订单为零）时，订单创建门控默认拒绝；
    /// 携带本原因且操作人角色持例外放行权限键才放行（键面与门控实现见
    /// services::supplier_qualification_gate 模块头注释），放行与原因同事务落 audit_logs。
    /// 非空白约束由 validator 保证（传了字段就不得为空白，静默当"未申请"是禁止写法）。
    #[validate(length(
        min = 1,
        max = 500,
        message = "资质例外放行原因不能为空白且不能超过500字符"
    ))]
    pub qualification_waiver_reason: Option<String>,

    /// 订单明细
    #[validate(nested)]
    #[validate(length(min = 1, message = "订单至少需要一行明细"))]
    pub items: Option<Vec<CreateOrderItemRequest>>,
}

/// 更新采购订单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseOrderRequest {
    pub supplier_id: Option<i32>,
    pub order_date: Option<NaiveDate>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

/// 创建订单明细请求
#[derive(Debug, Clone, Validate, Deserialize, Serialize)]
pub struct CreateOrderItemRequest {
    /// 行号
    pub line_no: Option<i32>,

    /// 物料 ID
    pub material_id: Option<i32>,

    /// 单价
    pub unit_price: Option<Decimal>,

    /// 订购数量（主单位）
    pub quantity_ordered: Option<Decimal>,

    /// 订购数量（辅助单位）
    pub quantity_alt_ordered: Option<Decimal>,

    /// 税率
    pub tax_rate: Option<Decimal>,

    /// 折扣百分比
    pub discount_percent: Option<Decimal>,

    /// 交货数量允收容差（百分比，可空）：NULL = 走默认解析（品类 > 全局），
    /// 非空 = 行级覆盖（含「约」订单写 10.00）。
    #[validate(custom(function = "validate_quantity_tolerance_pct"))]
    pub quantity_tolerance_pct: Option<Decimal>,

    /// 色号（转采购场景下从销售订单明细带入，用于反查供应商 SKU 对照）
    pub color_no: Option<String>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新订单明细请求（写侧字段与 [`CreateOrderItemRequest`] 对齐）
///
/// 「缺省即不改」语义：各字段均为 Option，None = 未提交、保持原值（既有
/// `if let Some(v)` 范式）。行级允差「显式清回 NULL（改用品类/全局默认）」
/// 本端点尚不可表达——该扩展需三态契约（如 sentinel），属后续联调项，勿把
/// None 当清空处理，否则会误删既有行级覆盖。
/// 保密约束：`supplier_product_code`/`supplier_color_no` 快照列不在本请求字段中——
/// 只能由服务层按 color_no 走与创建路径同一套映射反查权威逻辑派生，禁止前端直写。
#[derive(Debug, Default, Deserialize, Serialize, Validate)]
pub struct UpdateOrderItemRequest {
    pub material_id: Option<i32>,
    pub unit_price: Option<Decimal>,
    pub quantity_ordered: Option<Decimal>,

    /// 订购数量（辅助单位）
    pub quantity_alt_ordered: Option<Decimal>,

    pub tax_rate: Option<Decimal>,

    /// 折扣百分比
    pub discount_percent: Option<Decimal>,

    /// 交货数量允收容差（百分比）：与创建路径复用同一校验函数
    /// [`validate_quantity_tolerance_pct`]（Some 时 0~100），不另写一套规则。
    #[validate(custom(function = "validate_quantity_tolerance_pct"))]
    pub quantity_tolerance_pct: Option<Decimal>,

    /// 色号：写侧键名与创建请求一致（`color_no`），落库列为 `color_code`（既有口径）。
    /// 普通单行直写列（与创建路径同口径，不反查）；仅转采购行（行上已带供应商
    /// 快照列）提交色号才触发供应商 SKU 对照反查、刷新保密快照列，反查/解析
    /// 与创建路径共用权威函数（见 order_ops::crud）。
    pub color_no: Option<String>,

    pub notes: Option<String>,
}

impl UpdateOrderItemRequest {
    /// 写侧入口校验：行级允差越界（<0 或 >100）的拒绝文案只涉及用户自己提交的
    /// 数值与公开范围规则，按 `business_displayable` 如实外显；不得走
    /// `From<ValidationErrors>` → `AppError::validation` 的脱敏信封
    /// （用户只会看到「请求参数验证失败」，看不到被拒原因）。
    pub fn validate_write(&self) -> Result<(), AppError> {
        self.validate().map_err(|errors| {
            let detail = errors.errors().values().find_map(|kind| match kind {
                validator::ValidationErrorsKind::Field(errs) => errs.first().map(|e| {
                    e.message
                        .clone()
                        .unwrap_or_else(|| e.code.clone())
                        .into_owned()
                }),
                // 本 DTO 无嵌套/列表字段；兜底原样序列化错误集文本，不吞原因。
                // List 变体的载荷是 BTreeMap<下标, Box<ValidationErrors>>（不实现 Display），
                // 必须取其内层 ValidationErrors 才能格式化，直接 .to_string() 编译不过。
                validator::ValidationErrorsKind::Struct(inner) => Some(inner.to_string()),
                validator::ValidationErrorsKind::List(items) => {
                    items.values().next().map(|inner| inner.to_string())
                }
            });
            AppError::business_displayable(detail.unwrap_or_else(|| errors.to_string()))
        })
    }
}

/// 交货允差百分比范围校验：Some 时必须在 [0, 100] 区间内。
/// validator 框架对 Option<T> 自动解包，None 时跳过不校验。
fn validate_quantity_tolerance_pct(value: &Decimal) -> Result<(), ValidationError> {
    use crate::utils::delivery_tolerance::{TOLERANCE_PCT_MAX, TOLERANCE_PCT_MIN};
    if *value < TOLERANCE_PCT_MIN || *value > TOLERANCE_PCT_MAX {
        return Err(ValidationError::new(
            "交货允差百分比(quantity_tolerance_pct)必须在0~100之间",
        ));
    }
    Ok(())
}

// =====================================================
// 统一对外导出（兼容旧路径 + 子模块直接访问）
// =====================================================
// 批次 325 v10 复审修复：移除未使用的 pub use 重导出（外部通过 crate::services::po::order::PurchaseOrderService 引用）

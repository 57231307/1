//! 销售服务模块（so = sales order）
//!
//! 由原 `services/sales_service.rs`（1661 行）按业务子领域拆分而来。
//! 子模块：
//! - `order`    销售订单（核心 CRUD / 生命周期）
//! - `delivery` 销售发货（发货、库存扣减/释放、订单号生成）
//! - `price`    销售价格（占位模块，待后续扩展）
//! - `contract` 销售合同（占位模块，待后续扩展）
//! - `sales_return` 销售退货（占位模块，待后续扩展）
//!
//! P9-2 拆分：order 进一步拆分为 3 个子模块：
//! - `order_crud`     销售订单 CRUD 子模块
//! - `order_workflow` 销售订单工作流子模块
//! - `order_query`    销售订单查询子模块
//!
//! 兼容说明：原 `crate::services::so::order::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::so::*;` 重新导出以保持向后兼容。

use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::models::status::sales::fulfillment_source;
use crate::utils::error::AppError;

pub mod amounts;
pub mod contract;
pub mod delivery;
// 批次 488 D10-3 拆分：delivery 的业务实现子模块（ship/inventory/cancel/export/types）
pub mod delivery_ops;
pub mod order;
pub mod price;
pub mod sales_return;
// P9-2 新增 order 拆分后的 3 个子模块
pub mod order_crud;
pub mod order_workflow;
pub mod order_query;
// 缺陷 3 修复：面料行业版订单业务逻辑（原 handler 内联逻辑下沉）
pub mod fabric_order;

// =====================================================
// DTO 数据结构
// =====================================================

/// 销售订单详情响应
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderDetail {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub opportunity_id: Option<i32>,
    pub order_date: chrono::DateTime<chrono::Utc>,
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    pub ship_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub subtotal: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub discount_amount: rust_decimal::Decimal,
    pub shipping_cost: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub paid_amount: rust_decimal::Decimal,
    pub balance_amount: rust_decimal::Decimal,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
    pub created_by: Option<i32>,
    /// 创建人姓名（由 LEFT JOIN 到 `users` 表的 `real_name` 列富化；非实体列）
    pub creator_name: Option<String>,
    /// 收货联系人快照（`sales_order.contact_person`）：编辑表单预填用
    pub contact_person: Option<String>,
    /// 收货联系电话快照（`sales_order.contact_phone`）：编辑表单预填用
    pub contact_phone: Option<String>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 审批通过理由：sales_orders.approval_reason 真实列（m0079 加列，TEXT 可空，
    /// models/sales_order.rs:49）。写入方 `services/so/order_workflow.rs` approve 路径；
    /// 选填口径：空/纯空白归一为 None ⇒ 列保持 NULL=未采集。键名与列名逐字同源
    /// snake_case，无 skip_serializing_if：键恒存在，未采集出参为 null（前端据此
    /// 区分「未审批」与「字段缺失」）。
    pub approval_reason: Option<String>,
    /// 审批拒绝理由：sales_orders.rejected_reason 真实列（m0079 加列，TEXT 可空，
    /// models/sales_order.rs:51）。写入方 `services/so/contract.rs` reject 路径
    /// （必填，trim 非空）；reject 起不再挪用 notes。两动作两列：approve 不写本列、
    /// reject 不写 approval_reason。sales_orders 无 cancel_reason 列（m0079 该域仅
    /// 建两列），本域出参不存在该键。
    pub rejected_reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[sea_orm(skip)]
    pub items: Vec<SalesOrderItemDetail>,
}

/// 销售订单明细项详情
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderItemDetail {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub discount_percent: rust_decimal::Decimal,
    pub tax_percent: rust_decimal::Decimal,
    pub subtotal: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub discount_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub shipped_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub color_no: String,
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub grade_required: Option<String>,
    pub quantity_meters: rust_decimal::Decimal,
    pub quantity_kg: rust_decimal::Decimal,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub color_extra_cost: rust_decimal::Decimal,
    pub grade_price_diff: rust_decimal::Decimal,
    pub final_price: Option<rust_decimal::Decimal>,
    pub shipped_quantity_meters: rust_decimal::Decimal,
    pub shipped_quantity_kg: rust_decimal::Decimal,
    /// 行级交付容差百分比（NULL = 走默认解析，非空 = 行级覆盖）
    pub quantity_tolerance_pct: Option<rust_decimal::Decimal>,
    /// 流转链来源（行级）：NULL = 未定档，非空取值只认权威词表
    /// `models/status/sales.rs::fulfillment_source`（production/stock/purchase）。
    /// 键恒存在（无 skip_serializing_if），未采集出参为 null。
    pub fulfillment_source: Option<String>,
}

/// 创建销售订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSalesOrderRequest {
    #[validate(range(min = 1, message = "客户ID必须大于0"))]
    pub customer_id: i32,
    pub opportunity_id: Option<i32>,
    /// 订单日期：可空，缺省落库为当前时间
    pub order_date: Option<chrono::DateTime<chrono::Utc>>,
    /// 收货联系人快照（可从客户档案带出）
    pub contact_person: Option<String>,
    /// 收货联系电话快照
    pub contact_phone: Option<String>,
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    /// 订单初始状态：只允许缺省（服务端定初值落 sales_order::CREATE_INITIAL）或与之
    /// 逐字符相同的取值，其余状态由提交、审批等权威操作产生，建单直传一律拒绝
    #[validate(length(max = 50, message = "状态长度不能超过50个字符"))]
    pub status: Option<String>,
    #[validate(length(max = 500, message = "收货地址长度不能超过500个字符"))]
    pub shipping_address: Option<String>,
    #[validate(length(max = 500, message = "账单地址长度不能超过500个字符"))]
    pub billing_address: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
    #[validate(length(min = 1, message = "订单项不能为空"), nested)]
    pub items: Vec<SalesOrderItemRequest>,
    #[validate(length(max = 100, message = "付款条件长度不能超过100个字符"))]
    pub payment_terms: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
    #[validate(length(max = 50, message = "批次号长度不能超过50个字符"))]
    pub batch_no: Option<String>,
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "染缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
    #[validate(length(max = 20, message = "等级长度不能超过20个字符"))]
    pub grade: Option<String>,
    #[validate(length(max = 200, message = "包装要求长度不能超过200个字符"))]
    pub packaging_requirement: Option<String>,
    #[validate(length(max = 200, message = "质量标准长度不能超过200个字符"))]
    pub quality_standard: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct SalesOrderItemRequest {
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    /// 不含税单价（派生值）：提交则必须与按含税单价反算出的净额单价分位一致，
    /// 不符即点名拒绝，服务端采信的是反算值（落 sales_order_item.unit_price 列）。
    pub unit_price: rust_decimal::Decimal,
    /// 含税单价（销售录入事实）。税率非 0 时必须提交：价税分离只允许
    /// 「净额 = 含税 ÷ (1 + 税率% ÷ 100)、税额 = 含税 − 净额」，
    /// 缺本字段又带非 0 税率由服务层点名拒绝，服务端不用 `单价 × (1 + 税率)` 造含税价。
    /// 0 税率（内销不含税/免税）行可不提交，此时不含税单价与含税单价是同一个数。
    pub unit_price_with_tax: Option<rust_decimal::Decimal>,
    pub discount_percent: Option<rust_decimal::Decimal>,
    pub tax_percent: Option<rust_decimal::Decimal>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "颜色名称长度不能超过50个字符"))]
    pub color_name: Option<String>,
    #[validate(length(max = 50, message = "潘通色号长度不能超过50个字符"))]
    pub pantone_code: Option<String>,
    #[validate(length(max = 20, message = "要求等级长度不能超过20个字符"))]
    pub grade_required: Option<String>,
    pub quantity_meters: Option<rust_decimal::Decimal>,
    pub quantity_kg: Option<rust_decimal::Decimal>,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    #[validate(length(max = 100, message = "批次要求长度不能超过100个字符"))]
    pub batch_requirement: Option<String>,
    #[validate(length(max = 100, message = "染批要求长度不能超过100个字符"))]
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub color_extra_cost: Option<rust_decimal::Decimal>,
    pub grade_price_diff: Option<rust_decimal::Decimal>,
    pub final_price: Option<rust_decimal::Decimal>,
    /// 匹号（染色匹号贯穿销售订单条目，匹号领域一期）
    pub piece_no: Option<String>,
    /// 交货数量允收容差（百分比，可空）：NULL = 走默认解析（品类 > 全局），
    /// 非空 = 行级覆盖（含「约」订单写 10.00）。
    /// 范围校验 [0, 100]：负值使门控上下界倒转，超大值使超收门控失效；
    /// None 合法（不覆盖，走默认解析）。
    #[validate(custom(function = "validate_quantity_tolerance_pct"))]
    pub quantity_tolerance_pct: Option<rust_decimal::Decimal>,
    /// 流转链来源（行级，可空）：只接受权威词表
    /// `models/status/sales.rs::fulfillment_source` 的 production/stock/purchase；
    /// 不填 = 保持 NULL（未定档），服务端不替调用方填默认值。
    /// 词表外取值由建单与改单的共同漏斗入口点名拒绝（见
    /// [`SalesOrderItemRequest::validate_fulfillment_source`]），不落库。
    pub fulfillment_source: Option<String>,
}

impl SalesOrderItemRequest {
    /// 写侧入口校验：链来源只认闭合三值词表，词表外一律拒并点名允许清单。
    ///
    /// 调用方：`services/so/order_crud.rs` 建单与改单共用的明细行漏斗
    /// `create_order_items_and_calculate_totals`（两条写链都在它上面，落库前必过本函数）。
    /// `None`（未提交）是合法语义 = 未定档，直接放行、不填默认值；
    /// `Some(词表外 token)` 走 `AppError::validation_displayable` 如实外显
    /// 「提交的值 + 允许清单」，文案不含记录 ID、不含 serde/驱动原文。
    pub fn validate_fulfillment_source(&self) -> Result<(), AppError> {
        match self.fulfillment_source.as_deref() {
            None => Ok(()),
            Some(token) if fulfillment_source::is_valid(token) => Ok(()),
            Some(token) => {
                // 失败必留痕：违规 token 与产品 ID 只进日志，出参文案不带记录 ID。
                tracing::warn!(
                    product_id = self.product_id,
                    rejected_token = token,
                    allowed = fulfillment_source::ALL.join("/"),
                    "sales_order_item.fulfillment_source 词表外取值被拒：整笔明细写入中止"
                );
                Err(AppError::validation_displayable(format!(
                    "明细行链来源 {token} 不是合法取值，只能填 {} 之一；未定档请留空不传（留空即 NULL，服务端不代填默认值）",
                    fulfillment_source::ALL.join("/")
                )))
            }
        }
    }
}

/// 行级交付容差百分比范围校验 [0, 100]。
///
/// 由 `validator` 框架在 `Option<T>` 字段上自动解包：`None` 跳过（合法，不覆盖），
/// `Some(v)` 时调用本函数校验 `0 <= v <= 100`。
/// 负值会使容差门控上下界倒转、超大正值使超收门控失效。
fn validate_quantity_tolerance_pct(v: &rust_decimal::Decimal) -> Result<(), ValidationError> {
    use crate::utils::delivery_tolerance::{TOLERANCE_PCT_MAX, TOLERANCE_PCT_MIN};
    if *v < TOLERANCE_PCT_MIN || *v > TOLERANCE_PCT_MAX {
        return Err(ValidationError::new("交货容差百分比必须在 0 到 100 之间"));
    }
    Ok(())
}

/// 更新销售订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSalesOrderRequest {
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    /// 订单状态：只允许与该行当前态逐字符相同的写入（幂等，语义是不改状态）。
    /// 任何改态企图由编辑保存口状态门拒绝，状态推进走提交、审批、拒绝、取消、发货、完成权威操作
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    #[validate(nested)]
    pub items: Option<Vec<SalesOrderItemRequest>>,
}

// =====================================================
// 统一对外导出
// =====================================================
// 批次 325 v10 复审修复：移除未使用的 pub use 重导出（外部均通过完整子模块路径引用）
// ShipOrderRequest → crate::services::so::delivery::ShipOrderRequest
// SalesService → crate::services::so::order::SalesService

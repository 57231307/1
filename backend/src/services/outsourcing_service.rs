//! 委外加工物资 Service（facade）
//!
//! v14 批次 430：委托加工物资贯通
//! 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算 + §5.5 委外织布场景 + §5.7 损耗率标准 + §6.5 委托加工模式
//!
//! 真实业务流程（§5.4 三步分录）：
//! 发料——借 委托加工物资 / 贷 自制半成品-胚布
//! 加工费——借 委托加工物资+应交税费-进项税额 / 贷 银行存款
//! 入库——借 库存商品-成品布 / 贷 委托加工物资（合理损耗只影响单位成本，不影响总成本）
//!
//! 损耗处理规则（§5.4 + §5.7）：
//! 正常损耗摊入委托加工物资成本，按实际收回数量结转（不单独做分录）
//! 非正常损耗计入营业外支出/管理费用，不能进成本
//!
//! 核心能力：
//! 委外订单 CRUD + 状态机（draft→issued→processing→received→settled→closed）+ 取消
//! 委外发料明细 CRUD + 按订单查询
//! 委外收回入库单 CRUD + 状态机（draft→confirmed）+ 损耗分类与单位成本计算
//! 委外会计分录凭证 CRUD + 过账（issue/fee/receipt/loss 四类凭证）
//!
//! 复用现有功能（§10.0.1）：
//! suppliers 表（委外加工厂关联）、production_orders 表（关联生产订单）、dye_batch 表（关联缸号）、products / warehouses 表（物料与仓库）
//!
//! 批次 489 D10-2b 拆分：本文件作为 facade，保留纯函数 + Service struct + new 构造函数 + 测试。
//! 4 个 Service 的 impl 块迁移至 `outsourcing_ops` 子模块（order / order_item / receipt / voucher）。
//! DTO struct 迁移至 `outsourcing_ops::types`，本 facade 通过 `pub use` 二次 re-export 保持外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::status::outsourcing_loss_type;
use crate::models::status::outsourcing_order_status;
use crate::models::status::outsourcing_order_type;
use crate::models::status::outsourcing_voucher_type;
use crate::utils::error::AppError;

// re-export DTOs 与 ops 子模块，保持外部 `use crate::services::outsourcing_service::{...}` 路径不变
pub use crate::services::outsourcing_ops::{
    CreateOutsourcingOrderItemRequest, CreateOutsourcingOrderRequest,
    CreateOutsourcingReceiptRequest, CreateOutsourcingVoucherRequest, OutsourcingOrderQuery,
    OutsourcingReceiptQuery, OutsourcingVoucherQuery, UpdateOutsourcingOrderItemRequest,
    UpdateOutsourcingOrderRequest, UpdateOutsourcingReceiptRequest,
};

// ============================================================================
// 委外加工计算纯函数
// ============================================================================

/// 计算损耗率 = loss_quantity / issue_quantity（业务规则：若发出数量为 0，返回 0（避免除零））
pub fn compute_loss_rate(loss_quantity: Decimal, issue_quantity: Decimal) -> Decimal {
    if issue_quantity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    loss_quantity / issue_quantity
}

/// 计算总成本 = 材料成本 + 加工费 + 运费 - 非正常损耗金额（业务规则（§5.4）：正常损耗摊入成本（不影响总成本，只影响单位成本）；非正常损耗金额从总成本中扣除（计入营业外支出））
pub fn compute_total_cost(
    material_cost: Decimal,
    processing_fee: Decimal,
    freight_fee: Decimal,
    abnormal_loss_amount: Decimal,
) -> Decimal {
    material_cost + processing_fee + freight_fee - abnormal_loss_amount
}

/// 计算单位成本 = 总成本 / 收回数量（业务规则：若收回数量为 0，返回 0（避免除零）；正常损耗只影响单位成本，不影响总成本）
pub fn compute_unit_cost(total_cost: Decimal, return_quantity: Decimal) -> Decimal {
    if return_quantity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    total_cost / return_quantity
}

/// 计算标准损耗率（按工序）
/// 业务规则（§5.7 行业通用损耗率标准，取中值）：dyeing(染色) = 0.05（印染工序 4%-6%，取中值 5%）；weaving(织布) = 0.035（织布工序 2%-5%，取中值 3.5%）；printing(印花) = 0.05（同印染工序）；finishing(后整理) = 0.03（后整理损耗较低）；other(其他) = 0.0（无标准）
pub fn compute_standard_loss_rate(order_type: &str) -> Decimal {
    match order_type {
        outsourcing_order_type::DYEING | outsourcing_order_type::PRINTING => {
            Decimal::new(5, 2) // 0.05
        }
        outsourcing_order_type::WEAVING => Decimal::new(35, 3), // 0.035
        outsourcing_order_type::FINISHING => Decimal::new(3, 2), // 0.03
        _ => Decimal::ZERO,
    }
}

/// 损耗分类：根据实际损耗率与标准损耗率比较
/// 业务规则（§5.4 + §5.7）：actual <= standard 返回 "normal"（正常损耗，摊入成本）；actual > standard 返回 "abnormal"（非正常损耗，计入营业外支出）
pub fn classify_loss(actual_loss_rate: Decimal, standard_loss_rate: Decimal) -> &'static str {
    if actual_loss_rate <= standard_loss_rate {
        outsourcing_loss_type::NORMAL
    } else {
        outsourcing_loss_type::ABNORMAL
    }
}

/// 计算非正常损耗金额（业务规则（§5.4）：超定额损耗 = max(0, 实际损耗 - 发出 × 标准损耗率)；非正常损耗金额 = 超定额损耗 × 单位材料成本；单位材料成本 = 材料成本 / 发出数量；若发出数量为 0，返回 0）
pub fn compute_abnormal_loss_amount(
    issue_quantity: Decimal,
    return_quantity: Decimal,
    unit_material_cost: Decimal,
    standard_loss_rate: Decimal,
) -> Decimal {
    if issue_quantity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let actual_loss_quantity = issue_quantity - return_quantity;
    let standard_loss_quantity = issue_quantity * standard_loss_rate;
    let excess_loss = actual_loss_quantity - standard_loss_quantity;
    if excess_loss <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    excess_loss * unit_material_cost
}

/// 校验委外类型是否合法
pub fn validate_order_type(order_type: &str) -> Result<(), AppError> {
    let valid_types = [
        outsourcing_order_type::DYEING,
        outsourcing_order_type::PRINTING,
        outsourcing_order_type::WEAVING,
        outsourcing_order_type::FINISHING,
        outsourcing_order_type::OTHER,
    ];
    if !valid_types.contains(&order_type) {
        return Err(AppError::business(format!(
            "委外类型必须是 dyeing / printing / weaving / finishing / other，当前: {}",
            order_type
        )));
    }
    Ok(())
}

/// 校验委外订单状态是否合法
pub fn validate_order_status(status: &str) -> Result<(), AppError> {
    let valid = [
        outsourcing_order_status::DRAFT,
        outsourcing_order_status::ISSUED,
        outsourcing_order_status::PROCESSING,
        outsourcing_order_status::RECEIVED,
        outsourcing_order_status::SETTLED,
        outsourcing_order_status::CLOSED,
        outsourcing_order_status::CANCELLED,
    ];
    if !valid.contains(&status) {
        return Err(AppError::business(format!(
            "委外订单状态必须是 draft / issued / processing / received / settled / closed / cancelled，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 校验损耗类型是否合法
pub fn validate_loss_type(loss_type: &str) -> Result<(), AppError> {
    let valid = [
        outsourcing_loss_type::NORMAL,
        outsourcing_loss_type::ABNORMAL,
    ];
    if !valid.contains(&loss_type) {
        return Err(AppError::business(format!(
            "损耗类型必须是 normal / abnormal，当前: {}",
            loss_type
        )));
    }
    Ok(())
}

/// 校验凭证类型是否合法
pub fn validate_voucher_type(voucher_type: &str) -> Result<(), AppError> {
    let valid = [
        outsourcing_voucher_type::ISSUE,
        outsourcing_voucher_type::FEE,
        outsourcing_voucher_type::RECEIPT,
        outsourcing_voucher_type::LOSS,
    ];
    if !valid.contains(&voucher_type) {
        return Err(AppError::business(format!(
            "凭证类型必须是 issue / fee / receipt / loss，当前: {}",
            voucher_type
        )));
    }
    Ok(())
}

// ============================================================================
// 委外加工 Service struct 定义（facade）
// ============================================================================
//
// 4 个 Service struct 与 `new` 构造函数保留在本 facade 中。
// impl 块迁移至 `outsourcing_ops` 子模块，Rust 允许同一 crate 多文件多 impl 块。
// `db` 字段使用 `pub(crate)` 可见性，供 ops 子模块的 impl 块访问。

/// 委外加工订单 Service
pub struct OutsourcingOrderService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl OutsourcingOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 委外加工发料明细 Service
pub struct OutsourcingOrderItemService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl OutsourcingOrderItemService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 委外收回入库单 Service
pub struct OutsourcingReceiptService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl OutsourcingReceiptService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 委外加工会计分录凭证 Service
pub struct OutsourcingVoucherService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl OutsourcingVoucherService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

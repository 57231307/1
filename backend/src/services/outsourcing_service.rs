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
    CreateOutsourcingOrderItemRequest, CreateOutsourcingOrderRequest, CreateOutsourcingReceiptRequest,
    CreateOutsourcingVoucherRequest, OutsourcingOrderQuery, OutsourcingReceiptQuery,
    OutsourcingVoucherQuery, UpdateOutsourcingOrderItemRequest, UpdateOutsourcingOrderRequest,
    UpdateOutsourcingReceiptRequest,
};

// ============================================================================
// 委外加工计算纯函数
// ============================================================================

/// 计算损耗率 = loss_quantity / issue_quantity
///
/// 业务规则：
/// - 若发出数量为 0，返回 0（避免除零）
pub fn compute_loss_rate(loss_quantity: Decimal, issue_quantity: Decimal) -> Decimal {
    if issue_quantity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    loss_quantity / issue_quantity
}

/// 计算总成本 = 材料成本 + 加工费 + 运费 - 非正常损耗金额
///
/// 业务规则（§5.4）：
/// - 正常损耗摊入成本（不影响总成本，只影响单位成本）
/// - 非正常损耗金额从总成本中扣除（计入营业外支出）
pub fn compute_total_cost(
    material_cost: Decimal,
    processing_fee: Decimal,
    freight_fee: Decimal,
    abnormal_loss_amount: Decimal,
) -> Decimal {
    material_cost + processing_fee + freight_fee - abnormal_loss_amount
}

/// 计算单位成本 = 总成本 / 收回数量
///
/// 业务规则：
/// - 若收回数量为 0，返回 0（避免除零）
/// - 正常损耗只影响单位成本，不影响总成本
pub fn compute_unit_cost(total_cost: Decimal, return_quantity: Decimal) -> Decimal {
    if return_quantity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    total_cost / return_quantity
}

/// 计算标准损耗率（按工序）
///
/// 业务规则（§5.7 行业通用损耗率标准，取中值）：
/// - dyeing(染色) = 0.05（印染工序 4%-6%，取中值 5%）
/// - weaving(织布) = 0.035（织布工序 2%-5%，取中值 3.5%）
/// - printing(印花) = 0.05（同印染工序）
/// - finishing(后整理) = 0.03（后整理损耗较低）
/// - other(其他) = 0.0（无标准）
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
///
/// 业务规则（§5.4 + §5.7）：
/// - actual <= standard 返回 "normal"（正常损耗，摊入成本）
/// - actual > standard 返回 "abnormal"（非正常损耗，计入营业外支出）
pub fn classify_loss(actual_loss_rate: Decimal, standard_loss_rate: Decimal) -> &'static str {
    if actual_loss_rate <= standard_loss_rate {
        outsourcing_loss_type::NORMAL
    } else {
        outsourcing_loss_type::ABNORMAL
    }
}

/// 计算非正常损耗金额
///
/// 业务规则（§5.4）：
/// - 超定额损耗 = max(0, 实际损耗 - 发出 × 标准损耗率)
/// - 非正常损耗金额 = 超定额损耗 × 单位材料成本
/// - 单位材料成本 = 材料成本 / 发出数量
/// - 若发出数量为 0，返回 0
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
    let valid = [outsourcing_loss_type::NORMAL, outsourcing_loss_type::ABNORMAL];
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn 测试计算损耗率_正常() {
        // 损耗 2 吨，发出 100 吨 → 2%
        let result = compute_loss_rate(Decimal::new(2, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::new(2, 2)); // 0.02
    }

    #[test]
    fn 测试计算损耗率_发出为零返回零() {
        let result = compute_loss_rate(Decimal::new(2, 0), Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算总成本_正常() {
        // 材料 500000 + 加工费 100000 + 运费 0 - 非正常损耗 0 = 600000
        let result = compute_total_cost(
            Decimal::new(500000, 0),
            Decimal::new(100000, 0),
            Decimal::ZERO,
            Decimal::ZERO,
        );
        assert_eq!(result, Decimal::new(600000, 0));
    }

    #[test]
    fn 测试计算总成本_扣除非正常损耗() {
        // 材料 500000 + 加工费 100000 + 运费 0 - 非正常损耗 5000 = 595000
        let result = compute_total_cost(
            Decimal::new(500000, 0),
            Decimal::new(100000, 0),
            Decimal::ZERO,
            Decimal::new(5000, 0),
        );
        assert_eq!(result, Decimal::new(595000, 0));
    }

    #[test]
    fn 测试计算单位成本_正常() {
        // 总成本 600000 / 收回 298 = 2013.4228...
        let result = compute_unit_cost(Decimal::new(600000, 0), Decimal::new(298, 0));
        assert!(result > Decimal::ZERO);
    }

    #[test]
    fn 测试计算单位成本_收回为零返回零() {
        let result = compute_unit_cost(Decimal::new(600000, 0), Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算标准损耗率_染色() {
        // dyeing 印染工序中值 5%
        let result = compute_standard_loss_rate(outsourcing_order_type::DYEING);
        assert_eq!(result, Decimal::new(5, 2));
    }

    #[test]
    fn 测试计算标准损耗率_织布() {
        // weaving 织布工序中值 3.5%
        let result = compute_standard_loss_rate(outsourcing_order_type::WEAVING);
        assert_eq!(result, Decimal::new(35, 3));
    }

    #[test]
    fn 测试计算标准损耗率_其他() {
        // other 无标准 0
        let result = compute_standard_loss_rate(outsourcing_order_type::OTHER);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试损耗分类_正常损耗() {
        // 实际 0.02 ≤ 标准 0.05 → normal
        let result = classify_loss(Decimal::new(2, 2), Decimal::new(5, 2));
        assert_eq!(result, outsourcing_loss_type::NORMAL);
    }

    #[test]
    fn 测试损耗分类_非正常损耗() {
        // 实际 0.08 > 标准 0.05 → abnormal
        let result = classify_loss(Decimal::new(8, 2), Decimal::new(5, 2));
        assert_eq!(result, outsourcing_loss_type::ABNORMAL);
    }

    #[test]
    fn 测试计算非正常损耗金额_正常无超定额() {
        // 发出 300，收回 298，损耗 2，标准 0.05 → 标准损耗 15，超定额 0
        let result = compute_abnormal_loss_amount(
            Decimal::new(300, 0),
            Decimal::new(298, 0),
            Decimal::new(1666, 0), // 单位材料成本
            Decimal::new(5, 2),    // 0.05
        );
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算非正常损耗金额_有超定额() {
        // 发出 100，收回 90，损耗 10，标准 0.05 → 标准损耗 5，超定额 5
        // 单位材料成本 1000 → 非正常损耗金额 5 × 1000 = 5000
        let result = compute_abnormal_loss_amount(
            Decimal::new(100, 0),
            Decimal::new(90, 0),
            Decimal::new(1000, 0),
            Decimal::new(5, 2),
        );
        assert_eq!(result, Decimal::new(5000, 0));
    }

    #[test]
    fn 测试校验委外类型_合法() {
        assert!(validate_order_type("dyeing").is_ok());
        assert!(validate_order_type("printing").is_ok());
        assert!(validate_order_type("weaving").is_ok());
        assert!(validate_order_type("finishing").is_ok());
        assert!(validate_order_type("other").is_ok());
    }

    #[test]
    fn 测试校验委外类型_非法() {
        assert!(validate_order_type("invalid").is_err());
    }

    #[test]
    fn 测试校验委外订单状态_合法() {
        assert!(validate_order_status("draft").is_ok());
        assert!(validate_order_status("issued").is_ok());
        assert!(validate_order_status("processing").is_ok());
        assert!(validate_order_status("received").is_ok());
        assert!(validate_order_status("settled").is_ok());
        assert!(validate_order_status("closed").is_ok());
        assert!(validate_order_status("cancelled").is_ok());
    }

    #[test]
    fn 测试校验委外订单状态_非法() {
        assert!(validate_order_status("invalid").is_err());
    }

    #[test]
    fn 测试校验损耗类型_合法() {
        assert!(validate_loss_type("normal").is_ok());
        assert!(validate_loss_type("abnormal").is_ok());
    }

    #[test]
    fn 测试校验损耗类型_非法() {
        assert!(validate_loss_type("invalid").is_err());
    }

    #[test]
    fn 测试校验凭证类型_合法() {
        assert!(validate_voucher_type("issue").is_ok());
        assert!(validate_voucher_type("fee").is_ok());
        assert!(validate_voucher_type("receipt").is_ok());
        assert!(validate_voucher_type("loss").is_ok());
    }

    #[test]
    fn 测试校验凭证类型_非法() {
        assert!(validate_voucher_type("invalid").is_err());
    }
}

//! 委外加工物资 Service
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

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::outsourcing_order::{
    self, ActiveModel as OrderActiveModel, Entity as OrderEntity, Model as OrderModel,
};
use crate::models::outsourcing_order_item::{
    self, ActiveModel as ItemActiveModel, Entity as ItemEntity, Model as ItemModel,
};
use crate::models::outsourcing_receipt::{
    self, ActiveModel as ReceiptActiveModel, Entity as ReceiptEntity, Model as ReceiptModel,
};
use crate::models::outsourcing_voucher::{
    self, ActiveModel as VoucherActiveModel, Entity as VoucherEntity, Model as VoucherModel,
};
use crate::models::status::outsourcing_loss_type;
use crate::models::status::outsourcing_order_status;
use crate::models::status::outsourcing_order_type;
use crate::models::status::outsourcing_receipt_status;
use crate::models::status::outsourcing_voucher_type;
use crate::utils::error::AppError;

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
// 委外加工订单 Service
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
    pub standard_loss_rate: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新委外订单请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingOrderRequest {
    pub order_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub issue_date: Option<chrono::NaiveDate>,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub issue_quantity: Option<Decimal>,
    pub issue_unit: Option<String>,
    pub material_cost: Option<Decimal>,
    pub standard_loss_rate: Option<Decimal>,
    pub remarks: Option<String>,
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

/// 委外加工订单 Service
pub struct OutsourcingOrderService {
    db: Arc<DatabaseConnection>,
}

/// 收回损耗与成本计算结果（record_receipt 内部传递）
struct ReceiptCalculation {
    loss_quantity: Decimal,
    actual_loss_rate: Decimal,
    loss_type_str: &'static str,
    is_loss_normal: bool,
    abnormal_loss_amount: Decimal,
    total_cost: Decimal,
    unit_cost: Decimal,
}

impl OutsourcingOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建委外订单（draft 状态）
    pub async fn create(&self, req: CreateOutsourcingOrderRequest) -> Result<OrderModel, AppError> {
        validate_order_type(&req.order_type)?;

        // 校验发出数量非负
        if req.issue_quantity < Decimal::ZERO {
            return Err(AppError::business("发出数量不能为负"));
        }
        // 校验材料成本非负
        if req.material_cost < Decimal::ZERO {
            return Err(AppError::business("发出材料成本不能为负"));
        }

        // 校验委外加工厂存在
        if crate::models::supplier::Entity::find_by_id(req.supplier_id)
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "委外加工厂 {} 不存在",
                req.supplier_id
            )));
        }

        // 校验生产订单存在（若提供）
        if let Some(order_id) = req.production_order_id {
            if crate::models::production_order::Entity::find_by_id(order_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("生产订单 {} 不存在", order_id)));
            }
        }

        // 校验缸号存在（若提供）
        if let Some(dye_batch_id) = req.dye_batch_id {
            if crate::models::dye_batch::Entity::find_by_id(dye_batch_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("缸号 {} 不存在", dye_batch_id)));
            }
        }

        // 校验订单号唯一性
        if let Some(_existing) = OrderEntity::find()
            .filter(outsourcing_order::Column::OrderNo.eq(&req.order_no))
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "委外订单号 {} 已存在",
                req.order_no
            )));
        }

        // 标准损耗率：未提供时按工序自动计算
        let standard_loss_rate = req
            .standard_loss_rate
            .unwrap_or_else(|| compute_standard_loss_rate(&req.order_type));

        let now = crate::utils::date_utils::utc_now_fixed();
        let issue_unit = req.issue_unit.unwrap_or_else(|| "kg".to_string());

        let active = OrderActiveModel {
            id: Default::default(),
            order_no: Set(req.order_no),
            order_type: Set(req.order_type),
            supplier_id: Set(req.supplier_id),
            production_order_id: Set(req.production_order_id),
            dye_batch_id: Set(req.dye_batch_id),
            color_no: Set(req.color_no),
            dye_lot_no: Set(req.dye_lot_no),
            issue_date: Set(req.issue_date),
            expected_return_date: Set(req.expected_return_date),
            actual_return_date: Set(None),
            issue_quantity: Set(req.issue_quantity),
            issue_unit: Set(issue_unit),
            return_quantity: Set(Decimal::ZERO),
            loss_quantity: Set(Decimal::ZERO),
            loss_type: Set(None),
            loss_rate: Set(None),
            standard_loss_rate: Set(Some(standard_loss_rate)),
            material_cost: Set(req.material_cost),
            processing_fee: Set(Decimal::ZERO),
            freight_fee: Set(Decimal::ZERO),
            tax_amount: Set(Decimal::ZERO),
            abnormal_loss_amount: Set(Decimal::ZERO),
            total_cost: Set(req.material_cost),
            unit_cost: Set(Decimal::ZERO),
            status: Set(outsourcing_order_status::DRAFT.to_string()),
            voucher_no_issue: Set(None),
            voucher_no_fee: Set(None),
            voucher_no_receipt: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外订单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新委外订单（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOutsourcingOrderRequest,
    ) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: OrderActiveModel = model.into();

        if let Some(v) = req.order_type {
            validate_order_type(&v)?;
            active.order_type = Set(v);
        }
        if let Some(v) = req.supplier_id {
            // 校验委外加工厂存在
            if crate::models::supplier::Entity::find_by_id(v)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("委外加工厂 {} 不存在", v)));
            }
            active.supplier_id = Set(v);
        }
        if let Some(v) = req.production_order_id {
            active.production_order_id = Set(Some(v));
        }
        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.issue_date {
            active.issue_date = Set(v);
        }
        if let Some(v) = req.expected_return_date {
            active.expected_return_date = Set(Some(v));
        }
        if let Some(v) = req.issue_quantity {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出数量不能为负"));
            }
            active.issue_quantity = Set(v);
        }
        if let Some(v) = req.issue_unit {
            active.issue_unit = Set(v);
        }
        if let Some(v) = req.material_cost {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出材料成本不能为负"));
            }
            active.material_cost = Set(v);
            // 重新计算总成本（无加工费/运费/非正常损耗阶段）
            active.total_cost = Set(v);
        }
        if let Some(v) = req.standard_loss_rate {
            active.standard_loss_rate = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除委外订单（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: OrderActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 发料：draft → issued，创建发料凭证（借：委托加工物资 / 贷：自制半成品-胚布）
    pub async fn issue_order(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可发料，当前状态: {}",
                model.status
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        // 生成发料凭证号
        let voucher_no = Self::generate_voucher_no("IS");

        // 创建发料凭证（§5.4 第一步分录）
        let voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(voucher_no.clone()),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::ISSUE.to_string()),
            debit_account: Set("委托加工物资".to_string()),
            credit_account: Set("自制半成品-胚布".to_string()),
            amount: Set(model.material_cost),
            tax_amount: Set(Decimal::ZERO),
            voucher_date: Set(model.issue_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 发料", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("发料凭证创建失败: {}", e)))?;

        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::ISSUED.to_string());
        active.voucher_no_issue = Set(Some(voucher_no));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 标记加工中：issued → processing
    pub async fn record_processing(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::ISSUED {
            return Err(AppError::business(format!(
                "仅已发料(issued)状态可标记加工中，当前状态: {}",
                model.status
            )));
        }
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::PROCESSING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 收回：创建收回单 + 计算损耗 + 创建入库凭证，processing → received
    pub async fn record_receipt(
        &self,
        id: i32,
        req: CreateOutsourcingReceiptRequest,
    ) -> Result<ReceiptModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_receipt_eligibility(&model, req.return_quantity)?;
        let calc = Self::compute_receipt_calculation(&model, req.return_quantity);
        let now = crate::utils::date_utils::utc_now_fixed();

        let receipt = self
            .insert_receipt_record(id, &model, &req, &calc, now)
            .await?;
        let voucher_no = self
            .insert_receipt_voucher(id, &model, &req, &calc, now)
            .await?;
        self.insert_loss_voucher_if_needed(id, &model, &req, &calc, now)
            .await?;
        self.apply_order_receipt(model, &req, &calc, &voucher_no, now)
            .await?;

        Ok(receipt)
    }

    /// 校验收回前置条件：订单状态与收回数量
    fn validate_receipt_eligibility(
        model: &OrderModel,
        return_quantity: Decimal,
    ) -> Result<(), AppError> {
        if model.status != outsourcing_order_status::PROCESSING
            && model.status != outsourcing_order_status::ISSUED
        {
            return Err(AppError::business(format!(
                "仅已发料(issued)或加工中(processing)状态可收回，当前状态: {}",
                model.status
            )));
        }
        let loss_quantity = model.issue_quantity - return_quantity;
        if loss_quantity < Decimal::ZERO {
            return Err(AppError::business(format!(
                "收回数量 {} 不能大于发出数量 {}",
                return_quantity, model.issue_quantity
            )));
        }
        Ok(())
    }

    /// 计算收回损耗与成本指标
    fn compute_receipt_calculation(
        model: &OrderModel,
        return_quantity: Decimal,
    ) -> ReceiptCalculation {
        let loss_quantity = model.issue_quantity - return_quantity;
        let actual_loss_rate = compute_loss_rate(loss_quantity, model.issue_quantity);
        let standard_loss_rate = model.standard_loss_rate.unwrap_or(Decimal::ZERO);
        let loss_type_str = classify_loss(actual_loss_rate, standard_loss_rate);
        let is_loss_normal = loss_type_str == outsourcing_loss_type::NORMAL;
        let unit_material_cost = if model.issue_quantity > Decimal::ZERO {
            model.material_cost / model.issue_quantity
        } else {
            Decimal::ZERO
        };
        let abnormal_loss_amount = compute_abnormal_loss_amount(
            model.issue_quantity,
            return_quantity,
            unit_material_cost,
            standard_loss_rate,
        );
        let total_cost = compute_total_cost(
            model.material_cost,
            model.processing_fee,
            model.freight_fee,
            abnormal_loss_amount,
        );
        let unit_cost = compute_unit_cost(total_cost, return_quantity);
        ReceiptCalculation {
            loss_quantity,
            actual_loss_rate,
            loss_type_str,
            is_loss_normal,
            abnormal_loss_amount,
            total_cost,
            unit_cost,
        }
    }

    /// 创建收回入库单
    async fn insert_receipt_record(
        &self,
        id: i32,
        model: &OrderModel,
        req: &CreateOutsourcingReceiptRequest,
        calc: &ReceiptCalculation,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<ReceiptModel, AppError> {
        let receipt_active = ReceiptActiveModel {
            id: Default::default(),
            receipt_no: Set(req.receipt_no.clone()),
            outsourcing_order_id: Set(id),
            receipt_date: Set(req.receipt_date),
            product_id: Set(req.product_id),
            color_no: Set(req.color_no.clone()),
            dye_lot_no: Set(req.dye_lot_no.clone()),
            batch_no: Set(req.batch_no.clone()),
            warehouse_id: Set(req.warehouse_id),
            return_quantity: Set(req.return_quantity),
            loss_quantity: Set(calc.loss_quantity),
            loss_type: Set(Some(calc.loss_type_str.to_string())),
            loss_rate: Set(Some(calc.actual_loss_rate)),
            is_loss_normal: Set(calc.is_loss_normal),
            unit_cost: Set(calc.unit_cost),
            total_cost: Set(calc.total_cost),
            abnormal_loss_amount: Set(calc.abnormal_loss_amount),
            quality_status: Set(req.quality_status.clone()),
            grade: Set(req.grade.clone()),
            inventory_transaction_id: Set(None),
            status: Set(outsourcing_receipt_status::CONFIRMED.to_string()),
            remarks: Set(req.remarks.clone()),
            is_deleted: Set(false),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        receipt_active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外收回单创建失败: {}", e)))
    }

    /// 创建入库凭证（§5.4 第三步分录：借 库存商品-成品布 / 贷 委托加工物资）
    async fn insert_receipt_voucher(
        &self,
        id: i32,
        model: &OrderModel,
        req: &CreateOutsourcingReceiptRequest,
        calc: &ReceiptCalculation,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<String, AppError> {
        let voucher_no = Self::generate_voucher_no("RC");
        let voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(voucher_no.clone()),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::RECEIPT.to_string()),
            debit_account: Set("库存商品-成品布".to_string()),
            credit_account: Set("委托加工物资".to_string()),
            amount: Set(calc.total_cost),
            tax_amount: Set(Decimal::ZERO),
            voucher_date: Set(req.receipt_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 入库", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("入库凭证创建失败: {}", e)))?;
        Ok(voucher_no)
    }

    /// 若存在非正常损耗，创建损耗处理凭证（借 营业外支出 / 贷 委托加工物资）
    async fn insert_loss_voucher_if_needed(
        &self,
        id: i32,
        model: &OrderModel,
        req: &CreateOutsourcingReceiptRequest,
        calc: &ReceiptCalculation,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<(), AppError> {
        if calc.abnormal_loss_amount <= Decimal::ZERO {
            return Ok(());
        }
        let loss_voucher_no = Self::generate_voucher_no("LS");
        let loss_voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(loss_voucher_no),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::LOSS.to_string()),
            debit_account: Set("营业外支出".to_string()),
            credit_account: Set("委托加工物资".to_string()),
            amount: Set(calc.abnormal_loss_amount),
            tax_amount: Set(Decimal::ZERO),
            voucher_date: Set(req.receipt_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!(
                "委外订单 {} 非正常损耗处理",
                model.order_no
            ))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        loss_voucher_active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("损耗处理凭证创建失败: {}", e)))?;
        Ok(())
    }

    /// 更新订单：累计收回数量、损耗、状态、入库凭证号
    async fn apply_order_receipt(
        &self,
        model: OrderModel,
        req: &CreateOutsourcingReceiptRequest,
        calc: &ReceiptCalculation,
        voucher_no: &str,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<(), AppError> {
        let mut active: OrderActiveModel = model.into();
        active.return_quantity = Set(req.return_quantity);
        active.loss_quantity = Set(calc.loss_quantity);
        active.loss_type = Set(Some(calc.loss_type_str.to_string()));
        active.loss_rate = Set(Some(calc.actual_loss_rate));
        active.abnormal_loss_amount = Set(calc.abnormal_loss_amount);
        active.total_cost = Set(calc.total_cost);
        active.unit_cost = Set(calc.unit_cost);
        active.actual_return_date = Set(Some(req.receipt_date));
        active.voucher_no_receipt = Set(Some(voucher_no.to_string()));
        active.status = Set(outsourcing_order_status::RECEIVED.to_string());
        active.updated_at = Set(now);
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 结算：received → settled，创建加工费凭证（借：委托加工物资+应交税费 / 贷：银行存款）
    ///
    /// 业务规则：
    /// - 加工费/运费/税额需在订单更新时填入（processing_fee / freight_fee / tax_amount 字段）
    /// - 加工费凭证金额 = processing_fee + freight_fee
    /// - 税额单独记录在 tax_amount 字段
    pub async fn settle(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::RECEIVED {
            return Err(AppError::business(format!(
                "仅已收回(received)状态可结算，当前状态: {}",
                model.status
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let voucher_no = Self::generate_voucher_no("FE");

        // 创建加工费凭证（§5.4 第二步分录）
        let fee_amount = model.processing_fee + model.freight_fee;
        let voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(voucher_no.clone()),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::FEE.to_string()),
            debit_account: Set("委托加工物资".to_string()),
            credit_account: Set("银行存款".to_string()),
            amount: Set(fee_amount),
            tax_amount: Set(model.tax_amount),
            voucher_date: Set(now.date_naive()),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 加工费结算", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("加工费凭证创建失败: {}", e)))?;

        // 更新订单总成本与状态
        let total_cost = compute_total_cost(
            model.material_cost,
            model.processing_fee,
            model.freight_fee,
            model.abnormal_loss_amount,
        );
        let unit_cost = compute_unit_cost(total_cost, model.return_quantity);

        let mut active: OrderActiveModel = model.into();
        active.total_cost = Set(total_cost);
        active.unit_cost = Set(unit_cost);
        active.voucher_no_fee = Set(Some(voucher_no));
        active.status = Set(outsourcing_order_status::SETTLED.to_string());
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭：settled → closed
    pub async fn close_order(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::SETTLED {
            return Err(AppError::business(format!(
                "仅已结算(settled)状态可关闭，当前状态: {}",
                model.status
            )));
        }
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消：任意非 closed 状态 → cancelled
    pub async fn cancel(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == outsourcing_order_status::CLOSED {
            return Err(AppError::business("已关闭状态不可取消"));
        }
        if model.status == outsourcing_order_status::CANCELLED {
            return Err(AppError::business("已取消状态不可重复取消"));
        }
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<OrderModel, AppError> {
        OrderEntity::find_by_id(id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外订单 {} 不存在", id)))
    }

    /// 按订单号查询
    pub async fn get_by_no(&self, order_no: &str) -> Result<OrderModel, AppError> {
        OrderEntity::find()
            .filter(outsourcing_order::Column::OrderNo.eq(order_no))
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外订单号 {} 不存在", order_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: OutsourcingOrderQuery,
    ) -> Result<(Vec<OrderModel>, u64), AppError> {
        let mut q = OrderEntity::find()
            .filter(outsourcing_order::Column::IsDeleted.eq(false));
        if let Some(v) = query.order_type {
            q = q.filter(outsourcing_order::Column::OrderType.eq(v));
        }
        if let Some(v) = query.supplier_id {
            q = q.filter(outsourcing_order::Column::SupplierId.eq(v));
        }
        if let Some(v) = query.production_order_id {
            q = q.filter(outsourcing_order::Column::ProductionOrderId.eq(v));
        }
        if let Some(v) = query.dye_batch_id {
            q = q.filter(outsourcing_order::Column::DyeBatchId.eq(v));
        }
        if let Some(v) = query.dye_lot_no {
            q = q.filter(outsourcing_order::Column::DyeLotNo.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(outsourcing_order::Column::Status.eq(v));
        }
        if let Some(v) = query.issue_date_from {
            q = q.filter(outsourcing_order::Column::IssueDate.gte(v));
        }
        if let Some(v) = query.issue_date_to {
            q = q.filter(outsourcing_order::Column::IssueDate.lte(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(outsourcing_order::Column::OrderNo.contains(&kw))
                    .add(outsourcing_order::Column::ColorNo.contains(&kw))
                    .add(outsourcing_order::Column::DyeLotNo.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(outsourcing_order::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 生成凭证号：OV-{prefix}-YYYYMMDDHHMMSS-NNN
    fn generate_voucher_no(prefix: &str) -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("OV-{}-{}-{:03}", prefix, timestamp, random)
    }
}

// ============================================================================
// 委外加工发料明细 Service
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
}

/// 更新委外发料明细请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingOrderItemRequest {
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 委外加工发料明细 Service
pub struct OutsourcingOrderItemService {
    db: Arc<DatabaseConnection>,
}

impl OutsourcingOrderItemService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建委外发料明细
    pub async fn create(
        &self,
        req: CreateOutsourcingOrderItemRequest,
    ) -> Result<ItemModel, AppError> {
        // 校验数量非负
        if req.quantity < Decimal::ZERO {
            return Err(AppError::business("发出数量不能为负"));
        }
        if req.unit_cost < Decimal::ZERO {
            return Err(AppError::business("单位成本不能为负"));
        }

        // 校验委外订单存在
        if OrderEntity::find_by_id(req.outsourcing_order_id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "委外订单 {} 不存在",
                req.outsourcing_order_id
            )));
        }

        // 校验物料存在
        if crate::models::product::Entity::find_by_id(req.product_id)
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!("物料 {} 不存在", req.product_id)));
        }

        let total_cost = req.quantity * req.unit_cost;
        let now = crate::utils::date_utils::utc_now_fixed();
        let unit = req.unit.unwrap_or_else(|| "kg".to_string());

        let active = ItemActiveModel {
            id: Default::default(),
            outsourcing_order_id: Set(req.outsourcing_order_id),
            product_id: Set(req.product_id),
            color_no: Set(req.color_no),
            dye_lot_no: Set(req.dye_lot_no),
            batch_no: Set(req.batch_no),
            warehouse_id: Set(req.warehouse_id),
            quantity: Set(req.quantity),
            unit: Set(unit),
            unit_cost: Set(req.unit_cost),
            total_cost: Set(total_cost),
            inventory_transaction_id: Set(None),
            remarks: Set(req.remarks),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外发料明细创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新委外发料明细
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOutsourcingOrderItemRequest,
    ) -> Result<ItemModel, AppError> {
        let model = self.get_by_id(id).await?;

        // 在 model.into() 之前记录原值，避免 ActiveValue 取值复杂
        let original_quantity = model.quantity;
        let original_unit_cost = model.unit_cost;
        let mut new_quantity = original_quantity;
        let mut new_unit_cost = original_unit_cost;
        let mut need_recompute_cost = false;

        let mut active: ItemActiveModel = model.into();

        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.batch_no {
            active.batch_no = Set(Some(v));
        }
        if let Some(v) = req.warehouse_id {
            active.warehouse_id = Set(Some(v));
        }
        if let Some(v) = req.quantity {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出数量不能为负"));
            }
            new_quantity = v;
            need_recompute_cost = true;
        }
        if let Some(v) = req.unit {
            active.unit = Set(v);
        }
        if let Some(v) = req.unit_cost {
            if v < Decimal::ZERO {
                return Err(AppError::business("单位成本不能为负"));
            }
            new_unit_cost = v;
            need_recompute_cost = true;
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        if need_recompute_cost {
            active.total_cost = Set(new_quantity * new_unit_cost);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除委外发料明细（物理删除，明细无软删除字段）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        ItemEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外发料明细删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ItemModel, AppError> {
        ItemEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外发料明细 {} 不存在", id)))
    }

    /// 按委外订单查询明细列表
    pub async fn list_by_order(&self, order_id: i32) -> Result<Vec<ItemModel>, AppError> {
        let items = ItemEntity::find()
            .filter(outsourcing_order_item::Column::OutsourcingOrderId.eq(order_id))
            .order_by_desc(outsourcing_order_item::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

// ============================================================================
// 委外收回入库单 Service
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
}

/// 更新委外收回入库单请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingReceiptRequest {
    pub receipt_date: Option<chrono::NaiveDate>,
    pub product_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub return_quantity: Option<Decimal>,
    pub loss_quantity: Option<Decimal>,
    pub quality_status: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
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

/// 委外收回入库单 Service
pub struct OutsourcingReceiptService {
    db: Arc<DatabaseConnection>,
}

impl OutsourcingReceiptService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建委外收回入库单（draft 状态）
    pub async fn create(
        &self,
        req: CreateOutsourcingReceiptRequest,
    ) -> Result<ReceiptModel, AppError> {
        if req.return_quantity < Decimal::ZERO {
            return Err(AppError::business("收回数量不能为负"));
        }

        Self::validate_create_request(&*self.db, &req).await?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = Self::build_receipt_active_model(&req, now);

        active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外收回单创建失败: {}", e)))
    }

    /// 校验创建请求：委外订单存在 + 成品存在 + 收回单号唯一
    async fn validate_create_request(
        db: &DatabaseConnection,
        req: &CreateOutsourcingReceiptRequest,
    ) -> Result<(), AppError> {
        // 校验委外订单存在
        if OrderEntity::find_by_id(req.outsourcing_order_id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "委外订单 {} 不存在",
                req.outsourcing_order_id
            )));
        }

        // 校验成品存在
        if crate::models::product::Entity::find_by_id(req.product_id)
            .one(db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!("成品 {} 不存在", req.product_id)));
        }

        // 校验收回单号唯一性
        if let Some(_existing) = ReceiptEntity::find()
            .filter(outsourcing_receipt::Column::ReceiptNo.eq(&req.receipt_no))
            .filter(outsourcing_receipt::Column::IsDeleted.eq(false))
            .one(db)
            .await?
        {
            return Err(AppError::business(format!(
                "收回单号 {} 已存在",
                req.receipt_no
            )));
        }

        Ok(())
    }

    /// 构造委外收回入库单 ActiveModel（draft 状态）
    fn build_receipt_active_model(
        req: &CreateOutsourcingReceiptRequest,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> ReceiptActiveModel {
        ReceiptActiveModel {
            id: Default::default(),
            receipt_no: Set(req.receipt_no.clone()),
            outsourcing_order_id: Set(req.outsourcing_order_id),
            receipt_date: Set(req.receipt_date),
            product_id: Set(req.product_id),
            color_no: Set(req.color_no.clone()),
            dye_lot_no: Set(req.dye_lot_no.clone()),
            batch_no: Set(req.batch_no.clone()),
            warehouse_id: Set(req.warehouse_id),
            return_quantity: Set(req.return_quantity),
            loss_quantity: Set(req.loss_quantity.unwrap_or(Decimal::ZERO)),
            loss_type: Set(None),
            loss_rate: Set(None),
            is_loss_normal: Set(true),
            unit_cost: Set(Decimal::ZERO),
            total_cost: Set(Decimal::ZERO),
            abnormal_loss_amount: Set(Decimal::ZERO),
            quality_status: Set(req.quality_status.clone()),
            grade: Set(req.grade.clone()),
            inventory_transaction_id: Set(None),
            status: Set(outsourcing_receipt_status::DRAFT.to_string()),
            remarks: Set(req.remarks.clone()),
            is_deleted: Set(false),
            created_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 更新委外收回入库单（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOutsourcingReceiptRequest,
    ) -> Result<ReceiptModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_receipt_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: ReceiptActiveModel = model.into();

        if let Some(v) = req.receipt_date {
            active.receipt_date = Set(v);
        }
        if let Some(v) = req.product_id {
            active.product_id = Set(v);
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.batch_no {
            active.batch_no = Set(Some(v));
        }
        if let Some(v) = req.warehouse_id {
            active.warehouse_id = Set(Some(v));
        }
        if let Some(v) = req.return_quantity {
            if v < Decimal::ZERO {
                return Err(AppError::business("收回数量不能为负"));
            }
            active.return_quantity = Set(v);
        }
        if let Some(v) = req.loss_quantity {
            active.loss_quantity = Set(v);
        }
        if let Some(v) = req.quality_status {
            active.quality_status = Set(Some(v));
        }
        if let Some(v) = req.grade {
            active.grade = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除委外收回入库单（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_receipt_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: ReceiptActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 确认收回单：draft → confirmed，计算损耗分类和单位成本
    pub async fn confirm(&self, id: i32) -> Result<ReceiptModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_receipt_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可确认，当前状态: {}",
                model.status
            )));
        }

        // 查询关联委外订单，获取发出数量、标准损耗率、材料成本
        let order = OrderEntity::find_by_id(model.outsourcing_order_id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("委外订单 {} 不存在", model.outsourcing_order_id))
            })?;

        // 计算损耗
        let loss_quantity = order.issue_quantity - model.return_quantity;
        let actual_loss_rate = compute_loss_rate(loss_quantity, order.issue_quantity);
        let standard_loss_rate = order.standard_loss_rate.unwrap_or(Decimal::ZERO);
        let loss_type_str = classify_loss(actual_loss_rate, standard_loss_rate);
        let is_loss_normal = loss_type_str == outsourcing_loss_type::NORMAL;

        // 计算非正常损耗金额
        let unit_material_cost = if order.issue_quantity > Decimal::ZERO {
            order.material_cost / order.issue_quantity
        } else {
            Decimal::ZERO
        };
        let abnormal_loss_amount = compute_abnormal_loss_amount(
            order.issue_quantity,
            model.return_quantity,
            unit_material_cost,
            standard_loss_rate,
        );

        // 计算总成本与单位成本
        let total_cost = compute_total_cost(
            order.material_cost,
            order.processing_fee,
            order.freight_fee,
            abnormal_loss_amount,
        );
        let unit_cost = compute_unit_cost(total_cost, model.return_quantity);

        let mut active: ReceiptActiveModel = model.into();
        active.loss_quantity = Set(loss_quantity);
        active.loss_type = Set(Some(loss_type_str.to_string()));
        active.loss_rate = Set(Some(actual_loss_rate));
        active.is_loss_normal = Set(is_loss_normal);
        active.abnormal_loss_amount = Set(abnormal_loss_amount);
        active.total_cost = Set(total_cost);
        active.unit_cost = Set(unit_cost);
        active.status = Set(outsourcing_receipt_status::CONFIRMED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ReceiptModel, AppError> {
        ReceiptEntity::find_by_id(id)
            .filter(outsourcing_receipt::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外收回单 {} 不存在", id)))
    }

    /// 按收回单号查询
    pub async fn get_by_no(&self, receipt_no: &str) -> Result<ReceiptModel, AppError> {
        ReceiptEntity::find()
            .filter(outsourcing_receipt::Column::ReceiptNo.eq(receipt_no))
            .filter(outsourcing_receipt::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收回单号 {} 不存在", receipt_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: OutsourcingReceiptQuery,
    ) -> Result<(Vec<ReceiptModel>, u64), AppError> {
        let mut q = ReceiptEntity::find()
            .filter(outsourcing_receipt::Column::IsDeleted.eq(false));
        if let Some(v) = query.outsourcing_order_id {
            q = q.filter(outsourcing_receipt::Column::OutsourcingOrderId.eq(v));
        }
        if let Some(v) = query.product_id {
            q = q.filter(outsourcing_receipt::Column::ProductId.eq(v));
        }
        if let Some(v) = query.dye_lot_no {
            q = q.filter(outsourcing_receipt::Column::DyeLotNo.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(outsourcing_receipt::Column::Status.eq(v));
        }
        if let Some(v) = query.receipt_date_from {
            q = q.filter(outsourcing_receipt::Column::ReceiptDate.gte(v));
        }
        if let Some(v) = query.receipt_date_to {
            q = q.filter(outsourcing_receipt::Column::ReceiptDate.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(outsourcing_receipt::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

// ============================================================================
// 委外加工会计分录凭证 Service
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

/// 委外加工会计分录凭证 Service
pub struct OutsourcingVoucherService {
    db: Arc<DatabaseConnection>,
}

impl OutsourcingVoucherService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建委外凭证
    pub async fn create(
        &self,
        req: CreateOutsourcingVoucherRequest,
    ) -> Result<VoucherModel, AppError> {
        validate_voucher_type(&req.voucher_type)?;

        if req.amount < Decimal::ZERO {
            return Err(AppError::business("金额不能为负"));
        }

        // 校验委外订单存在
        if OrderEntity::find_by_id(req.outsourcing_order_id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "委外订单 {} 不存在",
                req.outsourcing_order_id
            )));
        }

        // 校验凭证号唯一性
        if let Some(_existing) = VoucherEntity::find()
            .filter(outsourcing_voucher::Column::VoucherNo.eq(&req.voucher_no))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "凭证号 {} 已存在",
                req.voucher_no
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(req.voucher_no),
            outsourcing_order_id: Set(req.outsourcing_order_id),
            voucher_type: Set(req.voucher_type),
            debit_account: Set(req.debit_account),
            credit_account: Set(req.credit_account),
            amount: Set(req.amount),
            tax_amount: Set(req.tax_amount.unwrap_or(Decimal::ZERO)),
            voucher_date: Set(req.voucher_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外凭证创建失败: {}", e)))?;
        Ok(result)
    }

    /// 删除委外凭证（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.is_posted {
            return Err(AppError::business("已过账凭证不可删除"));
        }
        VoucherEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外凭证删除失败: {}", e)))?;
        Ok(())
    }

    /// 过账：is_posted = true, posted_at = now
    pub async fn post(&self, id: i32) -> Result<VoucherModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.is_posted {
            return Err(AppError::business("凭证已过账，不可重复过账"));
        }
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: VoucherActiveModel = model.into();
        active.is_posted = Set(true);
        active.posted_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<VoucherModel, AppError> {
        VoucherEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外凭证 {} 不存在", id)))
    }

    /// 按凭证号查询
    pub async fn get_by_no(&self, voucher_no: &str) -> Result<VoucherModel, AppError> {
        VoucherEntity::find()
            .filter(outsourcing_voucher::Column::VoucherNo.eq(voucher_no))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证号 {} 不存在", voucher_no)))
    }

    /// 按委外订单查询凭证列表
    pub async fn list_by_order(&self, order_id: i32) -> Result<Vec<VoucherModel>, AppError> {
        let items = VoucherEntity::find()
            .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(order_id))
            .order_by_desc(outsourcing_voucher::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: OutsourcingVoucherQuery,
    ) -> Result<(Vec<VoucherModel>, u64), AppError> {
        let mut q = VoucherEntity::find();
        if let Some(v) = query.outsourcing_order_id {
            q = q.filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(v));
        }
        if let Some(v) = query.voucher_type {
            q = q.filter(outsourcing_voucher::Column::VoucherType.eq(v));
        }
        if let Some(v) = query.is_posted {
            q = q.filter(outsourcing_voucher::Column::IsPosted.eq(v));
        }
        if let Some(v) = query.voucher_date_from {
            q = q.filter(outsourcing_voucher::Column::VoucherDate.gte(v));
        }
        if let Some(v) = query.voucher_date_to {
            q = q.filter(outsourcing_voucher::Column::VoucherDate.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(outsourcing_voucher::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
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

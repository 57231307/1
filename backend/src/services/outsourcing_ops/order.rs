//! 委外加工订单 Service impl 子模块（outsourcing_ops/order）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` L291-963 迁移。
//! 包含 OutsourcingOrderService 的 17 个方法：
//! - create / update / delete（CRUD）
//! - issue_order / record_processing / record_receipt / settle / close_order / cancel（状态机）
//! - get_by_id / get_by_no / list（查询）
//! - validate_receipt_eligibility / compute_receipt_calculation（私有 helper）
//! - insert_receipt_record / insert_receipt_voucher / insert_loss_voucher_if_needed / apply_order_receipt（私有 helper）
//! - generate_voucher_no（私有 helper）
//!
//! 业务规则：
//! - 状态机：draft → issued → processing → received → settled → closed；任意非 closed/cancelled → cancelled
//! - 收回时计算损耗分类与单位成本（§5.4 三步分录）
//! - 凭证号格式：OV-{prefix}-YYYYMMDDHHMMSS-NNN

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::models::outsourcing_order::{
    self, ActiveModel as OrderActiveModel, Entity as OrderEntity, Model as OrderModel,
};
use crate::models::outsourcing_receipt::{
    ActiveModel as ReceiptActiveModel, Model as ReceiptModel,
};
use crate::models::outsourcing_voucher::ActiveModel as VoucherActiveModel;
use crate::models::status::outsourcing_loss_type;
use crate::models::status::outsourcing_order_status;
use crate::models::status::outsourcing_receipt_status;
use crate::models::status::outsourcing_voucher_type;
use crate::utils::error::AppError;

use crate::services::outsourcing_service::{
    classify_loss, compute_abnormal_loss_amount, compute_loss_rate, compute_standard_loss_rate,
    compute_total_cost, compute_unit_cost, validate_order_type, OutsourcingOrderService,
};
use crate::services::outsourcing_ops::types::{
    CreateOutsourcingOrderRequest, CreateOutsourcingReceiptRequest, OutsourcingOrderQuery,
    UpdateOutsourcingOrderRequest,
};

/// 收回损耗与成本计算结果（record_receipt 内部传递）
pub(super) struct ReceiptCalculation {
    pub(super) loss_quantity: Decimal,
    pub(super) actual_loss_rate: Decimal,
    pub(super) loss_type_str: &'static str,
    pub(super) is_loss_normal: bool,
    pub(super) abnormal_loss_amount: Decimal,
    pub(super) total_cost: Decimal,
    pub(super) unit_cost: Decimal,
}

impl OutsourcingOrderService {
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

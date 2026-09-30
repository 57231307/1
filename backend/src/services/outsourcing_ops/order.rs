//! 委外加工订单 Service impl 子模块（outsourcing_ops/order）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` L291-963 迁移。
//! 包含 OutsourcingOrderService 的 12 个方法：
//! - create / update / delete（CRUD）
//! - issue_order / record_processing / settle / close_order / cancel（状态机）
//! - get_by_id / get_by_no / list（查询）
//! - validate_receipt_eligibility / compute_receipt_calculation（共享 helper）
//!
//! 业务规则：
//! - 状态机：draft → issued → processing → received → settled → closed；任意非 closed/cancelled → cancelled
//! - 收回时计算损耗分类与单位成本（§5.4 三步分录）
//! - 凭证号统一由 DocumentNumberGenerator 生成：OV{类型段}{YYYYMMDD}{3位流水}

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};

use crate::models::outsourcing_order::{
    self, ActiveModel as OrderActiveModel, Entity as OrderEntity, Model as OrderModel,
};
use crate::models::outsourcing_voucher::{
    ActiveModel as VoucherActiveModel, Column as VoucherColumn, Entity as VoucherEntity,
};
use crate::models::status::outsourcing_loss_type;
use crate::models::status::outsourcing_order_status;
use crate::models::status::outsourcing_voucher_type;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

use crate::services::outsourcing_ops::receipt::ReceiptCalculation;
use crate::services::outsourcing_ops::types::{
    CreateOutsourcingOrderRequest, OutsourcingOrderQuery, UpdateOutsourcingOrderRequest,
};
use crate::services::outsourcing_service::{
    OutsourcingOrderService, classify_loss, compute_abnormal_loss_amount, compute_loss_rate,
    compute_standard_loss_rate, compute_total_cost, compute_unit_cost, validate_order_type,
};

/// 校验收回前置条件：订单状态与收回数量
pub(crate) fn validate_receipt_eligibility(
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
pub(crate) fn compute_receipt_calculation(
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

impl OutsourcingOrderService {
    /// 校验委外订单创建请求（类型/数量/加工厂/生产订单/缸号/订单号唯一性）
    async fn validate_create_request(
        &self,
        req: &CreateOutsourcingOrderRequest,
    ) -> Result<(), AppError> {
        validate_order_type(&req.order_type)?;
        if req.issue_quantity < Decimal::ZERO {
            return Err(AppError::business("发出数量不能为负"));
        }
        if req.material_cost < Decimal::ZERO {
            return Err(AppError::business("发出材料成本不能为负"));
        }
        self.validate_create_references(req).await?;
        self.validate_order_no_unique(&req.order_no).await
    }

    /// 校验关联引用存在性（加工厂/生产订单/缸号）
    async fn validate_create_references(
        &self,
        req: &CreateOutsourcingOrderRequest,
    ) -> Result<(), AppError> {
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
        if let Some(order_id) = req.production_order_id {
            if crate::models::production_order::Entity::find_by_id(order_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("生产订单 {} 不存在", order_id)));
            }
        }
        if let Some(dye_batch_id) = req.dye_batch_id {
            if crate::models::dye_batch::Entity::find_by_id(dye_batch_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("缸号 {} 不存在", dye_batch_id)));
            }
        }
        Ok(())
    }

    /// 校验委外订单号唯一性
    async fn validate_order_no_unique(&self, order_no: &str) -> Result<(), AppError> {
        if OrderEntity::find()
            .filter(outsourcing_order::Column::OrderNo.eq(order_no))
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_some()
        {
            return Err(AppError::business(format!(
                "委外订单号 {} 已存在",
                order_no
            )));
        }
        Ok(())
    }

    /// 构建委外订单 ActiveModel（含标准损耗率与单位默认值计算）
    fn build_order_active_model(
        req: CreateOutsourcingOrderRequest,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> OrderActiveModel {
        let standard_loss_rate = req
            .standard_loss_rate
            .unwrap_or_else(|| compute_standard_loss_rate(&req.order_type));
        let issue_unit = req.issue_unit.unwrap_or_else(|| "kg".to_string());
        OrderActiveModel {
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
        }
    }

    /// 创建委外订单（draft 状态）
    pub async fn create(&self, req: CreateOutsourcingOrderRequest) -> Result<OrderModel, AppError> {
        self.validate_create_request(&req).await?;
        let now = crate::utils::date_utils::utc_now_fixed();
        let active = Self::build_order_active_model(req, now);
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("委外订单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新委外订单（仅 draft 状态可更新）
    ///
    /// 三态写入（RFC 7386，对齐 department_service::update）：
    /// None=不 Set、Some(None)=Set(None) 置 NULL（仅 DB 可空列）、Some(Some(v))=Set(v) 覆盖；
    /// NOT NULL 列（order_type/supplier_id/issue_date/issue_quantity/issue_unit/material_cost，
    /// v15 outsourcing_order DDL）的显式 null 在任何 DB 访问前拒绝（外显不脱敏）。
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOutsourcingOrderRequest,
    ) -> Result<OrderModel, AppError> {
        if matches!(req.order_type, Some(None)) {
            return Err(AppError::business_displayable(
                "委外类型不能清空：该字段为必填项",
            ));
        }
        if matches!(req.supplier_id, Some(None)) {
            return Err(AppError::business_displayable(
                "委外加工厂不能清空：该字段为必填项",
            ));
        }
        if matches!(req.issue_date, Some(None)) {
            return Err(AppError::business_displayable(
                "发料日期不能清空：该字段为必填项",
            ));
        }
        if matches!(req.issue_quantity, Some(None)) {
            return Err(AppError::business_displayable(
                "发出数量不能清空：该字段为必填项",
            ));
        }
        if matches!(req.issue_unit, Some(None)) {
            return Err(AppError::business_displayable(
                "发出单位不能清空：该字段为必填项",
            ));
        }
        if matches!(req.material_cost, Some(None)) {
            return Err(AppError::business_displayable(
                "发出材料成本不能清空：该字段为必填项",
            ));
        }

        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: OrderActiveModel = model.into();

        // NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(v) = req.order_type.flatten() {
            validate_order_type(&v)?;
            active.order_type = Set(v);
        }
        if let Some(v) = req.supplier_id.flatten() {
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
        if let Some(v) = req.issue_date.flatten() {
            active.issue_date = Set(v);
        }
        if let Some(v) = req.issue_quantity.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出数量不能为负"));
            }
            active.issue_quantity = Set(v);
        }
        if let Some(v) = req.issue_unit.flatten() {
            active.issue_unit = Set(v);
        }
        // DB 可空列：Some(None)=Set(None) 清空、Some(Some(v))=Set(Some(v)) 覆盖
        if let Some(v) = req.production_order_id {
            active.production_order_id = Set(v);
        }
        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(v);
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(v);
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(v);
        }
        if let Some(v) = req.expected_return_date {
            active.expected_return_date = Set(v);
        }
        if let Some(v) = req.standard_loss_rate {
            active.standard_loss_rate = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(v);
        }
        // material_cost：NOT NULL 列——覆盖时联动重算总成本（无加工费/运费/非正常损耗阶段）
        if let Some(v) = req.material_cost.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出材料成本不能为负"));
            }
            active.material_cost = Set(v);
            active.total_cost = Set(v);
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
    ///
    /// V15 主线审计 P0 修复：原实现顺序执行 3 步（凭证创建 / 主单更新 / 事件发布），
    /// 任一步失败都会留下半成品数据。把凭证创建和主单更新放进同一数据库事务，
    /// 事件发布在事务 commit 后执行（事件发布失败不影响业务数据一致性）。
    pub async fn issue_order(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可发料，当前状态: {}",
                model.status
            )));
        }

        // 匹号领域二期：发料前校验明细匹号——染色/印花外发必须精确到生产匹，
        // 且匹存在、状态可用（防止虚构匹号/不可用匹外发导致回仓对账断裂）
        let item_svc =
            crate::services::outsourcing_service::OutsourcingOrderItemService::new(self.db.clone());
        let items = item_svc.list_by_order(id).await?;
        crate::services::piece_domain_service::validate_pieces_for_issue(&*self.db, &items).await?;

        let now = crate::utils::date_utils::utc_now_fixed();

        let txn = (*self.db).begin().await?;

        // 生成发料凭证号（统一生成器，事务内取号：`OVIS{YYYYMMDD}{3位流水}`）
        let voucher_no = DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            "OVIS",
            VoucherEntity,
            VoucherColumn::VoucherNo,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "委外发料凭证号生成失败");
            AppError::business_displayable("委外发料凭证号生成失败，请稍后重试")
        })?;

        // 阶段 1：创建发料凭证
        let voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(voucher_no.clone()),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::ISSUE.to_string()),
            debit_account: Set("委托加工物资".to_string()),
            credit_account: Set("自制半成品-胚布".to_string()),
            amount: Set(model.material_cost),
            tax_amount: Set(Decimal::ZERO),
            tax_transfer_amount: Set(Decimal::ZERO),
            voucher_date: Set(model.issue_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 发料", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active
            .insert(&txn)
            .await
            .map_err(|e| AppError::database(format!("发料凭证创建失败: {}", e)))?;

        // 阶段 2：更新订单主单
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::ISSUED.to_string());
        active.voucher_no_issue = Set(Some(voucher_no.clone()));
        active.updated_at = Set(now);
        let updated = active
            .update(&txn)
            .await
            .map_err(|e| AppError::database(format!("委外订单状态更新失败: {}", e)))?;

        txn.commit().await?;

        // 阶段 3：事件发布（事务外，业务数据已落库；事件失败由 EVENT_BUS 自行重试/降级）
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingMaterialIssued {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
                issue_quantity: updated.issue_quantity,
                voucher_no_issue: updated.voucher_no_issue.clone(),
            },
        );
        tracing::info!(order_id = updated.id, "委外发料事件已发布");

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

        // V15 Batch04-P1-5：发布委外加工中事件，供生产看板/进度追踪订阅
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingProcessingRecorded {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
            },
        );
        tracing::info!(order_id = updated.id, "委外加工中事件已发布");

        Ok(updated)
    }

    /// 结算：received → settled，创建加工费凭证（借：委托加工物资+应交税费 / 贷：银行存款）
    /// 业务规则：加工费/运费/税额需在订单更新时填入（processing_fee / freight_fee / tax_amount 字段）；加工费凭证金额 = processing_fee + freight_fee；税额单独记录在 tax_amount 字段
    ///
    /// V15 主线审计 P0 修复：原实现顺序执行 2 步（凭证创建 / 主单更新），
    /// 任一步失败都会留下半成品数据。把凭证创建和主单更新放进同一数据库事务，
    /// 事件发布在事务 commit 后执行。
    pub async fn settle(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::RECEIVED {
            return Err(AppError::business(format!(
                "仅已收回(received)状态可结算，当前状态: {}",
                model.status
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let txn = (*self.db).begin().await?;

        // 生成加工费凭证号（统一生成器，事务内取号：`OVFE{YYYYMMDD}{3位流水}`）
        let voucher_no = DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            "OVFE",
            VoucherEntity,
            VoucherColumn::VoucherNo,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "委外加工费凭证号生成失败");
            AppError::business_displayable("委外加工费凭证号生成失败，请稍后重试")
        })?;

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
            tax_transfer_amount: Set(Decimal::ZERO),
            voucher_date: Set(now.date_naive()),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 加工费结算", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active
            .insert(&txn)
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
        active.voucher_no_fee = Set(Some(voucher_no.clone()));
        active.status = Set(outsourcing_order_status::SETTLED.to_string());
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;

        txn.commit().await?;

        // V15 Batch04-P1-5：发布委外结算事件，供成本归集/应付账款订阅
        let normal_loss = (updated.loss_quantity - updated.abnormal_loss_amount).max(Decimal::ZERO);
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingOrderSettled {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
                processing_fee: updated.processing_fee,
                freight_fee: updated.freight_fee,
                normal_loss,
                abnormal_loss: updated.abnormal_loss_amount,
                total_cost: updated.total_cost,
                unit_cost: updated.unit_cost,
                voucher_no_fee: updated.voucher_no_fee.clone(),
            },
        );
        tracing::info!(order_id = updated.id, "委外结算事件已发布");

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

        // V15 Batch04-P1-5：发布委外完成事件，供库存入库/成本结转订阅
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingOrderCompleted {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
                return_quantity: updated.return_quantity,
                voucher_no_receipt: updated.voucher_no_receipt.clone(),
            },
        );
        tracing::info!(order_id = updated.id, "委外完成事件已发布");

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
        let mut q = OrderEntity::find().filter(outsourcing_order::Column::IsDeleted.eq(false));
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
}

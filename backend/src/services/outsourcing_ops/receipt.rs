//! 委外加工收回入库单 Service impl 子模块（outsourcing_ops/receipt）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` L1216-1511 迁移。
//! 包含 OutsourcingReceiptService 的 9 个方法：
//! - create / update / delete（CRUD）
//! - confirm（draft → confirmed，计算损耗分类与单位成本）
//! - get_by_id / get_by_no / list（查询）
//! - validate_create_request / build_receipt_active_model（私有 helper）
//!
//! 业务规则：
//! - 状态机：draft → confirmed（confirm 时计算损耗分类与单位成本）
//! - confirm 需查询关联委外订单，按 §5.4 计算损耗与成本
//! - 仅 draft 状态可更新/删除

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::models::outsourcing_order::{self, Entity as OrderEntity};
use crate::models::outsourcing_receipt::{
    self, ActiveModel as ReceiptActiveModel, Entity as ReceiptEntity, Model as ReceiptModel,
};
use crate::models::status::outsourcing_loss_type;
use crate::models::status::outsourcing_receipt_status;
use crate::utils::error::AppError;

use crate::services::outsourcing_ops::types::{
    CreateOutsourcingReceiptRequest, OutsourcingReceiptQuery, UpdateOutsourcingReceiptRequest,
};
use crate::services::outsourcing_service::{
    classify_loss, compute_abnormal_loss_amount, compute_loss_rate, compute_total_cost,
    compute_unit_cost, OutsourcingReceiptService,
};

impl OutsourcingReceiptService {
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
            return Err(AppError::business(format!(
                "成品 {} 不存在",
                req.product_id
            )));
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

        // 缺陷 2.2 修复：委外收回确认后自动触发质检记录创建并回写 inspection_id
        let inspection_id = Self::trigger_quality_inspection(&*self.db, &updated, &order).await?;

        // 缺陷 2.2：将质检记录 ID 持久化到收回单，建立委外收回→质检的关联链路
        if let Some(insp_id) = inspection_id {
            let mut patch: ReceiptActiveModel = <ReceiptActiveModel as Default>::default();
            patch.id = sea_orm::ActiveValue::Set(updated.id);
            patch.inspection_id = sea_orm::ActiveValue::Set(Some(insp_id));
            patch.updated_at =
                sea_orm::ActiveValue::Set(crate::utils::date_utils::utc_now_fixed());
            let reloaded = patch.update(&*self.db).await?;
            return Ok(reloaded);
        }

        Ok(updated)
    }

    /// 缺陷 2.2：委外收回确认后自动创建质检记录，返回质检记录 ID
    async fn trigger_quality_inspection(
        db: &DatabaseConnection,
        receipt: &ReceiptModel,
        order: &crate::models::outsourcing_order::Model,
    ) -> Result<Option<i32>, AppError> {
        use crate::services::quality_inspection_service::{
            CreateInspectionRecordRequest, QualityInspectionService,
        };

        let inspection_no = format!("QI-OS-{}-{}", receipt.outsourcing_order_id, receipt.id);
        let inspection_date = chrono::Utc::now().date_naive();
        let grade = receipt.grade.clone().unwrap_or_else(|| "B".to_string());
        let quality_status = receipt
            .quality_status
            .clone()
            .unwrap_or_else(|| "qualified".to_string());
        let inspection_result = if quality_status == "qualified" {
            "合格".to_string()
        } else {
            "不合格".to_string()
        };
        let qualified_qty = if quality_status == "qualified" {
            receipt.return_quantity
        } else {
            Decimal::ZERO
        };
        let unqualified_qty = if quality_status != "qualified" {
            receipt.return_quantity
        } else {
            Decimal::ZERO
        };
        let qualification_rate = if receipt.return_quantity > Decimal::ZERO {
            Some((qualified_qty / receipt.return_quantity) * Decimal::from(100))
        } else {
            None
        };

        let req = CreateInspectionRecordRequest {
            inspection_no,
            inspection_type: "outsourcing_receipt".to_string(),
            related_type: Some("outsourcing_receipt".to_string()),
            related_id: Some(receipt.id),
            product_id: receipt.product_id,
            batch_no: receipt.batch_no.clone(),
            supplier_id: Some(order.supplier_id),
            customer_id: None,
            inspection_date,
            inspector_id: receipt.created_by,
            total_qty: receipt.return_quantity,
            inspected_qty: receipt.return_quantity,
            qualified_qty: Some(qualified_qty),
            unqualified_qty: Some(unqualified_qty),
            qualification_rate,
            inspection_result,
            remark: Some(format!("委外收回单 {} 自动触发质检", receipt.receipt_no)),
            grade: Some(grade),
            color_no: receipt.color_no.clone(),
            dye_lot_no: receipt.dye_lot_no.clone(),
        };

        let svc = QualityInspectionService::new(std::sync::Arc::new(db.clone()));
        let record = svc
            .create_record(req, receipt.created_by.unwrap_or(0))
            .await?;

        // 不合格时触发不合格品处理流程
        if quality_status != "qualified" {
            tracing::warn!(
                receipt_id = receipt.id,
                inspection_id = record.id,
                "委外收回质检不合格，触发不合格品处理流程"
            );
            // process_unqualified 需要 handling_method 参数，由后续业务流程决定
            // 这里仅记录告警，实际处理由质检员在质检界面操作
        }

        Ok(Some(record.id))
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
        let mut q = ReceiptEntity::find().filter(outsourcing_receipt::Column::IsDeleted.eq(false));
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

//! 染化料领用单 Service impl 子模块（chemical_ops/requisition）
//!
//! 批次 490 D10-3a 拆分：从原 `chemical_service.rs` L1312-1595 迁移。
//! 包含 ChemicalRequisitionService 的 11 个方法：
//! - generate_requisition_no（私有 helper，生成领用单号 CR-YYYYMMDDHHMMSS-NNN）
//! - create / update / delete（CRUD）
//! - approve / issue / close / cancel（状态机）
//! - get_by_id / get_by_no / list（查询）
//!
//! 业务规则：
//! - 创建时校验类型合法、生产领用必须关联染色缸号、缸号/生产订单存在性、总金额非负
//! - 领用单号格式：CR-YYYYMMDDHHMMSS-NNN
//! - 状态机：draft → approved → issued → partial_returned → closed；任意非 closed/cancelled → cancelled
//! - 更新/删除仅 draft 状态可操作
//! - 审批：draft → approved；发料：approved → issued；关闭：issued/partial_returned → closed
//! - 取消：closed/cancelled 状态不可取消
//! - 软删除（is_deleted = true）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::chemical_requisition::{
    self, ActiveModel as RequisitionActiveModel, Entity as RequisitionEntity,
    Model as RequisitionModel,
};
use crate::models::status::chemical_requisition_status;
use crate::models::status::chemical_requisition_type;
use crate::utils::error::AppError;

use crate::services::chemical_ops::types::{
    ChemicalRequisitionQuery, CreateChemicalRequisitionRequest, UpdateChemicalRequisitionRequest,
};
use crate::services::chemical_service::{validate_requisition_type, ChemicalRequisitionService};

impl ChemicalRequisitionService {
    /// 生成领用单号：CR-YYYYMMDDHHMMSS-NNN
    fn generate_requisition_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("CR-{}-{:03}", timestamp, random)
    }

    /// 创建染化料领用单
    pub async fn create(
        &self,
        req: CreateChemicalRequisitionRequest,
    ) -> Result<RequisitionModel, AppError> {
        validate_requisition_type(&req.requisition_type)?;

        // 校验生产领用必须关联染色缸号
        if req.requisition_type == chemical_requisition_type::PRODUCTION
            && req.dye_batch_id.is_none()
        {
            return Err(AppError::business("生产领用必须关联染色缸号"));
        }

        // 校验染色缸号存在（若提供）
        if let Some(dye_batch_id) = req.dye_batch_id {
            if crate::models::dye_batch::Entity::find_by_id(dye_batch_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!(
                    "染色缸号 {} 不存在",
                    dye_batch_id
                )));
            }
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

        let total_amount = req.total_amount.unwrap_or(Decimal::ZERO);
        if total_amount < Decimal::ZERO {
            return Err(AppError::business("总金额不能为负"));
        }

        let requisition_no = Self::generate_requisition_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RequisitionActiveModel {
            id: Default::default(),
            requisition_no: Set(requisition_no),
            requisition_type: Set(req.requisition_type),
            department_id: Set(req.department_id),
            requisition_date: Set(req.requisition_date),
            required_date: Set(req.required_date),
            dye_batch_id: Set(req.dye_batch_id),
            production_order_id: Set(req.production_order_id),
            status: Set(chemical_requisition_status::DRAFT.to_string()),
            total_amount: Set(total_amount),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            approved_by: Set(None),
            issued_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料领用单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新染化料领用单（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalRequisitionRequest,
    ) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: RequisitionActiveModel = model.into();

        if let Some(v) = req.department_id {
            active.department_id = Set(Some(v));
        }
        if let Some(v) = req.required_date {
            active.required_date = Set(Some(v));
        }
        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = req.production_order_id {
            active.production_order_id = Set(Some(v));
        }
        if let Some(v) = req.total_amount {
            if v < Decimal::ZERO {
                return Err(AppError::business("总金额不能为负"));
            }
            active.total_amount = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除染化料领用单（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 审批领用单（draft → approved）
    pub async fn approve(
        &self,
        id: i32,
        approved_by: Option<i32>,
    ) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可审批，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::APPROVED.to_string());
        active.approved_by = Set(approved_by);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 发料（approved → issued）
    pub async fn issue(
        &self,
        id: i32,
        issued_by: Option<i32>,
    ) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::APPROVED {
            return Err(AppError::business(format!(
                "仅已审批(approved)状态可发料，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::ISSUED.to_string());
        active.issued_by = Set(issued_by);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭领用单（issued/partial_returned → closed）
    pub async fn close(&self, id: i32) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != chemical_requisition_status::ISSUED
            && model.status != chemical_requisition_status::PARTIAL_RETURNED
        {
            return Err(AppError::business(format!(
                "仅已发料(issued)或部分退回(partial_returned)状态可关闭，当前状态: {}",
                model.status
            )));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消领用单（任意非 closed/cancelled 状态 → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<RequisitionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == chemical_requisition_status::CLOSED {
            return Err(AppError::business("已关闭状态不可取消"));
        }
        if model.status == chemical_requisition_status::CANCELLED {
            return Err(AppError::business("已取消状态不可重复取消"));
        }
        let mut active: RequisitionActiveModel = model.into();
        active.status = Set(chemical_requisition_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RequisitionModel, AppError> {
        RequisitionEntity::find_by_id(id)
            .filter(chemical_requisition::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染化料领用单 {} 不存在", id)))
    }

    /// 按单号查询
    pub async fn get_by_no(&self, requisition_no: &str) -> Result<RequisitionModel, AppError> {
        RequisitionEntity::find()
            .filter(chemical_requisition::Column::RequisitionNo.eq(requisition_no))
            .filter(chemical_requisition::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("领用单号 {} 不存在", requisition_no))
            })
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ChemicalRequisitionQuery,
    ) -> Result<(Vec<RequisitionModel>, u64), AppError> {
        let mut q = RequisitionEntity::find()
            .filter(chemical_requisition::Column::IsDeleted.eq(false));
        if let Some(v) = query.requisition_type {
            q = q.filter(chemical_requisition::Column::RequisitionType.eq(v));
        }
        if let Some(v) = query.department_id {
            q = q.filter(chemical_requisition::Column::DepartmentId.eq(v));
        }
        if let Some(v) = query.dye_batch_id {
            q = q.filter(chemical_requisition::Column::DyeBatchId.eq(v));
        }
        if let Some(v) = query.production_order_id {
            q = q.filter(chemical_requisition::Column::ProductionOrderId.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(chemical_requisition::Column::Status.eq(v));
        }
        if let Some(v) = query.requisition_date_start {
            q = q.filter(chemical_requisition::Column::RequisitionDate.gte(v));
        }
        if let Some(v) = query.requisition_date_end {
            q = q.filter(chemical_requisition::Column::RequisitionDate.lte(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(chemical_requisition::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

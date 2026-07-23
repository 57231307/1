//! 打样通知单 Service impl 子模块（lab_dip_ops/request）
//!
//! 批次 D10 拆分：从原 `lab_dip_service.rs` 迁移 LabDipRequestService 业务方法。
//! 包含 11 个公开方法：
//! - create / update / delete（CRUD）
//! - get_by_id / list（查询）
//! - start_sampling / submit_to_customer / approve_ok_sample / reject_and_redo
//!  / restart_sampling / complete（状态流转）
//!
//! 业务规则：
//! - 打样版数范围 1-10，对色光源必填，交期不能早于今天
//! - 状态机：pending → sampling → submitted → approved/rejected → completed
//!           rejected → sampling（重新打样）；approved → completed（复样通过后建库）
//! - 仅 pending/sampling 可更新，仅 pending 可删除
//! - 送客户确认前必须至少有 1 个小样；OK 样确认需选中属于该通知单的小样
//!
//! 纯函数（generate_request_no / validate_status_transition / validate_can_update
//! / validate_can_delete）与 struct 定义、new 构造函数保留在 facade `lab_dip_service`。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::lab_dip_request::{
    self, ActiveModel as RequestActiveModel, Entity as RequestEntity, Model as RequestModel,
};
use crate::models::lab_dip_sample::{self, ActiveModel as SampleActiveModel, Entity as SampleEntity};
use crate::models::status::lab_dip_request as req_status;
use crate::models::status::lab_dip_sample as sample_status;
use crate::utils::error::AppError;

use crate::services::lab_dip_ops::types::{
    CreateLabDipRequestRequest, LabDipRequestQuery, UpdateLabDipRequestRequest,
};
use crate::services::lab_dip_service::LabDipRequestService;

impl LabDipRequestService {
    /// 创建打样通知单
    pub async fn create(&self, req: CreateLabDipRequestRequest) -> Result<RequestModel, AppError> {
        // 业务校验：打样版数至少 1 版
        let sample_versions = req.sample_versions.unwrap_or(4);
        if sample_versions < 1 {
            return Err(AppError::business("打样版数至少 1 版"));
        }
        if sample_versions > 10 {
            return Err(AppError::business("打样版数最多 10 版"));
        }

        // 业务校验：对色光源必填（真实业务强制）
        if req.light_source.trim().is_empty() {
            return Err(AppError::business("对色光源必填（D65/TL84/U3000/CWF/A 等）"));
        }

        // 业务校验：交期不能早于今天
        let today = chrono::Utc::now().date_naive();
        if req.required_date < today {
            return Err(AppError::business("客户要求交期不能早于今天"));
        }

        let request_no = Self::generate_request_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RequestActiveModel {
            id: Default::default(),
            request_no: Set(request_no),
            customer_id: Set(req.customer_id),
            customer_color_no: Set(req.customer_color_no),
            customer_color_name: Set(req.customer_color_name),
            sample_type: Set(req.sample_type),
            fabric_spec: Set(req.fabric_spec),
            fabric_component: Set(req.fabric_component),
            sample_size: Set(req.sample_size),
            light_source: Set(req.light_source),
            secondary_light_source: Set(req.secondary_light_source),
            color_fastness_req: Set(req.color_fastness_req),
            eco_requirement: Set(req.eco_requirement),
            sample_versions: Set(sample_versions),
            dye_category: Set(req.dye_category),
            required_date: Set(req.required_date),
            expected_days: Set(req.expected_days),
            status: Set(req_status::PENDING.to_string()),
            customer_approved_at: Set(None),
            customer_approval_comment: Set(None),
            approved_sample_id: Set(None),
            production_recipe_id: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("打样通知单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新打样通知单（仅 pending/sampling 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateLabDipRequestRequest,
    ) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_update(&model.status)?;

        let mut active: RequestActiveModel = model.into();

        if let Some(v) = req.customer_id {
            active.customer_id = Set(Some(v));
        }
        if let Some(v) = req.customer_color_no {
            active.customer_color_no = Set(Some(v));
        }
        if let Some(v) = req.customer_color_name {
            active.customer_color_name = Set(Some(v));
        }
        if let Some(v) = req.sample_type {
            active.sample_type = Set(Some(v));
        }
        if let Some(v) = req.fabric_spec {
            active.fabric_spec = Set(Some(v));
        }
        if let Some(v) = req.fabric_component {
            active.fabric_component = Set(Some(v));
        }
        if let Some(v) = req.sample_size {
            active.sample_size = Set(Some(v));
        }
        if let Some(v) = req.light_source {
            if v.trim().is_empty() {
                return Err(AppError::business("对色光源不能为空"));
            }
            active.light_source = Set(v);
        }
        if let Some(v) = req.secondary_light_source {
            active.secondary_light_source = Set(Some(v));
        }
        if let Some(v) = req.color_fastness_req {
            active.color_fastness_req = Set(Some(v));
        }
        if let Some(v) = req.eco_requirement {
            active.eco_requirement = Set(Some(v));
        }
        if let Some(v) = req.sample_versions {
            if !(1..=10).contains(&v) {
                return Err(AppError::business("打样版数范围 1-10"));
            }
            active.sample_versions = Set(v);
        }
        if let Some(v) = req.dye_category {
            active.dye_category = Set(Some(v));
        }
        if let Some(v) = req.required_date {
            active.required_date = Set(v);
        }
        if let Some(v) = req.expected_days {
            active.expected_days = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除打样通知单（仅 pending 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_delete(&model.status)?;

        let mut active: RequestActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询打样通知单
    pub async fn get_by_id(&self, id: i32) -> Result<RequestModel, AppError> {
        let model = RequestEntity::find_by_id(id)
            .filter(lab_dip_request::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样通知单 {} 不存在", id)))?;
        Ok(model)
    }

    /// 分页查询打样通知单
    pub async fn list(
        &self,
        query: LabDipRequestQuery,
    ) -> Result<(Vec<RequestModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RequestEntity::find().filter(lab_dip_request::Column::IsDeleted.eq(false));

        if let Some(no) = &query.request_no {
            q = q.filter(lab_dip_request::Column::RequestNo.contains(no));
        }
        if let Some(cid) = query.customer_id {
            q = q.filter(lab_dip_request::Column::CustomerId.eq(cid));
        }
        if let Some(status) = &query.status {
            q = q.filter(lab_dip_request::Column::Status.eq(status));
        }

        q = q.order_by_desc(lab_dip_request::Column::CreatedAt);

        let paginator = q.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 状态流转：开始打样（pending → sampling）
    pub async fn start_sampling(&self, id: i32) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, req_status::SAMPLING)?;

        let mut active: RequestActiveModel = model.into();
        active.status = Set(req_status::SAMPLING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 状态流转：送客户确认（sampling → submitted）
    pub async fn submit_to_customer(&self, id: i32) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, req_status::SUBMITTED)?;

        // 业务校验：送客户确认前必须至少有 1 个小样
        let sample_count = SampleEntity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await?;
        if sample_count == 0 {
            return Err(AppError::business("送客户确认前必须至少有 1 个小样"));
        }

        let mut active: RequestActiveModel = model.into();
        active.status = Set(req_status::SUBMITTED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 状态流转：客户确认 OK 样（submitted → approved）
    ///
    /// 真实业务：客户从 ABCD 多版中选 1 版作为 OK 样
    pub async fn approve_ok_sample(
        &self,
        id: i32,
        sample_id: i32,
        comment: Option<String>,
    ) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, req_status::APPROVED)?;

        // 校验选中的小样存在且属于该通知单
        let sample = SampleEntity::find_by_id(sample_id)
            .filter(lab_dip_sample::Column::RequestId.eq(id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::business(format!("小样 {} 不存在或不属于该通知单", sample_id)))?;

        // 事务：更新通知单状态 + 更新选中样状态 + 更新其他样状态
        let now = crate::utils::date_utils::utc_now_fixed();

        // 1. 更新通知单
        let mut req_active: RequestActiveModel = model.into();
        req_active.status = Set(req_status::APPROVED.to_string());
        req_active.customer_approved_at = Set(Some(now));
        req_active.customer_approval_comment = Set(comment);
        req_active.approved_sample_id = Set(Some(sample_id));
        req_active.updated_at = Set(now);
        let updated_req = req_active.update(&*self.db).await?;

        // 2. 更新选中样状态为 selected
        let mut sample_active: SampleActiveModel = sample.into();
        sample_active.matching_result = Set(sample_status::SELECTED.to_string());
        sample_active.approved_at = Set(Some(now));
        sample_active.updated_at = Set(now);
        sample_active.update(&*self.db).await?;

        Ok(updated_req)
    }

    /// 状态流转：客户要求重打（submitted → rejected）
    pub async fn reject_and_redo(&self, id: i32, comment: String) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, req_status::REJECTED)?;

        let mut active: RequestActiveModel = model.into();
        active.status = Set(req_status::REJECTED.to_string());
        active.customer_approval_comment = Set(Some(comment));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 状态流转：rejected → sampling（重新打样）
    pub async fn restart_sampling(&self, id: i32) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, req_status::SAMPLING)?;

        let mut active: RequestActiveModel = model.into();
        active.status = Set(req_status::SAMPLING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 状态流转：完成建库（approved → completed，复样通过后调用）
    pub async fn complete(&self, id: i32, production_recipe_id: i32) -> Result<RequestModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, req_status::COMPLETED)?;

        let mut active: RequestActiveModel = model.into();
        active.status = Set(req_status::COMPLETED.to_string());
        active.production_recipe_id = Set(Some(production_recipe_id));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

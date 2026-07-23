//! 打样小样 Service impl 子模块（lab_dip_ops/sample）
//!
//! 批次 D10 拆分：从原 `lab_dip_service.rs` 迁移 LabDipSampleService 业务方法。
//! 包含 6 个公开方法 + 3 个私有 helper：
//! - create / update / delete（CRUD）
//! - record_matching_result（对色结果记录）
//! - get_by_id / list_by_request（查询）
//! - validate_and_get_request / compute_version_seq_and_label / build_sample_active_model（私有 helper）
//!
//! 业务规则：
//! - 添加小样需通知单处于 sampling 状态
//! - 版本序号自动递增，校验版数上限与版本标识唯一性（ABCD 多版样）
//! - 对色结果：色差 4-5 级为 matched（OK），<4 级为 not_matched（重打）
//! - 仅 pending 对色状态可更新 / 删除
//!
//! 纯函数 label_from_seq 与 struct 定义、new 构造函数保留在 facade `lab_dip_service`。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::lab_dip_request::{self, Entity as RequestEntity, Model as RequestModel};
use crate::models::lab_dip_sample::{
    self, ActiveModel as SampleActiveModel, Entity as SampleEntity, Model as SampleModel,
};
use crate::models::status::lab_dip_request as req_status;
use crate::models::status::lab_dip_sample as sample_status;
use crate::utils::error::AppError;

use crate::services::lab_dip_ops::types::{
    CreateLabDipSampleRequest, RecordMatchingResultRequest, UpdateLabDipSampleRequest,
};
use crate::services::lab_dip_service::{LabDipSampleService, COLOR_DIFF_OK_GRADE};

impl LabDipSampleService {
    /// 创建打样小样
    pub async fn create(&self, req: CreateLabDipSampleRequest) -> Result<SampleModel, AppError> {
        // 校验通知单存在且处于 sampling 状态
        let request = self.validate_and_get_request(req.request_id).await?;

        // 计算版本序号并校验标识唯一
        let (version_seq, version_label) = self
            .compute_version_seq_and_label(
                req.request_id,
                request.sample_versions,
                req.version_label.clone(),
            )
            .await?;

        // 构建小样 ActiveModel 并入库
        let active = Self::build_sample_active_model(&req, version_label, version_seq);
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("打样小样创建失败: {}", e)))?;
        Ok(result)
    }

    /// 校验通知单存在且处于 sampling 状态
    async fn validate_and_get_request(
        &self,
        request_id: i32,
    ) -> Result<RequestModel, AppError> {
        let request = RequestEntity::find_by_id(request_id)
            .filter(lab_dip_request::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样通知单 {} 不存在", request_id)))?;

        if request.status != req_status::SAMPLING {
            return Err(AppError::business(format!(
                "通知单状态 {} 不可添加小样（仅 sampling 状态可添加）",
                request.status
            )));
        }
        Ok(request)
    }

    /// 计算版本序号，校验版数上限与标识唯一性
    async fn compute_version_seq_and_label(
        &self,
        request_id: i32,
        request_sample_versions: i32,
        custom_label: Option<String>,
    ) -> Result<(i32, String), AppError> {
        // 统计已有小样数量
        let existing_count = SampleEntity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(request_id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await? as i32;

        let version_seq = existing_count + 1;

        // 业务校验：小样数量不能超过通知单规定的打样版数
        if version_seq > request_sample_versions {
            return Err(AppError::business(format!(
                "小样数量已达上限 {}（通知单规定打样版数）",
                request_sample_versions
            )));
        }

        let version_label = custom_label.unwrap_or_else(|| Self::label_from_seq(version_seq));

        // 业务校验：版本标识唯一（同一通知单下）
        let exists = SampleEntity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(request_id))
            .filter(lab_dip_sample::Column::VersionLabel.eq(&version_label))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await?;
        if exists > 0 {
            return Err(AppError::business(format!(
                "版本标识 {} 已存在（同一通知单下版本标识唯一）",
                version_label
            )));
        }
        Ok((version_seq, version_label))
    }

    /// 构建小样 ActiveModel（含全部字段）
    fn build_sample_active_model(
        req: &CreateLabDipSampleRequest,
        version_label: String,
        version_seq: i32,
    ) -> SampleActiveModel {
        let now = crate::utils::date_utils::utc_now_fixed();
        SampleActiveModel {
            id: Default::default(),
            request_id: Set(req.request_id),
            version_label: Set(version_label),
            version_seq: Set(version_seq),
            recipe_no: Set(req.recipe_no.clone()),
            dye_recipe_id: Set(req.dye_recipe_id),
            formula: Set(req.formula.clone()),
            formula_detail: Set(req.formula_detail.clone()),
            temperature: Set(req.temperature),
            time_minutes: Set(req.time_minutes),
            liquor_ratio: Set(req.liquor_ratio.clone()),
            ph_value: Set(req.ph_value),
            dyeing_method: Set(req.dyeing_method.clone()),
            dye_cost: Set(req.dye_cost),
            auxiliary_cost: Set(req.auxiliary_cost),
            total_cost: Set(req.total_cost),
            color_difference_grade: Set(req.color_difference_grade),
            color_difference_value: Set(req.color_difference_value),
            matching_result: Set(sample_status::PENDING.to_string()),
            approved_by: Set(None),
            approved_at: Set(None),
            approval_comment: Set(None),
            resample_status: Set(Some("none".to_string())),
            resample_recipe_id: Set(None),
            remarks: Set(req.remarks.clone()),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 更新打样小样（仅 pending 对色状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateLabDipSampleRequest,
    ) -> Result<SampleModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.matching_result != sample_status::PENDING {
            return Err(AppError::business(format!(
                "对色结果 {} 不可更新（仅 pending 状态可更新）",
                model.matching_result
            )));
        }

        let mut active: SampleActiveModel = model.into();

        if let Some(v) = req.recipe_no {
            active.recipe_no = Set(Some(v));
        }
        if let Some(v) = req.dye_recipe_id {
            active.dye_recipe_id = Set(Some(v));
        }
        if let Some(v) = req.formula {
            active.formula = Set(Some(v));
        }
        if let Some(v) = req.formula_detail {
            active.formula_detail = Set(Some(v));
        }
        if let Some(v) = req.temperature {
            active.temperature = Set(Some(v));
        }
        if let Some(v) = req.time_minutes {
            active.time_minutes = Set(Some(v));
        }
        if let Some(v) = req.liquor_ratio {
            active.liquor_ratio = Set(Some(v));
        }
        if let Some(v) = req.ph_value {
            active.ph_value = Set(Some(v));
        }
        if let Some(v) = req.dyeing_method {
            active.dyeing_method = Set(Some(v));
        }
        if let Some(v) = req.dye_cost {
            active.dye_cost = Set(Some(v));
        }
        if let Some(v) = req.auxiliary_cost {
            active.auxiliary_cost = Set(Some(v));
        }
        if let Some(v) = req.total_cost {
            active.total_cost = Set(Some(v));
        }
        if let Some(v) = req.color_difference_grade {
            active.color_difference_grade = Set(Some(v));
        }
        if let Some(v) = req.color_difference_value {
            active.color_difference_value = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 记录对色结果（真实业务：色差 4-5 级为 matched，<4 级为 not_matched）
    pub async fn record_matching_result(
        &self,
        id: i32,
        req: RecordMatchingResultRequest,
    ) -> Result<SampleModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.matching_result != sample_status::PENDING {
            return Err(AppError::business(format!(
                "对色结果已记录为 {}，不可重复记录",
                model.matching_result
            )));
        }

        // 业务校验：色差等级范围 1-5
        if !(1..=5).contains(&req.color_difference_grade) {
            return Err(AppError::business("色差等级范围 1-5 级"));
        }

        // 真实业务判定：4-5 级为 matched（OK），<4 级为 not_matched（重打）
        let result = if req.color_difference_grade >= COLOR_DIFF_OK_GRADE {
            sample_status::MATCHED
        } else {
            sample_status::NOT_MATCHED
        };

        let now = crate::utils::date_utils::utc_now_fixed();

        let mut active: SampleActiveModel = model.into();
        active.color_difference_grade = Set(Some(req.color_difference_grade));
        active.color_difference_value = Set(req.color_difference_value);
        active.matching_result = Set(result.to_string());
        active.approved_by = Set(req.approved_by);
        active.approved_at = Set(Some(now));
        active.approval_comment = Set(req.approval_comment);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询小样
    pub async fn get_by_id(&self, id: i32) -> Result<SampleModel, AppError> {
        let model = SampleEntity::find_by_id(id)
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样小样 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按通知单 ID 查询所有小样
    pub async fn list_by_request(&self, request_id: i32) -> Result<Vec<SampleModel>, AppError> {
        let samples = SampleEntity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(request_id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .order_by_asc(lab_dip_sample::Column::VersionSeq)
            .all(&*self.db)
            .await?;
        Ok(samples)
    }

    /// 软删除小样（仅 pending 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.matching_result != sample_status::PENDING {
            return Err(AppError::business(format!(
                "对色结果 {} 不可删除（仅 pending 状态可删除）",
                model.matching_result
            )));
        }

        let mut active: SampleActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }
}

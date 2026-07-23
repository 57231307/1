//! 复样记录 Service impl 子模块（lab_dip_ops/resample）
//!
//! 批次 D10 拆分：从原 `lab_dip_service.rs` 迁移 LabDipResampleService 业务方法。
//! 包含 5 个公开方法 + 5 个私有 helper：
//! - create / record_result / issue_tech_card（业务流转）
//! - get_by_id / list_by_request（查询）
//! - validate_resample_request / validate_resample_source_sample
//!  / validate_workshop_fabric_batch / build_resample_active_model
//!  / mark_source_sample_resampling（私有 helper）
//!
//! 业务规则：
//! - 复样需通知单处于 approved 状态，源样为 selected（OK 样）
//! - 车间半制品布批号必填（复样必须用车间半制品布，不可用化验室存布）
//! - 复样单号格式：RS-YYYYMMDDHHMMSS-NNN
//! - 复样结果：色差 4-5 级为 passed（可投产），<4 级为 failed（不可投产）
//! - 染色技术卡仅复样通过可开（研发组长开卡），不可重复开卡
//!
//! 纯函数 generate_resample_no 与 struct 定义、new 构造函数保留在 facade `lab_dip_service`。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::lab_dip_request::{self, Entity as RequestEntity};
use crate::models::lab_dip_resample::{
    self, ActiveModel as ResampleActiveModel, Entity as ResampleEntity, Model as ResampleModel,
};
use crate::models::lab_dip_sample::{
    self, ActiveModel as SampleActiveModel, Entity as SampleEntity, Model as SampleModel,
};
use crate::models::status::lab_dip_request as req_status;
use crate::models::status::lab_dip_resample as resample_status;
use crate::models::status::lab_dip_sample as sample_status;
use crate::utils::error::AppError;

use crate::services::lab_dip_ops::types::{
    CreateResampleRequest, IssueTechCardRequest, RecordResampleResultRequest,
};
use crate::services::lab_dip_service::{LabDipResampleService, COLOR_DIFF_OK_GRADE};

impl LabDipResampleService {
    /// 创建复样记录：OK 样确认后大货生产前必须复样
    pub async fn create(&self, req: CreateResampleRequest) -> Result<ResampleModel, AppError> {
        Self::validate_resample_request(&*self.db, req.request_id).await?;
        let source_sample = Self::validate_resample_source_sample(
            &*self.db,
            req.request_id,
            req.source_sample_id,
        )
        .await?;
        Self::validate_workshop_fabric_batch(&req.workshop_fabric_batch)?;
        let resample_no = Self::generate_resample_no();
        let now = crate::utils::date_utils::utc_now_fixed();
        let active = Self::build_resample_active_model(req, resample_no, now);
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("复样记录创建失败: {}", e)))?;
        Self::mark_source_sample_resampling(&*self.db, source_sample, now).await?;
        Ok(result)
    }

    /// 校验通知单存在且处于 approved 状态
    async fn validate_resample_request(
        db: &DatabaseConnection,
        request_id: i32,
    ) -> Result<(), AppError> {
        let request = RequestEntity::find_by_id(request_id)
            .filter(lab_dip_request::Column::IsDeleted.eq(false))
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样通知单 {} 不存在", request_id)))?;
        if request.status != req_status::APPROVED {
            return Err(AppError::business(format!(
                "通知单状态 {} 不可创建复样（仅 approved 状态可复样）",
                request.status
            )));
        }
        Ok(())
    }

    /// 校验源样存在且为 selected（OK 样）
    async fn validate_resample_source_sample(
        db: &DatabaseConnection,
        request_id: i32,
        source_sample_id: i32,
    ) -> Result<SampleModel, AppError> {
        let source_sample = SampleEntity::find_by_id(source_sample_id)
            .filter(lab_dip_sample::Column::RequestId.eq(request_id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .one(db)
            .await?
            .ok_or_else(|| {
                AppError::business(format!("OK 样 {} 不存在或不属于该通知单", source_sample_id))
            })?;
        if source_sample.matching_result != sample_status::SELECTED {
            return Err(AppError::business(format!(
                "源样对色结果 {} 不可复样（仅 selected OK 样可复样）",
                source_sample.matching_result
            )));
        }
        Ok(source_sample)
    }

    /// 校验车间半制品布批号必填
    fn validate_workshop_fabric_batch(batch: &Option<String>) -> Result<(), AppError> {
        if batch.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            return Err(AppError::business(
                "车间半制品布批号必填（复样必须用车间半制品布，不可用化验室存布）",
            ));
        }
        Ok(())
    }

    /// 构建复样 ActiveModel（含全部字段）
    fn build_resample_active_model(
        req: CreateResampleRequest,
        resample_no: String,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> ResampleActiveModel {
        ResampleActiveModel {
            id: Default::default(),
            request_id: Set(req.request_id),
            source_sample_id: Set(req.source_sample_id),
            resample_no: Set(resample_no),
            workshop_fabric_batch: Set(req.workshop_fabric_batch),
            dye_batch_no: Set(req.dye_batch_no),
            auxiliary_batch_no: Set(req.auxiliary_batch_no),
            production_plan_id: Set(req.production_plan_id),
            adjusted_formula: Set(req.adjusted_formula),
            adjustment_factor: Set(req.adjustment_factor),
            adjusted_temperature: Set(req.adjusted_temperature),
            adjusted_time_minutes: Set(req.adjusted_time_minutes),
            adjusted_liquor_ratio: Set(req.adjusted_liquor_ratio),
            color_difference_grade: Set(None),
            color_difference_value: Set(None),
            result: Set(resample_status::PENDING.to_string()),
            reviewed_by: Set(None),
            reviewed_at: Set(None),
            review_comment: Set(None),
            production_recipe_id: Set(None),
            tech_card_no: Set(None),
            tech_card_issued_by: Set(None),
            tech_card_issued_at: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 更新源样复样状态为 resampling
    async fn mark_source_sample_resampling(
        db: &DatabaseConnection,
        source_sample: SampleModel,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> Result<(), AppError> {
        let mut sample_active: SampleActiveModel = source_sample.into();
        sample_active.resample_status = Set(Some("resampling".to_string()));
        sample_active.updated_at = Set(now);
        sample_active.update(db).await?;
        Ok(())
    }

    /// 记录复样结果（真实业务：色差 4-5 级方可投产）
    pub async fn record_result(
        &self,
        id: i32,
        req: RecordResampleResultRequest,
    ) -> Result<ResampleModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.result != resample_status::PENDING {
            return Err(AppError::business(format!(
                "复样结果已记录为 {}，不可重复记录",
                model.result
            )));
        }

        if !(1..=5).contains(&req.color_difference_grade) {
            return Err(AppError::business("色差等级范围 1-5 级"));
        }

        // 真实业务判定：4-5 级为 passed（可投产），<4 级为 failed（不可投产）
        let result = if req.color_difference_grade >= COLOR_DIFF_OK_GRADE {
            resample_status::PASSED
        } else {
            resample_status::FAILED
        };

        let now = crate::utils::date_utils::utc_now_fixed();

        let mut active: ResampleActiveModel = model.into();
        active.color_difference_grade = Set(Some(req.color_difference_grade));
        active.color_difference_value = Set(req.color_difference_value);
        active.result = Set(result.to_string());
        active.reviewed_by = Set(req.reviewed_by);
        active.reviewed_at = Set(Some(now));
        active.review_comment = Set(req.review_comment);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;

        // 同步更新源样复样状态
        let source = SampleEntity::find_by_id(updated.source_sample_id)
            .one(&*self.db)
            .await?;
        if let Some(src) = source {
            let new_status = if updated.result == resample_status::PASSED {
                "resampled"
            } else {
                "failed"
            };
            let mut src_active: SampleActiveModel = src.into();
            src_active.resample_status = Set(Some(new_status.to_string()));
            src_active.updated_at = Set(now);
            src_active.update(&*self.db).await?;
        }

        Ok(updated)
    }

    /// 开具染色技术卡（真实业务：复样通过后由研发组长开卡）
    ///
    /// 染色技术卡附三件套：配方表 + 核可样 + 复色样
    pub async fn issue_tech_card(
        &self,
        id: i32,
        req: IssueTechCardRequest,
    ) -> Result<ResampleModel, AppError> {
        let model = self.get_by_id(id).await?;

        // 业务校验：仅复样通过可开技术卡
        if model.result != resample_status::PASSED {
            return Err(AppError::business(format!(
                "复样结果 {} 不可开技术卡（仅 passed 可开卡）",
                model.result
            )));
        }

        // 业务校验：未开过卡
        if model.tech_card_no.is_some() {
            return Err(AppError::business(format!(
                "技术卡 {} 已开具，不可重复开卡",
                model.tech_card_no.unwrap_or_default()
            )));
        }

        let tech_card_no = format!(
            "TC-{}-{:06}",
            chrono::Utc::now().format("%Y%m%d"),
            id
        );
        let now = crate::utils::date_utils::utc_now_fixed();

        let mut active: ResampleActiveModel = model.into();
        active.tech_card_no = Set(Some(tech_card_no));
        active.tech_card_issued_by = Set(Some(req.issued_by));
        active.tech_card_issued_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询复样记录
    pub async fn get_by_id(&self, id: i32) -> Result<ResampleModel, AppError> {
        let model = ResampleEntity::find_by_id(id)
            .filter(lab_dip_resample::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("复样记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按通知单 ID 查询所有复样记录
    pub async fn list_by_request(&self, request_id: i32) -> Result<Vec<ResampleModel>, AppError> {
        let items = ResampleEntity::find()
            .filter(lab_dip_resample::Column::RequestId.eq(request_id))
            .filter(lab_dip_resample::Column::IsDeleted.eq(false))
            .order_by_desc(lab_dip_resample::Column::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

//! 化验室打样 Service
//!
//! v14 批次 423B：化验室打样流程贯通
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环 + §11.1.1 染色技术卡
//! 真实业务流程：打样通知单 → 打样（ABCD 多版样）→ 色样确认（OK 样）→ 复样 → 建数据库
//!
//! 核心能力：
//! - 打样通知单 CRUD + 状态流转（pending → sampling → submitted → approved/rejected → completed）
//! - 打样小样 CRUD + ABCD 多版样管理 + 对色结果记录
//! - OK 样确认（客户从多版中选 1 版，状态 → selected）
//! - 复样记录 CRUD + 复样结果判定（passed/failed/adjusted）
//! - 染色技术卡开具（复样通过后由研发组长开卡）

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::lab_dip_request::{self, ActiveModel as RequestActiveModel, Entity as RequestEntity, Model as RequestModel};
use crate::models::lab_dip_resample::{self, ActiveModel as ResampleActiveModel, Entity as ResampleEntity, Model as ResampleModel};
use crate::models::lab_dip_sample::{self, ActiveModel as SampleActiveModel, Entity as SampleEntity, Model as SampleModel, FormulaDetailItem};
use crate::models::status::lab_dip_request as req_status;
use crate::models::status::lab_dip_resample as resample_status;
use crate::models::status::lab_dip_sample as sample_status;
use crate::utils::error::AppError;

/// 色差等级阈值（真实业务：4-5 级为 OK，<4 级为重打）
const COLOR_DIFF_OK_GRADE: i32 = 4;

// ============================================================================
// 打样通知单 Service
// ============================================================================

/// 创建打样通知单请求
///
/// 真实业务必填字段（依据 §11.1 打样通知单）：
/// - light_source: 对色光源（D65/TL84/U3000/CWF/A 等）
/// - sample_versions: 打样版数（默认 4，即 ABCD 四版）
/// - required_date: 客户要求交期
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabDipRequestRequest {
    pub customer_id: Option<i32>,
    pub customer_color_no: Option<String>,
    pub customer_color_name: Option<String>,
    pub sample_type: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_component: Option<String>,
    pub sample_size: Option<String>,
    /// 主对色光源（必填）：D65/TL84/U3000/CWF/A 等
    pub light_source: String,
    pub secondary_light_source: Option<String>,
    pub color_fastness_req: Option<String>,
    pub eco_requirement: Option<String>,
    /// 打样版数（默认 4，即 ABCD 四版）
    pub sample_versions: Option<i32>,
    pub dye_category: Option<String>,
    /// 客户要求交期（必填）
    pub required_date: chrono::NaiveDate,
    pub expected_days: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新打样通知单请求（仅 pending/sampling 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabDipRequestRequest {
    pub customer_id: Option<i32>,
    pub customer_color_no: Option<String>,
    pub customer_color_name: Option<String>,
    pub sample_type: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_component: Option<String>,
    pub sample_size: Option<String>,
    pub light_source: Option<String>,
    pub secondary_light_source: Option<String>,
    pub color_fastness_req: Option<String>,
    pub eco_requirement: Option<String>,
    pub sample_versions: Option<i32>,
    pub dye_category: Option<String>,
    pub required_date: Option<chrono::NaiveDate>,
    pub expected_days: Option<i32>,
    pub remarks: Option<String>,
}

/// 打样通知单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct LabDipRequestQuery {
    pub request_no: Option<String>,
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 打样通知单 Service
pub struct LabDipRequestService {
    db: Arc<DatabaseConnection>,
}

impl LabDipRequestService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成打样通知单号：LD-YYYYMMDDHHMMSS-NNN
    fn generate_request_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("LD-{}-{:03}", timestamp, random)
    }

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
            if v < 1 || v > 10 {
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

    // ===== 状态流转校验 =====

    /// 校验状态流转合法性
    ///
    /// 状态机：pending → sampling → submitted → approved/rejected → completed
    ///         rejected → sampling（重新打样）
    ///         approved → completed（复样通过后建库）
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            req_status::PENDING => matches!(new, req_status::SAMPLING),
            req_status::SAMPLING => matches!(new, req_status::SUBMITTED),
            req_status::SUBMITTED => matches!(new, req_status::APPROVED | req_status::REJECTED),
            req_status::REJECTED => matches!(new, req_status::SAMPLING),
            req_status::APPROVED => matches!(new, req_status::COMPLETED),
            req_status::COMPLETED => false, // 终态
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "打样通知单状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }

    /// 校验：仅 pending/sampling 状态可更新
    pub fn validate_can_update(status: &str) -> Result<(), AppError> {
        if !matches!(status, req_status::PENDING | req_status::SAMPLING) {
            return Err(AppError::business(format!(
                "当前状态 {} 不可更新（仅 pending/sampling 可更新）",
                status
            )));
        }
        Ok(())
    }

    /// 校验：仅 pending 状态可删除
    pub fn validate_can_delete(status: &str) -> Result<(), AppError> {
        if status != req_status::PENDING {
            return Err(AppError::business(format!(
                "当前状态 {} 不可删除（仅 pending 可删除）",
                status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 打样小样 Service
// ============================================================================

/// 创建打样小样请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabDipSampleRequest {
    pub request_id: i32,
    /// 版本标识：A/B/C/D/E...（如不传则自动生成）
    pub version_label: Option<String>,
    pub recipe_no: Option<String>,
    pub dye_recipe_id: Option<i32>,
    pub formula: Option<String>,
    pub formula_detail: Option<Vec<FormulaDetailItem>>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub liquor_ratio: Option<String>,
    pub ph_value: Option<Decimal>,
    pub dyeing_method: Option<String>,
    pub dye_cost: Option<Decimal>,
    pub auxiliary_cost: Option<Decimal>,
    pub total_cost: Option<Decimal>,
    pub color_difference_grade: Option<i32>,
    pub color_difference_value: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新打样小样请求（仅 pending 对色状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateLabDipSampleRequest {
    pub recipe_no: Option<String>,
    pub dye_recipe_id: Option<i32>,
    pub formula: Option<String>,
    pub formula_detail: Option<Vec<FormulaDetailItem>>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub liquor_ratio: Option<String>,
    pub ph_value: Option<Decimal>,
    pub dyeing_method: Option<String>,
    pub dye_cost: Option<Decimal>,
    pub auxiliary_cost: Option<Decimal>,
    pub total_cost: Option<Decimal>,
    pub color_difference_grade: Option<i32>,
    pub color_difference_value: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 记录对色结果请求
#[derive(Debug, Clone, Deserialize)]
pub struct RecordMatchingResultRequest {
    /// 色差等级（4-5 级为 OK，<4 级为重打）
    pub color_difference_grade: i32,
    pub color_difference_value: Option<Decimal>,
    /// 审核人
    pub approved_by: Option<i32>,
    pub approval_comment: Option<String>,
}

/// 打样小样 Service
pub struct LabDipSampleService {
    db: Arc<DatabaseConnection>,
}

impl LabDipSampleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 根据序号生成版本标识：1→A, 2→B, 3→C, 4→D, 5→E...
    fn label_from_seq(seq: i32) -> String {
        let c = ((seq - 1) as u8 + b'A') as char;
        c.to_string()
    }

    /// 创建打样小样
    pub async fn create(&self, req: CreateLabDipSampleRequest) -> Result<SampleModel, AppError> {
        // 校验通知单存在且处于 sampling 状态
        let request = RequestEntity::find_by_id(req.request_id)
            .filter(lab_dip_request::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样通知单 {} 不存在", req.request_id)))?;

        if request.status != req_status::SAMPLING {
            return Err(AppError::business(format!(
                "通知单状态 {} 不可添加小样（仅 sampling 状态可添加）",
                request.status
            )));
        }

        // 计算版本序号和标识
        let existing_count = SampleEntity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(req.request_id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await? as i32;

        let version_seq = existing_count + 1;

        // 业务校验：小样数量不能超过通知单规定的打样版数
        if version_seq > request.sample_versions {
            return Err(AppError::business(format!(
                "小样数量已达上限 {}（通知单规定打样版数）",
                request.sample_versions
            )));
        }

        let version_label = req
            .version_label
            .unwrap_or_else(|| Self::label_from_seq(version_seq));

        // 业务校验：版本标识唯一（同一通知单下）
        let exists = SampleEntity::find()
            .filter(lab_dip_sample::Column::RequestId.eq(req.request_id))
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

        let now = crate::utils::date_utils::utc_now_fixed();

        let active = SampleActiveModel {
            id: Default::default(),
            request_id: Set(req.request_id),
            version_label: Set(version_label),
            version_seq: Set(version_seq),
            recipe_no: Set(req.recipe_no),
            dye_recipe_id: Set(req.dye_recipe_id),
            formula: Set(req.formula),
            formula_detail: Set(req.formula_detail),
            temperature: Set(req.temperature),
            time_minutes: Set(req.time_minutes),
            liquor_ratio: Set(req.liquor_ratio),
            ph_value: Set(req.ph_value),
            dyeing_method: Set(req.dyeing_method),
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
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("打样小样创建失败: {}", e)))?;
        Ok(result)
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
        if req.color_difference_grade < 1 || req.color_difference_grade > 5 {
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

// ============================================================================
// 复样记录 Service
// ============================================================================

/// 创建复样记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateResampleRequest {
    pub request_id: i32,
    pub source_sample_id: i32,
    pub workshop_fabric_batch: Option<String>,
    pub dye_batch_no: Option<String>,
    pub auxiliary_batch_no: Option<String>,
    pub production_plan_id: Option<i32>,
    pub adjusted_formula: Option<String>,
    pub adjustment_factor: Option<Decimal>,
    pub adjusted_temperature: Option<Decimal>,
    pub adjusted_time_minutes: Option<i32>,
    pub adjusted_liquor_ratio: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 记录复样结果请求
#[derive(Debug, Clone, Deserialize)]
pub struct RecordResampleResultRequest {
    pub color_difference_grade: i32,
    pub color_difference_value: Option<Decimal>,
    pub reviewed_by: Option<i32>,
    pub review_comment: Option<String>,
}

/// 染色技术卡开具请求
#[derive(Debug, Clone, Deserialize)]
pub struct IssueTechCardRequest {
    /// 开卡人（研发组长）
    pub issued_by: i32,
}

/// 复样记录 Service
pub struct LabDipResampleService {
    db: Arc<DatabaseConnection>,
}

impl LabDipResampleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成复样单号：RS-YYYYMMDDHHMMSS-NNN
    fn generate_resample_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("RS-{}-{:03}", timestamp, random)
    }

    /// 创建复样记录
    ///
    /// 真实业务：OK 样确认后（通知单 approved 状态），大货生产前必须复样
    pub async fn create(&self, req: CreateResampleRequest) -> Result<ResampleModel, AppError> {
        // 校验通知单存在且处于 approved 状态
        let request = RequestEntity::find_by_id(req.request_id)
            .filter(lab_dip_request::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("打样通知单 {} 不存在", req.request_id)))?;

        if request.status != req_status::APPROVED {
            return Err(AppError::business(format!(
                "通知单状态 {} 不可创建复样（仅 approved 状态可复样）",
                request.status
            )));
        }

        // 校验源样存在且为 selected（OK 样）
        let source_sample = SampleEntity::find_by_id(req.source_sample_id)
            .filter(lab_dip_sample::Column::RequestId.eq(req.request_id))
            .filter(lab_dip_sample::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::business(format!("OK 样 {} 不存在或不属于该通知单", req.source_sample_id)))?;

        if source_sample.matching_result != sample_status::SELECTED {
            return Err(AppError::business(format!(
                "源样对色结果 {} 不可复样（仅 selected OK 样可复样）",
                source_sample.matching_result
            )));
        }

        // 真实业务校验：车间半制品布批号必填（不可用化验室存布）
        if req.workshop_fabric_batch.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            return Err(AppError::business("车间半制品布批号必填（复样必须用车间半制品布，不可用化验室存布）"));
        }

        let resample_no = Self::generate_resample_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = ResampleActiveModel {
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
        };

        // 创建复样记录同时更新源样复样状态
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("复样记录创建失败: {}", e)))?;

        // 更新源样 resample_status
        let mut sample_active: SampleActiveModel = source_sample.into();
        sample_active.resample_status = Set(Some("resampling".to_string()));
        sample_active.updated_at = Set(now);
        sample_active.update(&*self.db).await?;

        Ok(result)
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

        if req.color_difference_grade < 1 || req.color_difference_grade > 5 {
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试版本标识生成：1→A, 2→B, 3→C, 4→D
    #[test]
    fn test_label_from_seq() {
        assert_eq!(LabDipSampleService::label_from_seq(1), "A");
        assert_eq!(LabDipSampleService::label_from_seq(2), "B");
        assert_eq!(LabDipSampleService::label_from_seq(3), "C");
        assert_eq!(LabDipSampleService::label_from_seq(4), "D");
        assert_eq!(LabDipSampleService::label_from_seq(5), "E");
    }

    /// 测试打样通知单状态流转合法性
    #[test]
    fn test_request_status_transition_valid() {
        // 合法流转
        assert!(LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::SAMPLING).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::SAMPLING, req_status::SUBMITTED).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::SUBMITTED, req_status::APPROVED).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::SUBMITTED, req_status::REJECTED).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::REJECTED, req_status::SAMPLING).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::APPROVED, req_status::COMPLETED).is_ok());
    }

    /// 测试打样通知单状态流转非法
    #[test]
    fn test_request_status_transition_invalid() {
        // 非法流转
        assert!(LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::SUBMITTED).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::SAMPLING, req_status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::APPROVED, req_status::SAMPLING).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::COMPLETED, req_status::SAMPLING).is_err());
    }

    /// 测试通知单更新状态校验
    #[test]
    fn test_validate_can_update() {
        assert!(LabDipRequestService::validate_can_update(req_status::PENDING).is_ok());
        assert!(LabDipRequestService::validate_can_update(req_status::SAMPLING).is_ok());
        assert!(LabDipRequestService::validate_can_update(req_status::SUBMITTED).is_err());
        assert!(LabDipRequestService::validate_can_update(req_status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_can_update(req_status::COMPLETED).is_err());
    }

    /// 测试通知单删除状态校验
    #[test]
    fn test_validate_can_delete() {
        assert!(LabDipRequestService::validate_can_delete(req_status::PENDING).is_ok());
        assert!(LabDipRequestService::validate_can_delete(req_status::SAMPLING).is_err());
        assert!(LabDipRequestService::validate_can_delete(req_status::APPROVED).is_err());
    }

    /// 测试打样通知单号生成格式
    #[test]
    fn test_generate_request_no() {
        let no = LabDipRequestService::generate_request_no();
        assert!(no.starts_with("LD-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // 3 位随机
    }

    /// 测试复样单号生成格式
    #[test]
    fn test_generate_resample_no() {
        let no = LabDipResampleService::generate_resample_no();
        assert!(no.starts_with("RS-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14);
        assert_eq!(parts[2].len(), 3);
    }
}

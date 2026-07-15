//! 染色配方 Service
//!
//! v14 批次 423A：从 handler 抽取业务逻辑，建立 service 抽象层。
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样流程 + §13.1 批次 423 规划
//!
//! 核心能力：
//! - 配方 CRUD + 软删除
//! - 状态流转校验（草稿→已审核/已停用；已审核→已停用；已停用→已审核）
//! - 审核流程（仅草稿可审核）
//! - 版本管理（仅已审核可建新版本，version+1，parent_recipe_id 关联）

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::dye_recipe::{self, ActiveModel, Entity as DyeRecipeEntity, Model as DyeRecipeModel};
use crate::models::status::dye_recipe as recipe_status;
use crate::utils::error::AppError;

/// 创建染色配方请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDyeRecipeRequest {
    pub recipe_no: Option<String>,
    pub recipe_name: Option<String>,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<Decimal>,
    pub liquor_ratio: Option<Decimal>,
    pub auxiliaries: Option<Vec<crate::models::dye_recipe::AuxiliariesItem>>,
    pub status: Option<String>,
    pub version: Option<i32>,
    pub parent_recipe_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新染色配方请求
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateDyeRecipeRequest {
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<Decimal>,
    pub liquor_ratio: Option<Decimal>,
    pub auxiliaries: Option<Vec<crate::models::dye_recipe::AuxiliariesItem>>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

/// 染色配方查询参数
#[derive(Debug, Clone, Default)]
pub struct DyeRecipeQuery {
    pub recipe_no: Option<String>,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub dye_type: Option<String>,
    pub status: Option<String>,
    pub page: u64,
    pub page_size: u64,
}

/// 染色配方 Service
pub struct DyeRecipeService {
    db: Arc<DatabaseConnection>,
}

impl DyeRecipeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成配方编号（格式：DR-{时间戳}-{4位随机}）
    /// 若调用方提供了非空编号则直接使用
    pub fn generate_recipe_no(provided: Option<&str>) -> String {
        if let Some(no) = provided {
            if !no.is_empty() {
                return no.to_string();
            }
        }
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_4_digit();
        format!("DR-{}-{:04}", timestamp, random)
    }

    /// 校验配方状态流转是否合法
    /// 草稿 → 已审核 / 已停用
    /// 已审核 → 已停用
    /// 已停用 → 已审核
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            recipe_status::DRAFT => matches!(new, recipe_status::APPROVED | recipe_status::DISABLED),
            recipe_status::APPROVED => matches!(new, recipe_status::DISABLED),
            recipe_status::DISABLED => matches!(new, recipe_status::APPROVED),
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "配方状态流转不合法：{} -> {}",
                current, new
            )));
        }
        Ok(())
    }

    /// 校验配方是否允许删除（已审核的配方不允许删除）
    pub fn validate_can_delete(status: Option<&str>) -> Result<(), AppError> {
        if status == Some(recipe_status::APPROVED) {
            return Err(AppError::business(
                "已审核的配方不允许删除，请先停用",
            ));
        }
        Ok(())
    }

    /// 校验配方是否允许审核（仅草稿状态可审核）
    pub fn validate_can_approve(status: Option<&str>) -> Result<(), AppError> {
        if status != Some(recipe_status::DRAFT) {
            return Err(AppError::business(format!(
                "只有草稿状态的配方可以审核，当前状态：{}",
                status.unwrap_or("未知")
            )));
        }
        Ok(())
    }

    /// 校验配方是否允许创建新版本（仅已审核状态可建新版本）
    pub fn validate_can_create_version(status: Option<&str>) -> Result<(), AppError> {
        if status != Some(recipe_status::APPROVED) {
            return Err(AppError::business(format!(
                "只有已审核的配方可以创建新版本，当前状态：{}",
                status.unwrap_or("未知")
            )));
        }
        Ok(())
    }

    /// 创建染色配方
    pub async fn create(&self, req: CreateDyeRecipeRequest) -> Result<DyeRecipeModel, AppError> {
        let recipe_no = Self::generate_recipe_no(req.recipe_no.as_deref());

        let active = ActiveModel {
            id: Default::default(),
            recipe_no: Set(recipe_no),
            recipe_name: Set(Some(
                req.recipe_name.unwrap_or_else(|| "未命名配方".to_string()),
            )),
            color_no: Set(req.color_code.clone()),
            formula: Set(req.chemical_formula.clone()),
            color_code: Set(req.color_code),
            color_name: Set(req.color_name),
            fabric_type: Set(req.fabric_type),
            dye_type: Set(req.dye_type),
            chemical_formula: Set(req.chemical_formula),
            temperature: Set(req.temperature),
            time_minutes: Set(req.time_minutes),
            ph_value: Set(req.ph_value),
            liquor_ratio: Set(req.liquor_ratio),
            auxiliaries: Set(req.auxiliaries),
            status: Set(Some(
                req.status
                    .unwrap_or_else(|| recipe_status::DRAFT.to_string()),
            )),
            is_deleted: Set(Some(false)),
            version: Set(req.version.or(Some(1))),
            parent_recipe_id: Set(req.parent_recipe_id),
            approved_by: Set(None),
            approved_at: Set(None),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(crate::utils::date_utils::utc_now_fixed()),
            updated_at: Set(crate::utils::date_utils::utc_now_fixed()),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("配方创建失败: {}", e)))?;
        Ok(result)
    }

    /// 根据 ID 获取配方
    pub async fn get_by_id(&self, id: i32) -> Result<DyeRecipeModel, AppError> {
        DyeRecipeEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("配方不存在"))
    }

    /// 分页查询配方列表
    pub async fn list(
        &self,
        query: DyeRecipeQuery,
    ) -> Result<(Vec<DyeRecipeModel>, u64), AppError> {
        let page = query.page.clamp(1, 1000);
        let page_size = query.page_size.clamp(1, 100);

        let mut select = DyeRecipeEntity::find().filter(dye_recipe::Column::IsDeleted.eq(false));

        if let Some(recipe_no) = &query.recipe_no {
            select = select.filter(dye_recipe::Column::RecipeNo.contains(recipe_no));
        }
        if let Some(color_code) = &query.color_code {
            select = select.filter(dye_recipe::Column::ColorCode.contains(color_code));
        }
        if let Some(color_name) = &query.color_name {
            select = select.filter(dye_recipe::Column::ColorName.contains(color_name));
        }
        if let Some(dye_type) = &query.dye_type {
            select = select.filter(dye_recipe::Column::DyeType.eq(dye_type));
        }
        if let Some(status) = &query.status {
            select = select.filter(dye_recipe::Column::Status.eq(status));
        }

        let paginator = select
            .order_by_desc(dye_recipe::Column::CreatedAt)
            .paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let recipes = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((recipes, total))
    }

    /// 更新配方
    pub async fn update(
        &self,
        id: i32,
        req: UpdateDyeRecipeRequest,
    ) -> Result<DyeRecipeModel, AppError> {
        let model = self.get_by_id(id).await?;
        // 在转为 ActiveModel 前记录当前状态，用于状态流转校验
        // 注意：必须 clone 后再 model.into()，否则 as_deref() 借用 model.status 会与
        // 后续 model.into() 移动 model 冲突（E0505 cannot move out of borrowed）
        let current_status = model
            .status
            .clone()
            .unwrap_or_else(|| recipe_status::DRAFT.to_string());
        let mut active: ActiveModel = model.into();

        if let Some(color_code) = req.color_code {
            active.color_code = Set(Some(color_code));
        }
        if let Some(color_name) = req.color_name {
            active.color_name = Set(Some(color_name));
        }
        if let Some(fabric_type) = req.fabric_type {
            active.fabric_type = Set(Some(fabric_type));
        }
        if let Some(dye_type) = req.dye_type {
            active.dye_type = Set(Some(dye_type));
        }
        if let Some(chemical_formula) = req.chemical_formula {
            active.chemical_formula = Set(Some(chemical_formula));
        }
        if let Some(temperature) = req.temperature {
            active.temperature = Set(Some(temperature));
        }
        if let Some(time_minutes) = req.time_minutes {
            active.time_minutes = Set(Some(time_minutes));
        }
        if let Some(ph_value) = req.ph_value {
            active.ph_value = Set(Some(ph_value));
        }
        if let Some(liquor_ratio) = req.liquor_ratio {
            active.liquor_ratio = Set(Some(liquor_ratio));
        }
        if let Some(auxiliaries) = req.auxiliaries {
            active.auxiliaries = Set(Some(auxiliaries));
        }
        if let Some(status) = req.status {
            // 校验状态流转合法性
            Self::validate_status_transition(&current_status, &status)?;
            active.status = Set(Some(status));
        }
        if let Some(remarks) = req.remarks {
            active.remarks = Set(Some(remarks));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除配方
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_delete(model.status.as_deref())?;

        let mut active: ActiveModel = model.into();
        active.is_deleted = Set(Some(true));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 审核配方
    pub async fn approve(&self, id: i32, approved_by: i32) -> Result<DyeRecipeModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_approve(model.status.as_deref())?;

        let mut active: ActiveModel = model.into();
        active.status = Set(Some(recipe_status::APPROVED.to_string()));
        active.approved_by = Set(Some(approved_by));
        active.approved_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 创建新版本（仅已审核配方可建新版本）
    pub async fn create_new_version(
        &self,
        id: i32,
        remarks: Option<String>,
        created_by: Option<i32>,
    ) -> Result<DyeRecipeModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_create_version(model.status.as_deref())?;

        let new_version = model.version.unwrap_or(1) + 1;
        let new_recipe_no = format!("{}-V{}", model.recipe_no, new_version);

        let active = ActiveModel {
            id: Default::default(),
            recipe_no: Set(new_recipe_no),
            recipe_name: Set(model.recipe_name.clone()),
            color_no: Set(model.color_no.clone()),
            formula: Set(model.formula.clone()),
            color_code: Set(model.color_code.clone()),
            color_name: Set(model.color_name.clone()),
            fabric_type: Set(model.fabric_type.clone()),
            dye_type: Set(model.dye_type.clone()),
            chemical_formula: Set(model.chemical_formula.clone()),
            temperature: Set(model.temperature),
            time_minutes: Set(model.time_minutes),
            ph_value: Set(model.ph_value),
            liquor_ratio: Set(model.liquor_ratio),
            auxiliaries: Set(model.auxiliaries.clone()),
            status: Set(Some(recipe_status::DRAFT.to_string())),
            is_deleted: Set(Some(false)),
            version: Set(Some(new_version)),
            parent_recipe_id: Set(Some(id)),
            approved_by: Set(None),
            approved_at: Set(None),
            remarks: Set(remarks),
            created_by: Set(created_by),
            created_at: Set(crate::utils::date_utils::utc_now_fixed()),
            updated_at: Set(crate::utils::date_utils::utc_now_fixed()),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("配方新版本创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按色号查询已审核配方（按版本降序）
    pub async fn get_recipes_by_color(
        &self,
        color_code: &str,
    ) -> Result<Vec<DyeRecipeModel>, AppError> {
        let recipes = DyeRecipeEntity::find()
            .filter(dye_recipe::Column::ColorCode.eq(color_code))
            .filter(dye_recipe::Column::Status.eq(recipe_status::APPROVED))
            .filter(dye_recipe::Column::IsDeleted.eq(false))
            .order_by_desc(dye_recipe::Column::Version)
            .all(&*self.db)
            .await?;
        Ok(recipes)
    }

    /// 获取配方的所有版本
    pub async fn get_recipe_versions(&self, id: i32) -> Result<Vec<DyeRecipeModel>, AppError> {
        let model = self.get_by_id(id).await?;
        let parent_id = model.parent_recipe_id.unwrap_or(id);

        let versions = DyeRecipeEntity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(dye_recipe::Column::Id.eq(parent_id))
                    .add(dye_recipe::Column::ParentRecipeId.eq(parent_id)),
            )
            .filter(dye_recipe::Column::IsDeleted.eq(false))
            .order_by_desc(dye_recipe::Column::Version)
            .all(&*self.db)
            .await?;
        Ok(versions)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试配方编号自动生成格式
    #[test]
    fn test_generate_recipe_no_auto() {
        let no = DyeRecipeService::generate_recipe_no(None);
        assert!(no.starts_with("DR-"));
        // 格式：DR-{14位时间戳}-{4位随机}
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // 时间戳 YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 4); // 4 位随机
    }

    /// 测试配方编号使用调用方提供的值
    #[test]
    fn test_generate_recipe_no_provided() {
        let no = DyeRecipeService::generate_recipe_no(Some("CUSTOM-001"));
        assert_eq!(no, "CUSTOM-001");
    }

    /// 测试配方编号空字符串时自动生成
    #[test]
    fn test_generate_recipe_no_empty() {
        let no = DyeRecipeService::generate_recipe_no(Some(""));
        assert!(no.starts_with("DR-"));
    }

    /// 测试状态流转：草稿 → 已审核（合法）
    #[test]
    fn test_status_transition_draft_to_approved() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::APPROVED
        )
        .is_ok());
    }

    /// 测试状态流转：草稿 → 已停用（合法）
    #[test]
    fn test_status_transition_draft_to_disabled() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::DISABLED
        )
        .is_ok());
    }

    /// 测试状态流转：已审核 → 已停用（合法）
    #[test]
    fn test_status_transition_approved_to_disabled() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::DISABLED
        )
        .is_ok());
    }

    /// 测试状态流转：已停用 → 已审核（合法）
    #[test]
    fn test_status_transition_disabled_to_approved() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::DISABLED,
            recipe_status::APPROVED
        )
        .is_ok());
    }

    /// 测试状态流转：草稿 → 草稿（非法，不能自转）
    #[test]
    fn test_status_transition_draft_to_draft_invalid() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::DRAFT
        )
        .is_err());
    }

    /// 测试状态流转：已审核 → 草稿（非法，不能回退到草稿）
    #[test]
    fn test_status_transition_approved_to_draft_invalid() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::DRAFT
        )
        .is_err());
    }

    /// 测试状态流转：已审核 → 已审核（非法，不能自转）
    #[test]
    fn test_status_transition_approved_to_approved_invalid() {
        assert!(DyeRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::APPROVED
        )
        .is_err());
    }

    /// 测试状态流转：未知状态（非法）
    #[test]
    fn test_status_transition_unknown_status() {
        assert!(DyeRecipeService::validate_status_transition("未知", recipe_status::APPROVED)
            .is_err());
    }

    /// 测试删除校验：已审核配方不允许删除
    #[test]
    fn test_validate_can_delete_approved() {
        assert!(DyeRecipeService::validate_can_delete(Some(recipe_status::APPROVED)).is_err());
    }

    /// 测试删除校验：草稿配方允许删除
    #[test]
    fn test_validate_can_delete_draft() {
        assert!(DyeRecipeService::validate_can_delete(Some(recipe_status::DRAFT)).is_ok());
    }

    /// 测试删除校验：已停用配方允许删除
    #[test]
    fn test_validate_can_delete_disabled() {
        assert!(DyeRecipeService::validate_can_delete(Some(recipe_status::DISABLED)).is_ok());
    }

    /// 测试删除校验：None 状态允许删除
    #[test]
    fn test_validate_can_delete_none() {
        assert!(DyeRecipeService::validate_can_delete(None).is_ok());
    }

    /// 测试审核校验：草稿状态可审核
    #[test]
    fn test_validate_can_approve_draft() {
        assert!(DyeRecipeService::validate_can_approve(Some(recipe_status::DRAFT)).is_ok());
    }

    /// 测试审核校验：已审核状态不可审核
    #[test]
    fn test_validate_can_approve_approved() {
        assert!(DyeRecipeService::validate_can_approve(Some(recipe_status::APPROVED)).is_err());
    }

    /// 测试审核校验：已停用状态不可审核
    #[test]
    fn test_validate_can_approve_disabled() {
        assert!(DyeRecipeService::validate_can_approve(Some(recipe_status::DISABLED)).is_err());
    }

    /// 测试创建版本校验：已审核状态可创建新版本
    #[test]
    fn test_validate_can_create_version_approved() {
        assert!(
            DyeRecipeService::validate_can_create_version(Some(recipe_status::APPROVED)).is_ok()
        );
    }

    /// 测试创建版本校验：草稿状态不可创建新版本
    #[test]
    fn test_validate_can_create_version_draft() {
        assert!(DyeRecipeService::validate_can_create_version(Some(recipe_status::DRAFT)).is_err());
    }

    /// 测试创建版本校验：已停用状态不可创建新版本
    #[test]
    fn test_validate_can_create_version_disabled() {
        assert!(
            DyeRecipeService::validate_can_create_version(Some(recipe_status::DISABLED)).is_err()
        );
    }

    /// 测试状态常量值正确性
    #[test]
    fn test_status_constants() {
        assert_eq!(recipe_status::DRAFT, "草稿");
        assert_eq!(recipe_status::APPROVED, "已审核");
        assert_eq!(recipe_status::DISABLED, "已停用");
    }
}

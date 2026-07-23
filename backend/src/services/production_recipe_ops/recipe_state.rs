//! 大货处方状态流转 impl 子模块（production_recipe_ops/recipe_state）
//!
//! D10-6a 拆分：从原 production_recipe_service.rs 迁移 ProductionRecipeService 的
//! 3 个状态流转方法（approve / close / cancel）。
//! 状态流转校验纯函数 validate_status_transition 保留在 facade，本模块通过 Self:: 调用。
//! 跨 ops 子模块调用：approve / close / cancel 通过 self.get_by_id 访问
//! recipe_crud 子模块定义的查询方法（get_by_id 为 pub）。

use sea_orm::{ActiveModelTrait, Set};

use crate::models::production_recipe::{ActiveModel as RecipeActiveModel, Model as RecipeModel};
use crate::models::status::production_recipe as recipe_status;
use crate::services::production_recipe_service::{ApproveRecipeRequest, ProductionRecipeService};
use crate::utils::error::AppError;

impl ProductionRecipeService {
    /// 审核大货处方（draft → approved）
    ///
    /// 真实业务：审核后自动建立生产领用单据（领用单据建立由下游模块消费 approved 事件）
    pub async fn approve(
        &self,
        id: i32,
        req: ApproveRecipeRequest,
    ) -> Result<RecipeModel, AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_status_transition(&model.status, recipe_status::APPROVED)?;

        // 业务校验：处方明细非空（审核前必须有物料明细）
        let detail = model.recipe_detail.as_ref();
        if detail.map(|d| d.is_empty()).unwrap_or(true) {
            return Err(AppError::business("审核前处方明细不能为空"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let mut active: RecipeActiveModel = model.into();
        active.status = Set(recipe_status::APPROVED.to_string());
        active.approved_by = Set(Some(req.approved_by));
        active.approved_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭大货处方（approved → closed）
    ///
    /// 真实业务：生产完成，处方归档
    pub async fn close(&self, id: i32) -> Result<RecipeModel, AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_status_transition(&model.status, recipe_status::CLOSED)?;

        let mut active: RecipeActiveModel = model.into();
        active.status = Set(recipe_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消大货处方（draft → cancelled）
    ///
    /// 真实业务：草稿状态作废
    pub async fn cancel(&self, id: i32) -> Result<RecipeModel, AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_status_transition(&model.status, recipe_status::CANCELLED)?;

        let mut active: RecipeActiveModel = model.into();
        active.status = Set(recipe_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

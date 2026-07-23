//! 加料处方 impl 子模块（production_recipe_ops/addition）
//!
//! D10-6a 拆分：从原 production_recipe_service.rs 迁移 ProductionRecipeAdditionService 的
//! 6 个 impl 方法（create / get_by_id / list_by_recipe / list / approve / close）。
//! 单号生成与状态校验纯函数保留在 facade，本模块通过 Self:: 调用。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::production_recipe::{self, Entity as RecipeEntity};
use crate::models::production_recipe_addition::{
    self, ActiveModel as AdditionActiveModel, Entity as AdditionEntity, Model as AdditionModel,
};
use crate::models::status::production_recipe as recipe_status;
use crate::models::status::production_recipe_addition as addition_status;
use crate::services::production_recipe_service::{
    ApproveRecipeRequest, CreateProductionRecipeAdditionRequest, ProductionRecipeAdditionQuery,
    ProductionRecipeAdditionService,
};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

impl ProductionRecipeAdditionService {
    /// 创建加料处方
    ///
    /// 业务校验：关联的大货处方必须为 approved 状态
    pub async fn create(
        &self,
        req: CreateProductionRecipeAdditionRequest,
    ) -> Result<AdditionModel, AppError> {
        // 校验大货处方存在且为 approved 状态
        let recipe = RecipeEntity::find_by_id(req.production_recipe_id)
            .filter(production_recipe::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("大货处方单 {} 不存在", req.production_recipe_id))
            })?;

        if recipe.status != recipe_status::APPROVED {
            return Err(AppError::business(format!(
                "大货处方单状态 {} 不可创建加料处方（仅 approved 状态可创建加料处方）",
                recipe.status
            )));
        }

        let addition_no = Self::generate_addition_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = AdditionActiveModel {
            id: Default::default(),
            addition_no: Set(addition_no),
            production_recipe_id: Set(req.production_recipe_id),
            work_order_id: Set(req.work_order_id.or(recipe.work_order_id)),
            dye_batch_id: Set(req.dye_batch_id.or(recipe.dye_batch_id)),
            addition_reason: Set(req.addition_reason),
            addition_detail: Set(req.addition_detail),
            total_cost: Set(req.total_cost),
            status: Set(addition_status::DRAFT.to_string()),
            approved_by: Set(None),
            approved_at: Set(None),
            issued_by: Set(req.issued_by),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("加料处方单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按 ID 查询加料处方
    pub async fn get_by_id(
        &self,
        id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<AdditionModel, AppError> {
        let model = AdditionEntity::find_by_id(id)
            .filter(production_recipe_addition::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("加料处方单 {} 不存在", id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // production_recipe_addition 表无 department_id，Dept 退化为 Self（按 created_by 校验）
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问加料处方单 {}（数据范围限制）",
                    id
                )));
            }
        }

        Ok(model)
    }

    /// 按大货处方查询加料单列表
    pub async fn list_by_recipe(
        &self,
        recipe_id: i32,
    ) -> Result<Vec<AdditionModel>, AppError> {
        let items = AdditionEntity::find()
            .filter(production_recipe_addition::Column::ProductionRecipeId.eq(recipe_id))
            .filter(production_recipe_addition::Column::IsDeleted.eq(false))
            .order_by_desc(production_recipe_addition::Column::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 分页查询加料处方
    pub async fn list(
        &self,
        query: ProductionRecipeAdditionQuery,
    ) -> Result<(Vec<AdditionModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = AdditionEntity::find()
            .filter(production_recipe_addition::Column::IsDeleted.eq(false));

        if let Some(rid) = query.production_recipe_id {
            q = q.filter(production_recipe_addition::Column::ProductionRecipeId.eq(rid));
        }
        if let Some(wid) = query.work_order_id {
            q = q.filter(production_recipe_addition::Column::WorkOrderId.eq(wid));
        }
        if let Some(status) = &query.status {
            q = q.filter(production_recipe_addition::Column::Status.eq(status));
        }

        q = q.order_by_desc(production_recipe_addition::Column::CreatedAt);

        let paginator = q.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 审核加料处方（draft → approved）
    pub async fn approve(
        &self,
        id: i32,
        req: ApproveRecipeRequest,
    ) -> Result<AdditionModel, AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_status_transition(&model.status, addition_status::APPROVED)?;

        // 业务校验：加料明细非空
        let detail = model.addition_detail.as_ref();
        if detail.map(|d| d.is_empty()).unwrap_or(true) {
            return Err(AppError::business("审核前加料明细不能为空"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let mut active: AdditionActiveModel = model.into();
        active.status = Set(addition_status::APPROVED.to_string());
        active.approved_by = Set(Some(req.approved_by));
        active.approved_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭加料处方（approved → closed）
    pub async fn close(&self, id: i32) -> Result<AdditionModel, AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_status_transition(&model.status, addition_status::CLOSED)?;

        let mut active: AdditionActiveModel = model.into();
        active.status = Set(addition_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

//! 大货处方 CRUD 与查询 impl 子模块（production_recipe_ops/recipe_crud）
//!
//! D10-6a 拆分：从原 production_recipe_service.rs 迁移 ProductionRecipeService 的
//! 7 个 CRUD/查询方法（create / update / delete / get_by_id / list /
//! get_by_work_order / list_additions_by_recipe）及 update 的 4 个辅助方法
//!（validate_work_order_change / validate_update_fields /
//! apply_recipe_relation_fields / apply_recipe_text_fields）。
//! 单号生成与状态校验纯函数保留在 facade，本模块通过 Self:: 调用。

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::production_recipe::{
    self, ActiveModel as RecipeActiveModel, Entity as RecipeEntity, Model as RecipeModel,
};
use crate::models::production_recipe_addition::{
    self, Entity as AdditionEntity, Model as AdditionModel,
};
use crate::models::status::production_recipe as recipe_status;
use crate::services::production_recipe_service::{
    CreateProductionRecipeRequest, ProductionRecipeQuery, ProductionRecipeService,
    UpdateProductionRecipeRequest,
};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

impl ProductionRecipeService {
    /// 创建大货处方
    ///
    /// 业务校验：
    /// 1. 同一 work_order_id 不可重复开具（一工单一处方约束）
    /// 2. 浴比非空且格式正确
    /// 3. 备布重量 > 0
    pub async fn create(
        &self,
        req: CreateProductionRecipeRequest,
    ) -> Result<RecipeModel, AppError> {
        // 业务校验：备布重量必须 > 0
        if req.fabric_weight <= Decimal::ZERO {
            return Err(AppError::business("备布重量必须大于 0"));
        }

        // 业务校验：浴比非空且格式正确
        if req.liquor_ratio.trim().is_empty() {
            return Err(AppError::business("浴比必填（如 1:8）"));
        }
        let _ratio_value = Self::parse_liquor_ratio(&req.liquor_ratio)?;

        // 业务校验：同一 work_order_id 不可重复开具（一工单一处方约束）
        if let Some(work_order_id) = req.work_order_id {
            let exists = RecipeEntity::find()
                .filter(production_recipe::Column::WorkOrderId.eq(work_order_id))
                .filter(production_recipe::Column::IsDeleted.eq(false))
                .filter(production_recipe::Column::Status.ne(recipe_status::CANCELLED))
                .count(&*self.db)
                .await?;
            if exists > 0 {
                return Err(AppError::business(format!(
                    "工单 {} 已存在大货处方单（同一工单只能开一张大货处方单，追加物料须开加料处方单）",
                    work_order_id
                )));
            }
        }

        let recipe_no = Self::generate_recipe_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RecipeActiveModel {
            id: Default::default(),
            recipe_no: Set(recipe_no),
            work_order_id: Set(req.work_order_id),
            dye_batch_id: Set(req.dye_batch_id),
            source_recipe_id: Set(req.source_recipe_id),
            lab_dip_resample_id: Set(req.lab_dip_resample_id),
            customer_id: Set(req.customer_id),
            color_no: Set(req.color_no),
            fabric_name: Set(req.fabric_name),
            fabric_spec: Set(req.fabric_spec),
            fabric_width: Set(req.fabric_width),
            gram_weight: Set(req.gram_weight),
            fabric_weight: Set(req.fabric_weight),
            equipment_no: Set(req.equipment_no),
            liquor_ratio: Set(req.liquor_ratio),
            bath_volume: Set(req.bath_volume),
            adjustment_factor: Set(req.adjustment_factor),
            recipe_detail: Set(req.recipe_detail),
            total_dye_cost: Set(req.total_dye_cost),
            total_auxiliary_cost: Set(req.total_auxiliary_cost),
            status: Set(recipe_status::DRAFT.to_string()),
            approved_by: Set(None),
            approved_at: Set(None),
            issued_by: Set(req.issued_by),
            printed_count: Set(Some(0)),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("大货处方单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新大货处方（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateProductionRecipeRequest,
    ) -> Result<RecipeModel, AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_can_update(&model.status)?;
        self.validate_work_order_change(id, &req, &model).await?;
        Self::validate_update_fields(&req)?;
        let mut active: RecipeActiveModel = model.into();
        Self::apply_recipe_relation_fields(&mut active, &req);
        Self::apply_recipe_text_fields(&mut active, &req);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 校验工单变更后的一工单一处方约束
    async fn validate_work_order_change(
        &self,
        id: i32,
        req: &UpdateProductionRecipeRequest,
        model: &RecipeModel,
    ) -> Result<(), AppError> {
        if let Some(new_work_order_id) = req.work_order_id {
            if Some(new_work_order_id) != model.work_order_id {
                let exists = RecipeEntity::find()
                    .filter(production_recipe::Column::WorkOrderId.eq(new_work_order_id))
                    .filter(production_recipe::Column::Id.ne(id))
                    .filter(production_recipe::Column::IsDeleted.eq(false))
                    .filter(production_recipe::Column::Status.ne(recipe_status::CANCELLED))
                    .count(&*self.db)
                    .await?;
                if exists > 0 {
                    return Err(AppError::business(format!(
                        "工单 {} 已存在其他大货处方单（一工单一处方约束）",
                        new_work_order_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// 校验备布重量和浴比格式
    fn validate_update_fields(req: &UpdateProductionRecipeRequest) -> Result<(), AppError> {
        if let Some(w) = req.fabric_weight {
            if w <= Decimal::ZERO {
                return Err(AppError::business("备布重量必须大于 0"));
            }
        }
        if let Some(r) = &req.liquor_ratio {
            if r.trim().is_empty() {
                return Err(AppError::business("浴比不能为空"));
            }
            let _ = Self::parse_liquor_ratio(r)?;
        }
        Ok(())
    }

    /// 应用关联 ID 和数值型字段
    fn apply_recipe_relation_fields(
        active: &mut RecipeActiveModel,
        req: &UpdateProductionRecipeRequest,
    ) {
        if let Some(v) = req.work_order_id {
            active.work_order_id = Set(Some(v));
        }
        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = req.source_recipe_id {
            active.source_recipe_id = Set(Some(v));
        }
        if let Some(v) = req.lab_dip_resample_id {
            active.lab_dip_resample_id = Set(Some(v));
        }
        if let Some(v) = req.customer_id {
            active.customer_id = Set(Some(v));
        }
        if let Some(v) = req.fabric_width {
            active.fabric_width = Set(Some(v));
        }
        if let Some(v) = req.gram_weight {
            active.gram_weight = Set(Some(v));
        }
        if let Some(v) = req.fabric_weight {
            active.fabric_weight = Set(v);
        }
        if let Some(v) = req.bath_volume {
            active.bath_volume = Set(Some(v));
        }
        if let Some(v) = req.adjustment_factor {
            active.adjustment_factor = Set(Some(v));
        }
        if let Some(v) = req.total_dye_cost {
            active.total_dye_cost = Set(Some(v));
        }
        if let Some(v) = req.total_auxiliary_cost {
            active.total_auxiliary_cost = Set(Some(v));
        }
    }

    /// 应用文本和明细字段（需 clone 避免移动）
    fn apply_recipe_text_fields(
        active: &mut RecipeActiveModel,
        req: &UpdateProductionRecipeRequest,
    ) {
        if let Some(v) = &req.color_no {
            active.color_no = Set(Some(v.clone()));
        }
        if let Some(v) = &req.fabric_name {
            active.fabric_name = Set(Some(v.clone()));
        }
        if let Some(v) = &req.fabric_spec {
            active.fabric_spec = Set(Some(v.clone()));
        }
        if let Some(v) = &req.equipment_no {
            active.equipment_no = Set(Some(v.clone()));
        }
        if let Some(v) = &req.liquor_ratio {
            active.liquor_ratio = Set(v.clone());
        }
        if let Some(v) = &req.recipe_detail {
            active.recipe_detail = Set(Some(v.clone()));
        }
        if let Some(v) = &req.remarks {
            active.remarks = Set(Some(v.clone()));
        }
    }

    /// 软删除大货处方（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id, None).await?;
        Self::validate_can_delete(&model.status)?;

        let mut active: RecipeActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询大货处方
    pub async fn get_by_id(
        &self,
        id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<RecipeModel, AppError> {
        let model = RecipeEntity::find_by_id(id)
            .filter(production_recipe::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("大货处方单 {} 不存在", id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // production_recipe 表无 department_id，Dept 退化为 Self（按 created_by 校验）
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, model.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问大货处方单 {}（数据范围限制）",
                    id
                )));
            }
        }

        Ok(model)
    }

    /// 分页查询大货处方
    pub async fn list(
        &self,
        query: ProductionRecipeQuery,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<RecipeModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RecipeEntity::find().filter(production_recipe::Column::IsDeleted.eq(false));

        // V15 P0-S01：行级数据权限过滤（production_recipe 表无 department_id，Dept 退化为 Self）
        if let Some(ctx) = data_scope {
            q = apply_data_scope(
                q,
                ctx,
                production_recipe::Column::CreatedBy,
                production_recipe::Column::CreatedBy,
            );
        }

        if let Some(wid) = query.work_order_id {
            q = q.filter(production_recipe::Column::WorkOrderId.eq(wid));
        }
        if let Some(bid) = query.dye_batch_id {
            q = q.filter(production_recipe::Column::DyeBatchId.eq(bid));
        }
        if let Some(cid) = query.customer_id {
            q = q.filter(production_recipe::Column::CustomerId.eq(cid));
        }
        if let Some(color_no) = &query.color_no {
            q = q.filter(production_recipe::Column::ColorNo.contains(color_no));
        }
        if let Some(status) = &query.status {
            q = q.filter(production_recipe::Column::Status.eq(status));
        }

        q = q.order_by_desc(production_recipe::Column::CreatedAt);

        let paginator = q.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 按工单查询大货处方（业务约束：一工单一处方）
    pub async fn get_by_work_order(
        &self,
        work_order_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Option<RecipeModel>, AppError> {
        let model = RecipeEntity::find()
            .filter(production_recipe::Column::WorkOrderId.eq(work_order_id))
            .filter(production_recipe::Column::IsDeleted.eq(false))
            .filter(production_recipe::Column::Status.ne(recipe_status::CANCELLED))
            .order_by_desc(production_recipe::Column::CreatedAt)
            .one(&*self.db)
            .await?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        if let (Some(ctx), Some(m)) = (data_scope, &model) {
            if !check_resource_owner(ctx, m.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问工单 {} 的大货处方（数据范围限制）",
                    work_order_id
                )));
            }
        }

        Ok(model)
    }

    /// 查询大货处方下的加料单
    pub async fn list_additions_by_recipe(
        &self,
        recipe_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<AdditionModel>, AppError> {
        // V15 P0-S01：校验大货处方存在 + 行级数据权限（透传 data_scope 给 get_by_id）
        let _ = self.get_by_id(recipe_id, data_scope).await?;
        let items = AdditionEntity::find()
            .filter(production_recipe_addition::Column::ProductionRecipeId.eq(recipe_id))
            .filter(production_recipe_addition::Column::IsDeleted.eq(false))
            .order_by_desc(production_recipe_addition::Column::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

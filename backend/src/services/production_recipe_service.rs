//! 大货处方与加料处方 Service
//!
//! v14 批次 424：大货处方与加料处方流程
//! 依据：面料行业真实业务调研文档 §11.2 大货处方（染色配料单）与加料处方（染色补料单）
//! 真实业务流程：
//!   大货处方单：扫描流转卡条码 → 依据备布数量 → 加载小样处方/历史大货处方 → 根据浴比/浴量
//!              → 填写物料明细 → 计算用量 → 开具大货处方单 → 审核后自动建立生产领用单据
//!   加料处方单：扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单
//!   关键约束：同一工单号只能开一张大货处方单，追加物料须开加料处方单
//!
//! 核心能力：
//! - 大货处方 CRUD + 状态流转（draft → approved → closed → cancelled）
//! - 用量计算（浓度% × 布重 × 浴比 / 100）
//! - 一工单一处方约束校验
//! - 加料处方 CRUD + 状态流转（draft → approved → closed）

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::production_recipe::{
    self, ActiveModel as RecipeActiveModel, Entity as RecipeEntity, Model as RecipeModel,
    RecipeMaterialItem,
};
use crate::models::production_recipe_addition::{
    self, ActiveModel as AdditionActiveModel, Entity as AdditionEntity, Model as AdditionModel,
};
use crate::models::status::production_recipe as recipe_status;
use crate::models::status::production_recipe_addition as addition_status;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

// ============================================================================
// 大货处方 Service
// ============================================================================

/// 创建大货处方请求
///
/// 真实业务必填字段（依据 §11.2 大货处方）：
/// - fabric_weight: 备布重量（用量计算依据）
/// - liquor_ratio: 浴比（如 1:8）
/// - recipe_detail: 处方明细（染料+助剂）
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductionRecipeRequest {
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub source_recipe_id: Option<i32>,
    pub lab_dip_resample_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    /// 备布重量 kg（必填，用量计算依据）
    pub fabric_weight: Decimal,
    pub equipment_no: Option<String>,
    /// 浴比如 1:8（必填）
    pub liquor_ratio: String,
    pub bath_volume: Option<Decimal>,
    pub adjustment_factor: Option<Decimal>,
    pub recipe_detail: Option<Vec<RecipeMaterialItem>>,
    pub total_dye_cost: Option<Decimal>,
    pub total_auxiliary_cost: Option<Decimal>,
    pub remarks: Option<String>,
    pub issued_by: Option<i32>,
    pub created_by: Option<i32>,
}

/// 更新大货处方请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProductionRecipeRequest {
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub source_recipe_id: Option<i32>,
    pub lab_dip_resample_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_spec: Option<String>,
    pub fabric_width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    pub fabric_weight: Option<Decimal>,
    pub equipment_no: Option<String>,
    pub liquor_ratio: Option<String>,
    pub bath_volume: Option<Decimal>,
    pub adjustment_factor: Option<Decimal>,
    pub recipe_detail: Option<Vec<RecipeMaterialItem>>,
    pub total_dye_cost: Option<Decimal>,
    pub total_auxiliary_cost: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 大货处方查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ProductionRecipeQuery {
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 审核请求
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveRecipeRequest {
    pub approved_by: i32,
}

/// 用量计算请求（按浓度+布重+浴比计算各物料用量）
#[derive(Debug, Clone, Deserialize)]
pub struct CalculateAmountsRequest {
    /// 备布重量 kg
    pub fabric_weight: Decimal,
    /// 浴比如 1:8
    pub liquor_ratio: String,
    /// 加成系数（默认 1.00）
    pub adjustment_factor: Option<Decimal>,
    /// 物料明细（需包含 concentration）
    pub items: Vec<RecipeMaterialItem>,
}

/// 大货处方 Service
pub struct ProductionRecipeService {
    db: Arc<DatabaseConnection>,
}

impl ProductionRecipeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成大货处方单号：PR-YYYYMMDDHHMMSS-NNN
    fn generate_recipe_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PR-{}-{:03}", timestamp, random)
    }

    /// 解析浴比字符串（如 "1:8"）为浴比数值（8.0）
    ///
    /// 真实业务：浴比格式为 "1:N"，N 通常为 5-20
    pub fn parse_liquor_ratio(ratio: &str) -> Result<Decimal, AppError> {
        let trimmed = ratio.trim();
        if trimmed.is_empty() {
            return Err(AppError::business("浴比不能为空"));
        }
        // 支持 "1:8" / "1：8"（全角冒号）/ "1/8" 三种格式
        // 一次遍历将全角冒号和斜杠统一为半角冒号，避免连续 str::replace 触发 clippy 警告
        let normalized: String = trimmed
            .chars()
            .map(|c| if c == '：' || c == '/' { ':' } else { c })
            .collect();
        let parts: Vec<&str> = normalized.split(':').collect();
        if parts.len() != 2 {
            return Err(AppError::business(format!(
                "浴比格式错误：{}（应为 1:N 格式，如 1:8）",
                ratio
            )));
        }
        let denominator = parts[1].trim().parse::<Decimal>().map_err(|_| {
            AppError::business(format!("浴比数值解析失败：{}", parts[1]))
        })?;
        if denominator <= Decimal::ZERO {
            return Err(AppError::business("浴比数值必须大于 0"));
        }
        Ok(denominator)
    }

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
        let model = self.get_by_id(id).await?;
        Self::validate_can_update(&model.status)?;

        // 若修改 work_order_id，需重新校验一工单一处方约束
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

        // 若修改备布重量，需 > 0
        if let Some(w) = req.fabric_weight {
            if w <= Decimal::ZERO {
                return Err(AppError::business("备布重量必须大于 0"));
            }
        }

        // 若修改浴比，需格式正确
        if let Some(ref r) = req.liquor_ratio {
            if r.trim().is_empty() {
                return Err(AppError::business("浴比不能为空"));
            }
            let _ = Self::parse_liquor_ratio(r)?;
        }

        let mut active: RecipeActiveModel = model.into();

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
        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.fabric_name {
            active.fabric_name = Set(Some(v));
        }
        if let Some(v) = req.fabric_spec {
            active.fabric_spec = Set(Some(v));
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
        if let Some(v) = req.equipment_no {
            active.equipment_no = Set(Some(v));
        }
        if let Some(v) = req.liquor_ratio {
            active.liquor_ratio = Set(v);
        }
        if let Some(v) = req.bath_volume {
            active.bath_volume = Set(Some(v));
        }
        if let Some(v) = req.adjustment_factor {
            active.adjustment_factor = Set(Some(v));
        }
        if let Some(v) = req.recipe_detail {
            active.recipe_detail = Set(Some(v));
        }
        if let Some(v) = req.total_dye_cost {
            active.total_dye_cost = Set(Some(v));
        }
        if let Some(v) = req.total_auxiliary_cost {
            active.total_auxiliary_cost = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除大货处方（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
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

    /// 审核大货处方（draft → approved）
    ///
    /// 真实业务：审核后自动建立生产领用单据（领用单据建立由下游模块消费 approved 事件）
    pub async fn approve(
        &self,
        id: i32,
        req: ApproveRecipeRequest,
    ) -> Result<RecipeModel, AppError> {
        let model = self.get_by_id(id).await?;
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
        let model = self.get_by_id(id).await?;
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
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, recipe_status::CANCELLED)?;

        let mut active: RecipeActiveModel = model.into();
        active.status = Set(recipe_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 计算用量（根据浓度+布重+浴比）
    ///
    /// 真实业务公式：用量 = 浓度% × 布重 × 浴比 / 100
    /// 其中浓度%为对布重百分比（owf%），浴比为 "1:N" 中的 N
    /// 加成系数用于修正小样→大货得色差异（默认 1.00）
    pub fn calculate_amounts(req: CalculateAmountsRequest) -> Result<Vec<RecipeMaterialItem>, AppError> {
        if req.fabric_weight <= Decimal::ZERO {
            return Err(AppError::business("备布重量必须大于 0"));
        }
        let ratio = Self::parse_liquor_ratio(&req.liquor_ratio)?;
        let factor = req.adjustment_factor.unwrap_or(Decimal::ONE);
        if factor <= Decimal::ZERO {
            return Err(AppError::business("加成系数必须大于 0"));
        }

        let hundred = Decimal::from(100);
        let mut result = Vec::with_capacity(req.items.len());
        for mut item in req.items {
            // 仅当浓度存在时才重新计算用量；助剂可能无浓度（直接给用量）
            if let Some(conc) = item.concentration {
                if conc < Decimal::ZERO {
                    return Err(AppError::business(format!(
                        "物料 {} 浓度不能为负",
                        item.material_code
                    )));
                }
                // 用量 = 浓度% × 布重 × 浴比 / 100 × 加成系数
                let amount = (conc * req.fabric_weight * ratio / hundred) * factor;
                item.amount = amount;
            }
            result.push(item);
        }
        Ok(result)
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

    // ===== 状态流转校验 =====

    /// 校验状态流转合法性
    ///
    /// 状态机：draft → approved → closed
    ///         draft → cancelled
    ///         approved → closed
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            recipe_status::DRAFT => matches!(new, recipe_status::APPROVED | recipe_status::CANCELLED),
            recipe_status::APPROVED => matches!(new, recipe_status::CLOSED),
            recipe_status::CLOSED => false,    // 终态
            recipe_status::CANCELLED => false, // 终态
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "大货处方状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }

    /// 校验：仅 draft 状态可更新
    pub fn validate_can_update(status: &str) -> Result<(), AppError> {
        if status != recipe_status::DRAFT {
            return Err(AppError::business(format!(
                "当前状态 {} 不可更新（仅 draft 可更新）",
                status
            )));
        }
        Ok(())
    }

    /// 校验：仅 draft 状态可删除
    pub fn validate_can_delete(status: &str) -> Result<(), AppError> {
        if status != recipe_status::DRAFT {
            return Err(AppError::business(format!(
                "当前状态 {} 不可删除（仅 draft 可删除）",
                status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 加料处方 Service
// ============================================================================

/// 创建加料处方请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductionRecipeAdditionRequest {
    /// 关联大货处方（必填）
    pub production_recipe_id: i32,
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    /// 加料原因：色差/助剂不足/工艺调整
    pub addition_reason: Option<String>,
    pub addition_detail: Option<Vec<crate::models::production_recipe_addition::AdditionMaterialItem>>,
    pub total_cost: Option<Decimal>,
    pub remarks: Option<String>,
    pub issued_by: Option<i32>,
    pub created_by: Option<i32>,
}

/// 加料处方查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ProductionRecipeAdditionQuery {
    pub production_recipe_id: Option<i32>,
    pub work_order_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 加料处方 Service
pub struct ProductionRecipeAdditionService {
    db: Arc<DatabaseConnection>,
}

impl ProductionRecipeAdditionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成加料处方单号：PA-YYYYMMDDHHMMSS-NNN
    fn generate_addition_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PA-{}-{:03}", timestamp, random)
    }

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
        let model = self.get_by_id(id).await?;
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
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, addition_status::CLOSED)?;

        let mut active: AdditionActiveModel = model.into();
        active.status = Set(addition_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    // ===== 状态流转校验 =====

    /// 校验状态流转合法性
    ///
    /// 状态机：draft → approved → closed
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            addition_status::DRAFT => matches!(new, addition_status::APPROVED),
            addition_status::APPROVED => matches!(new, addition_status::CLOSED),
            addition_status::CLOSED => false, // 终态
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "加料处方状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    /// 测试大货处方单号生成格式：PR-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_recipe_no() {
        let no = ProductionRecipeService::generate_recipe_no();
        assert!(no.starts_with("PR-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // 3 位随机
    }

    /// 测试加料处方单号生成格式：PA-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_addition_no() {
        let no = ProductionRecipeAdditionService::generate_addition_no();
        assert!(no.starts_with("PA-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14);
        assert_eq!(parts[2].len(), 3);
    }

    /// 测试浴比解析
    #[test]
    fn test_parse_liquor_ratio() {
        // 标准 "1:8" 格式
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1:8").unwrap(),
            Decimal::from(8)
        );
        // 全角冒号
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1：10").unwrap(),
            Decimal::from(10)
        );
        // 斜杠格式
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1/12").unwrap(),
            Decimal::from(12)
        );
        // 带空格
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio(" 1:8 ").unwrap(),
            Decimal::from(8)
        );
        // 非法格式
        assert!(ProductionRecipeService::parse_liquor_ratio("").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("abc").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:0").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:-5").is_err());
    }

    /// 测试用量计算
    ///
    /// 真实业务公式：用量 = 浓度% × 布重 × 浴比 / 100 × 加成系数
    #[test]
    fn test_calculate_amounts() {
        let fabric_weight = Decimal::from(100); // 100 kg
        let liquor_ratio = "1:8".to_string(); // 浴比 8
        let items = vec![RecipeMaterialItem {
            material_code: "D001".to_string(),
            material_name: "活性红".to_string(),
            concentration: Some(Decimal::from(2)), // 2% owf
            unit: "kg".to_string(),
            amount: Decimal::ZERO, // 待计算
            category: "dye".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: None,
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 用量 = 2 × 100 × 8 / 100 × 1 = 16 kg
        assert_eq!(result[0].amount, Decimal::from(16));
    }

    /// 测试用量计算（带加成系数）
    #[test]
    fn test_calculate_amounts_with_factor() {
        let fabric_weight = Decimal::from(200); // 200 kg
        let liquor_ratio = "1:10".to_string(); // 浴比 10
        let items = vec![RecipeMaterialItem {
            material_code: "D002".to_string(),
            material_name: "分散蓝".to_string(),
            concentration: Some(Decimal::from(3)), // 3% owf
            unit: "kg".to_string(),
            amount: Decimal::ZERO,
            category: "dye".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: Some(Decimal::from(150) / Decimal::from(100)), // 1.50 加成
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 用量 = 3 × 200 × 10 / 100 × 1.5 = 90 kg
        assert_eq!(result[0].amount, Decimal::from(90));
    }

    /// 测试用量计算（助剂无浓度，保留原用量）
    #[test]
    fn test_calculate_amounts_auxiliary_no_concentration() {
        let fabric_weight = Decimal::from(100);
        let liquor_ratio = "1:8".to_string();
        let original_amount = Decimal::from(5);
        let items = vec![RecipeMaterialItem {
            material_code: "A001".to_string(),
            material_name: "匀染剂".to_string(),
            concentration: None, // 助剂无浓度
            unit: "kg".to_string(),
            amount: original_amount,
            category: "auxiliary".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: None,
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 无浓度不重算，保留原用量
        assert_eq!(result[0].amount, original_amount);
    }

    /// 测试用量计算非法输入
    #[test]
    fn test_calculate_amounts_invalid() {
        // 备布重量 <= 0
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::ZERO,
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: None,
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 浴比格式错误
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "abc".to_string(),
            adjustment_factor: None,
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 加成系数 <= 0
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: Some(Decimal::ZERO),
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 浓度为负
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: None,
            items: vec![RecipeMaterialItem {
                material_code: "D001".to_string(),
                material_name: "活性红".to_string(),
                concentration: Some(Decimal::from(-1)),
                unit: "kg".to_string(),
                amount: Decimal::ZERO,
                category: "dye".to_string(),
            }],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());
    }

    /// 测试大货处方状态流转合法性
    #[test]
    fn test_recipe_status_transition_valid() {
        // 合法流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::APPROVED
        )
        .is_ok());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::CANCELLED
        )
        .is_ok());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::CLOSED
        )
        .is_ok());
    }

    /// 测试大货处方状态流转非法
    #[test]
    fn test_recipe_status_transition_invalid() {
        // 非法流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::CLOSED
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::DRAFT
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::CANCELLED
        )
        .is_err());
        // 终态不可流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::CLOSED,
            recipe_status::APPROVED
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::CANCELLED,
            recipe_status::DRAFT
        )
        .is_err());
    }

    /// 测试大货处方更新/删除状态校验
    #[test]
    fn test_recipe_validate_can_update_and_delete() {
        // 仅 draft 可更新
        assert!(ProductionRecipeService::validate_can_update(recipe_status::DRAFT).is_ok());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::APPROVED).is_err());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::CLOSED).is_err());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::CANCELLED).is_err());

        // 仅 draft 可删除
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::DRAFT).is_ok());
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::APPROVED).is_err());
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::CLOSED).is_err());
    }

    /// 测试加料处方状态流转
    #[test]
    fn test_addition_status_transition() {
        // 合法流转
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::DRAFT,
            addition_status::APPROVED
        )
        .is_ok());
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::APPROVED,
            addition_status::CLOSED
        )
        .is_ok());

        // 非法流转
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::DRAFT,
            addition_status::CLOSED
        )
        .is_err());
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::APPROVED,
            addition_status::DRAFT
        )
        .is_err());
        // 终态
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::CLOSED,
            addition_status::APPROVED
        )
        .is_err());
    }

    /// 测试 FromPrimitive trait 可用（确保 rust_decimal::prelude::FromPrimitive 引入正确）
    #[test]
    fn test_decimal_from_f64() {
        let d = Decimal::from_f64(1.5).unwrap();
        assert_eq!(d, Decimal::from(15) / Decimal::from(10));
    }
}

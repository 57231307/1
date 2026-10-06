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
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::dye_recipe::{
    self, ActiveModel, Entity as DyeRecipeEntity, Model as DyeRecipeModel,
};
use crate::models::status::dye_recipe as recipe_status;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

/// 配方编号（dye_recipe.recipe_no）自动编码前缀：沿用原手写格式
/// "DR-{时间戳}-{随机}" 的业务前缀 DR，新格式统一为 {DR}{YYYYMMDD}{3位流水}。
pub const DYE_RECIPE_NO_PREFIX: &str = "DR";

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时直接调 visit_none()，
/// 把"显式 null"塌成外层 `None`，与"键缺席"不可区分。本适配器先按内层 `Option<T>` 反序列化
/// 再包一层：键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 形态与 handlers/department_handler.rs 中同名私有适配器一致（跨域合并到共享 utils 超本波授权范围）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 创建染色配方请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDyeRecipeRequest {
    pub recipe_no: Option<String>,
    pub recipe_name: Option<String>,
    pub color_no: Option<String>,
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
///
/// 三态语义（RFC 7386，对齐 handlers/department_handler.rs 范式）：
/// 键缺席=保持原值、显式 null=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// 本域可空列依据：dye_recipe 补列全部为可空 ALTER
/// （color_no system/mod.rs:118、color_name :117、fabric_type :120、dye_type :119、
/// chemical_formula :116、temperature :129、time_minutes :130、ph_value :125、
/// liquor_ratio :123、auxiliaries :115、status :128、remarks :127）。
/// color_code 例外：建表即 NOT NULL（system/m0003_add_dye_tables.rs:30），
/// 显式 null 由 service 入口在任何 DB 访问前拒绝。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateDyeRecipeRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub color_no: Option<Option<String>>,
    /// 色代码：DB NOT NULL（m0003:30）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub color_code: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub color_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub fabric_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub dye_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub chemical_formula: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub temperature: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub time_minutes: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub ph_value: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub liquor_ratio: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub auxiliaries: Option<Option<Vec<crate::models::dye_recipe::AuxiliariesItem>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub status: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
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

    /// 校验配方状态流转是否合法
    /// （批次 423B 引入"待审核"中间态：草稿 → 待审核 → 已审核 / 已停用；支持待审核撤回草稿；
    /// 保留草稿直审兼容路径）
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            recipe_status::DRAFT => matches!(
                new,
                recipe_status::PENDING_APPROVAL | recipe_status::APPROVED | recipe_status::DISABLED
            ),
            recipe_status::PENDING_APPROVAL => matches!(
                new,
                recipe_status::APPROVED | recipe_status::DISABLED | recipe_status::DRAFT
            ),
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
            return Err(AppError::business("已审核的配方不允许删除，请先停用"));
        }
        Ok(())
    }

    /// 校验配方是否允许审核（草稿或待审核状态均可审核）
    pub fn validate_can_approve(status: Option<&str>) -> Result<(), AppError> {
        if status != Some(recipe_status::DRAFT) && status != Some(recipe_status::PENDING_APPROVAL) {
            return Err(AppError::business(format!(
                "只有草稿或待审核状态的配方可以审核，当前状态：{}",
                status.unwrap_or("未知")
            )));
        }
        Ok(())
    }

    /// 提交配方审核（批次 423B：草稿 → 待审核，贯通化验室打样审批流）
    pub async fn submit(&self, id: i32) -> Result<DyeRecipeModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status.as_deref() != Some(recipe_status::DRAFT) {
            return Err(AppError::business(format!(
                "只有草稿状态的配方可以提交审核，当前状态：{}",
                model.status.as_deref().unwrap_or("未知")
            )));
        }

        let mut active: ActiveModel = model.into();
        active.status = Set(Some(recipe_status::PENDING_APPROVAL.to_string()));
        // 清除历史提交产生的 approved_by=-1 占位（批次 423B 前遗留）
        if active.approved_by.is_set() {
            active.approved_by = NotSet;
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
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
        // 取号与 INSERT 同事务：调用方显式提供非空编号时尊重手工值（既有契约）；
        // 否则经通用生成器在事务内
        // 取 {DR}{YYYYMMDD}{3位流水}。
        // 为什么不再用旧的 "DR-{14位时间戳}-{4位随机}"：同秒并发碰撞概率非零，且
        // recipe_no 是配方业务识别与检索键（list 按前缀 contains 过滤、版本号按
        // "{recipe_no}-V{n}" 派生）。DDL 事实：recipe_no 为
        // migration/src/domain/system/mod.rs:126 ALTER 追加的 VARCHAR(255) 列、
        // 无 UNIQUE 约束，唯一性由生成器 advisory 锁 + 事务内取号 + 占用探测保证
        //（参照 services/quotation_ops/lifecycle.rs:54 的事务内取号路径）。
        let provided_no = req
            .recipe_no
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let txn = (*self.db).begin().await?;
        let recipe_no = match provided_no {
            Some(no) => no,
            None => DocumentNumberGenerator::generate_no_with_txn(
                &txn,
                DYE_RECIPE_NO_PREFIX,
                DyeRecipeEntity,
                dye_recipe::Column::RecipeNo,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    prefix = DYE_RECIPE_NO_PREFIX,
                    "配方号取号失败（dye_recipe_service.create）"
                );
                AppError::business_displayable("配方号生成失败，请稍后重试")
            })?,
        };

        let active = ActiveModel {
            id: Default::default(),
            recipe_no: Set(recipe_no),
            recipe_name: Set(Some(
                req.recipe_name.unwrap_or_else(|| "未命名配方".to_string()),
            )),
            color_no: Set(req.color_no.clone()),
            formula: Set(req.chemical_formula.clone()),
            color_code: Set(Some(
                req.color_code
                    .clone()
                    .or_else(|| req.color_no.clone())
                    .unwrap_or_default(),
            )),
            color_name: Set(req.color_name),
            fabric_type: Set(req.fabric_type),
            dye_type: Set(req.dye_type),
            chemical_formula: Set(req.chemical_formula),
            temperature: Set(req.temperature),
            time_minutes: Set(req.time_minutes),
            ph_value: Set(req.ph_value),
            liquor_ratio: Set(req.liquor_ratio),
            auxiliaries: Set(req.auxiliaries.map(crate::models::dye_recipe::Auxiliaries)),
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
            .insert(&txn)
            .await
            .map_err(|e| AppError::database(format!("配方创建失败: {}", e)))?;
        txn.commit().await?;
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
    ///
    /// 三态写入（RFC 7386，对齐 department_service::update）：
    /// None=不 Set、Some(None)=Set(None) 置 NULL（仅 DB 可空列）、Some(Some(v))=Set(v) 覆盖；
    /// NOT NULL 列 color_code（m0003:30）的显式 null 在任何 DB 访问前拒绝（外显不脱敏）。
    pub async fn update(
        &self,
        id: i32,
        req: UpdateDyeRecipeRequest,
    ) -> Result<DyeRecipeModel, AppError> {
        if matches!(req.color_code, Some(None)) {
            return Err(AppError::business_displayable(
                "色代码不能清空：该字段为必填项",
            ));
        }

        let model = self.get_by_id(id).await?;
        // 在转为 ActiveModel 前记录当前状态，用于状态流转校验
        // 注意：必须 clone 后再 model.into()，否则 as_deref() 借用 model.status 会与
        // 后续 model.into() 移动 model 冲突（E0505 cannot move out of borrowed）
        let current_status = model
            .status
            .clone()
            .unwrap_or_else(|| recipe_status::DRAFT.to_string());
        let mut active: ActiveModel = model.into();

        // DB 可空列：Some(None)=Set(None) 清空、Some(Some(v))=Set(Some(v)) 覆盖
        if let Some(v) = req.color_no {
            active.color_no = Set(v);
        }
        // color_code：NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(v) = req.color_code.flatten() {
            active.color_code = Set(Some(v));
        }
        if let Some(v) = req.color_name {
            active.color_name = Set(v);
        }
        if let Some(v) = req.fabric_type {
            active.fabric_type = Set(v);
        }
        if let Some(v) = req.dye_type {
            active.dye_type = Set(v);
        }
        if let Some(v) = req.chemical_formula {
            active.chemical_formula = Set(v);
        }
        if let Some(v) = req.temperature {
            active.temperature = Set(v);
        }
        if let Some(v) = req.time_minutes {
            active.time_minutes = Set(v);
        }
        if let Some(v) = req.ph_value {
            active.ph_value = Set(v);
        }
        if let Some(v) = req.liquor_ratio {
            active.liquor_ratio = Set(v);
        }
        if let Some(v) = req.auxiliaries {
            active.auxiliaries = Set(v.map(crate::models::dye_recipe::Auxiliaries));
        }
        // status：DB 可空列（system/mod.rs:128 补列无 NOT NULL；CHECK chk_dye_recipe_status
        // 对 NULL 恒成立）——Some(None)=Set(None) 清空回"未定状态"，覆盖时校验状态流转
        if let Some(v) = req.status {
            if let Some(v) = v.as_ref() {
                Self::validate_status_transition(&current_status, v)?;
            }
            active.status = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(v);
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
    /// created_by 为建版人，由 handler 从服务端会话取（AuthContext.user_id），
    /// 不接受请求体身份，故签名是必填 i32 而非 Option。
    pub async fn create_new_version(
        &self,
        id: i32,
        remarks: Option<String>,
        created_by: i32,
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
            created_by: Set(Some(created_by)),
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

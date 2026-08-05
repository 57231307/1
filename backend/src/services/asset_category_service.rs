use crate::models::asset_category;
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use tracing::info;

/// 资产分类查询参数
#[derive(Debug, Clone, Default)]
pub struct AssetCategoryQueryParams {
    pub is_active: Option<bool>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建资产分类请求
#[derive(Debug, Clone)]
pub struct CreateAssetCategoryRequest {
    pub category_code: String,
    pub category_name: String,
    pub parent_id: Option<i32>,
    pub default_useful_life: Option<i32>,
    pub default_depreciation_method: Option<String>,
    pub default_salvage_rate: Option<rust_decimal::Decimal>,
    pub description: Option<String>,
}

/// 更新资产分类请求
#[derive(Debug, Clone)]
pub struct UpdateAssetCategoryRequest {
    pub category_name: Option<String>,
    pub parent_id: Option<Option<i32>>,
    pub default_useful_life: Option<Option<i32>>,
    pub default_depreciation_method: Option<Option<String>>,
    pub default_salvage_rate: Option<Option<rust_decimal::Decimal>>,
    pub description: Option<Option<String>>,
    pub is_active: Option<bool>,
}

pub struct AssetCategoryService {
    db: Arc<DatabaseConnection>,
}

impl AssetCategoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建资产分类
    pub async fn create(
        &self,
        req: CreateAssetCategoryRequest,
        user_id: i32,
    ) -> Result<asset_category::Model, AppError> {
        let active = asset_category::ActiveModel {
            id: Default::default(),
            category_code: Set(req.category_code),
            category_name: Set(req.category_name),
            parent_id: Set(req.parent_id),
            default_useful_life: Set(req.default_useful_life),
            default_depreciation_method: Set(req.default_depreciation_method),
            default_salvage_rate: Set(req.default_salvage_rate),
            description: Set(req.description),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active.insert(&*self.db).await?;
        info!("资产分类创建成功：ID {}", result.id);
        Ok(result)
    }

    /// 查询资产分类列表
    pub async fn list(
        &self,
        params: AssetCategoryQueryParams,
    ) -> Result<(Vec<asset_category::Model>, u64), AppError> {
        let mut query = asset_category::Entity::find();

        if let Some(is_active) = params.is_active {
            query = query.filter(asset_category::Column::IsActive.eq(is_active));
        }

        let total = query.clone().count(&*self.db).await?;
        let categories = query
            .order_by(asset_category::Column::CategoryCode, Order::Asc)
            .paginate(&*self.db, params.page_size as u64)
            .fetch_page(params.page.max(1) as u64 - 1)
            .await?;

        Ok((categories, total))
    }

    /// 获取资产分类详情
    pub async fn get_by_id(&self, id: i32) -> Result<asset_category::Model, AppError> {
        asset_category::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("资产分类不存在：{}", id)))
    }

    /// 更新资产分类
    pub async fn update(
        &self,
        id: i32,
        req: UpdateAssetCategoryRequest,
    ) -> Result<asset_category::Model, AppError> {
        let existing = self.get_by_id(id).await?;
        let mut active: asset_category::ActiveModel = existing.into();

        if let Some(v) = req.category_name {
            active.category_name = Set(v);
        }
        if let Some(v) = req.parent_id {
            active.parent_id = Set(v);
        }
        if let Some(v) = req.default_useful_life {
            active.default_useful_life = Set(v);
        }
        if let Some(v) = req.default_depreciation_method {
            active.default_depreciation_method = Set(v);
        }
        if let Some(v) = req.default_salvage_rate {
            active.default_salvage_rate = Set(v);
        }
        if let Some(v) = req.description {
            active.description = Set(v);
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 删除资产分类（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let existing = self.get_by_id(id).await?;
        let mut active: asset_category::ActiveModel = existing.into();
        active.is_active = Set(false);
        active.updated_at = Set(Utc::now());
        active.update(&*self.db).await?;
        info!("资产分类已停用：ID {}", id);
        Ok(())
    }
}

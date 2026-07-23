//! 染化料分类 Service impl 子模块（chemical_ops/category）
//!
//! 批次 490 D10-3a 拆分：从原 `chemical_service.rs` L738-909 迁移。
//! 包含 ChemicalCategoryService 的 6 个方法：
//! - create / update / delete（CRUD）
//! - get_by_id / list / get_tree（查询）
//!
//! 业务规则：
//! - 创建时校验类型合法、父分类存在、编码唯一
//! - 更新时校验类型合法、禁止将自身设为父分类
//! - 删除时校验无子分类
//! - 软删除（is_deleted = true）
//! - get_tree 按 parent_id 查询启用的子分类树

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::chemical_category::{
    self, ActiveModel as CategoryActiveModel, Entity as CategoryEntity, Model as CategoryModel,
};
use crate::utils::error::AppError;

use crate::services::chemical_ops::types::{
    ChemicalCategoryQuery, CreateChemicalCategoryRequest, UpdateChemicalCategoryRequest,
};
use crate::services::chemical_service::{validate_chemical_type, ChemicalCategoryService};

impl ChemicalCategoryService {
    /// 创建染化料分类
    pub async fn create(&self, req: CreateChemicalCategoryRequest) -> Result<CategoryModel, AppError> {
        validate_chemical_type(&req.category_type)?;

        // 校验父分类存在（若提供）
        if let Some(parent_id) = req.parent_id {
            if CategoryEntity::find_by_id(parent_id)
                .filter(chemical_category::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("父分类 {} 不存在", parent_id)));
            }
        }

        // 校验编码唯一性
        if let Some(_existing) = CategoryEntity::find()
            .filter(chemical_category::Column::CategoryCode.eq(&req.category_code))
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "分类编码 {} 已存在",
                req.category_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let active = CategoryActiveModel {
            id: Default::default(),
            category_code: Set(req.category_code),
            category_name: Set(req.category_name),
            parent_id: Set(req.parent_id),
            category_type: Set(req.category_type),
            description: Set(req.description),
            sort_order: Set(req.sort_order.unwrap_or(0)),
            is_active: Set(true),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染化料分类创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新染化料分类
    pub async fn update(
        &self,
        id: i32,
        req: UpdateChemicalCategoryRequest,
    ) -> Result<CategoryModel, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: CategoryActiveModel = model.into();

        if let Some(v) = req.category_name {
            active.category_name = Set(v);
        }
        if let Some(v) = req.parent_id {
            // 禁止将自身设为父分类
            if v == id {
                return Err(AppError::business("不能将自身设为父分类"));
            }
            active.parent_id = Set(Some(v));
        }
        if let Some(v) = req.category_type {
            validate_chemical_type(&v)?;
            active.category_type = Set(v);
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = req.sort_order {
            active.sort_order = Set(v);
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除染化料分类
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        // 校验是否有子分类
        let children_count = CategoryEntity::find()
            .filter(chemical_category::Column::ParentId.eq(id))
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await?;
        if children_count > 0 {
            return Err(AppError::business("存在子分类，无法删除"));
        }

        let model = self.get_by_id(id).await?;
        let mut active: CategoryActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<CategoryModel, AppError> {
        CategoryEntity::find_by_id(id)
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("染化料分类 {} 不存在", id)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: ChemicalCategoryQuery,
    ) -> Result<(Vec<CategoryModel>, u64), AppError> {
        let mut q = CategoryEntity::find()
            .filter(chemical_category::Column::IsDeleted.eq(false));
        if let Some(v) = query.parent_id {
            q = q.filter(chemical_category::Column::ParentId.eq(v));
        }
        if let Some(v) = query.category_type {
            q = q.filter(chemical_category::Column::CategoryType.eq(v));
        }
        if let Some(v) = query.is_active {
            q = q.filter(chemical_category::Column::IsActive.eq(v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_asc(chemical_category::Column::SortOrder)
            .order_by_desc(chemical_category::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 查询分类树（按 parent_id 查询子分类）
    pub async fn get_tree(&self, parent_id: Option<i32>) -> Result<Vec<CategoryModel>, AppError> {
        let mut q = CategoryEntity::find()
            .filter(chemical_category::Column::IsDeleted.eq(false))
            .filter(chemical_category::Column::IsActive.eq(true));
        if let Some(pid) = parent_id {
            q = q.filter(chemical_category::Column::ParentId.eq(pid));
        } else {
            q = q.filter(chemical_category::Column::ParentId.is_null());
        }
        let items = q
            .order_by_asc(chemical_category::Column::SortOrder)
            .order_by_desc(chemical_category::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

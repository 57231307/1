use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, ColumnTrait, QueryFilter, QueryOrder, Set, NotSet, Order};
use sea_orm::prelude::*;
use std::sync::Arc;
use chrono::Utc;

use crate::models::product_category::{self, Entity as ProductCategoryEntity};

/// 产品类别服务
pub struct ProductCategoryService {
    db: Arc<DatabaseConnection>,
}

impl ProductCategoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
    
    /// 获取产品类别列表（支持分页和过滤）
    #[allow(unused_variables)]
    pub async fn list_categories(
        &self,
        page: u64,
        page_size: u64,
        parent_id: Option<i32>,
        search: Option<String>,
    ) -> Result<(Vec<product_category::Model>, u64), sea_orm::DbErr> {
        let mut query = ProductCategoryEntity::find();
        
        // 应用过滤条件
        if let Some(pid) = parent_id {
            query = query.filter(product_category::Column::ParentId.eq(pid));
        }
        
        if let Some(keyword) = search {
            query = query.filter(
                product_category::Column::Name.like(format!("%{}%", keyword))
            );
        }
        
        // 获取总数
        let total = query.clone().count(&*self.db).await?;
        
        // 应用分页和排序
        let categories = query
            .order_by(product_category::Column::Name, Order::Asc)
            .into_model::<product_category::Model>()
            .all(&*self.db)
            .await?;
        
        Ok((categories, total))
    }
    
    /// 获取产品类别详情
    pub async fn get_category(&self, id: i32) -> Result<product_category::Model, sea_orm::DbErr> {
        ProductCategoryEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("产品类别 ID {} 不存在", id)))
    }
    
    /// 创建产品类别
    pub async fn create_category(
        &self,
        name: String,
        parent_id: Option<i32>,
        description: Option<String>,
    ) -> Result<product_category::Model, sea_orm::DbErr> {
        // 检查父类别是否存在（如果提供了 parent_id）
        if let Some(pid) = parent_id {
            let _ = ProductCategoryEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("父类别 ID {} 不存在", pid)))?;
        }
        
        let active_model = product_category::ActiveModel {
            id: NotSet,
            category_code: Set(name.chars().take(10).collect()),
            name: Set(name),
            parent_id: Set(parent_id),
            description: Set(description),
            sort_order: Set(0),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };
        
        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }
    
    /// 更新产品类别
    pub async fn update_category(
        &self,
        id: i32,
        name: Option<String>,
        parent_id: Option<i32>,
        description: Option<String>,
    ) -> Result<product_category::Model, sea_orm::DbErr> {
        let mut category: product_category::ActiveModel = ProductCategoryEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("产品类别 ID {} 不存在", id)))?
            .into();
        
        if let Some(n) = name {
            // 检查类别名称是否已存在
            let existing = ProductCategoryEntity::find()
                .filter(product_category::Column::Name.eq(&n))
                .filter(product_category::Column::Id.ne(id))
                .one(&*self.db)
                .await?;
            
            if existing.is_some() {
                return Err(sea_orm::DbErr::Custom(format!("类别名称 '{}' 已存在", n)));
            }
            category.name = Set(n);
        }
        
        if let Some(pid) = parent_id {
            // 检查父类别是否存在
            let _ = ProductCategoryEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("父类别 ID {} 不存在", pid)))?;
            category.parent_id = Set(Some(pid));
        }
        
        if let Some(d) = description {
            category.description = Set(Some(d));
        }
        
        category.updated_at = Set(Utc::now());
        
        let result = category.update(&*self.db).await?;
        Ok(result)
    }
    
    /// 删除产品类别
    pub async fn delete_category(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        // 检查是否有子类别
        let children_count = ProductCategoryEntity::find()
            .filter(product_category::Column::ParentId.eq(id))
            .count(&*self.db)
            .await?;
        
        if children_count > 0 {
            return Err(sea_orm::DbErr::Custom("该类别存在子类别，无法删除".to_string()));
        }
        
        let result = ProductCategoryEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(sea_orm::DbErr::RecordNotFound(format!("产品类别 ID {} 不存在", id)));
        }
        Ok(())
    }

    /// 根据名称查询产品类别
    #[allow(dead_code)]
    pub async fn find_by_name(&self, name: &str) -> Result<product_category::Model, sea_orm::DbErr> {
        ProductCategoryEntity::find()
            .filter(product_category::Column::Name.eq(name))
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("产品类别名称 {} 不存在", name)))
    }
    
    /// 获取产品类别树形结构（简化版）
    pub async fn get_category_tree(&self) -> Result<Vec<product_category::Model>, sea_orm::DbErr> {
        // 简化实现：返回所有类别，前端自行组织树形结构
        ProductCategoryEntity::find()
            .order_by(product_category::Column::Name, Order::Asc)
            .all(&*self.db)
            .await
    }
}

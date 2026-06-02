#![allow(dead_code)]

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::product_category::{self, Entity as ProductCategoryEntity};
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;

/// 产品类别服务
pub struct ProductCategoryService {
    db: Arc<DatabaseConnection>,
}

impl ProductCategoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取产品类别列表（支持分页和过滤）
    pub async fn list(
        &self,
        query: crate::handlers::product_category_handler::ProductCategoryListQuery,
    ) -> Result<crate::utils::response::PaginatedResponse<product_category::Model>, AppError> {
        let mut q = ProductCategoryEntity::find();

        // 应用过滤条件
        if let Some(pid) = query.parent_id {
            q = q.filter(product_category::Column::ParentId.eq(pid));
        }

        if let Some(keyword) = query.search {
            let pattern = safe_like_pattern(&keyword);
            q = q.filter(product_category::Column::Name.like(&pattern));
        }

        // 获取总数
        let total = q.clone().count(&*self.db).await?;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        // 应用分页和排序
        let categories = q
            .order_by(product_category::Column::Name, Order::Asc)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .into_model::<product_category::Model>()
            .all(&*self.db)
            .await?;

        Ok(crate::utils::response::PaginatedResponse::new(
            categories, total, page, page_size,
        ))
    }

    /// 获取产品类别详情
    pub async fn get(&self, id: i32) -> Result<product_category::Model, AppError> {
        ProductCategoryEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("产品类别 ID {} 不存在", id)))
    }

    /// 创建产品类别
    pub async fn create(
        &self,
        req: crate::handlers::product_category_handler::CreateProductCategoryRequest,
    ) -> Result<product_category::Model, AppError> {
        // 检查父类别是否存在（如果提供了 parent_id）
        if let Some(pid) = req.parent_id {
            let _ = ProductCategoryEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("父类别 ID {} 不存在", pid)))?;
        }

        let active_model = product_category::ActiveModel {
            id: NotSet,
            name: Set(req.name),
            parent_id: Set(req.parent_id),
            description: Set(req.description),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新产品类别
    pub async fn update(
        &self,
        id: i32,
        req: crate::handlers::product_category_handler::UpdateProductCategoryRequest,
    ) -> Result<product_category::Model, AppError> {
        let mut category: product_category::ActiveModel = ProductCategoryEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("产品类别 ID {} 不存在", id)))?
            .into();

        if let Some(n) = req.name {
            // 检查类别名称是否已存在
            let existing = ProductCategoryEntity::find()
                .filter(product_category::Column::Name.eq(&n))
                .filter(product_category::Column::Id.ne(id))
                .one(&*self.db)
                .await?;

            if existing.is_some() {
                return Err(AppError::BusinessError(format!("类别名称 '{}' 已存在", n)));
            }
            category.name = Set(n);
        }

        if let Some(pid) = req.parent_id {
            // 检查父类别是否存在
            let _ = ProductCategoryEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("父类别 ID {} 不存在", pid)))?;
            category.parent_id = Set(Some(pid));
        }

        if let Some(d) = req.description {
            category.description = Set(Some(d));
        }

        category.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            category,
            Some(0),
        )
        .await?;
        Ok(result)
    }

    /// 删除产品类别
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        // 检查是否有子类别
        let children_count = ProductCategoryEntity::find()
            .filter(product_category::Column::ParentId.eq(id))
            .count(&*self.db)
            .await?;

        if children_count > 0 {
            return Err(AppError::BusinessError(
                "该类别存在子类别，无法删除".to_string(),
            ));
        }

        let result = ProductCategoryEntity::delete_by_id(id)
            .exec(&*self.db)
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound(format!("产品类别 ID {} 不存在", id)));
        }
        Ok(())
    }

    /// 根据名称查询产品类别
    pub async fn find_by_name(&self, name: &str) -> Result<product_category::Model, AppError> {
        ProductCategoryEntity::find()
            .filter(product_category::Column::Name.eq(name))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("产品类别名称 {} 不存在", name)))
    }

    /// 获取产品类别树形结构
    pub async fn get_category_tree(&self) -> Result<Vec<CategoryTreeNode>, AppError> {
        // 查询所有类别
        let all_categories = ProductCategoryEntity::find()
            .order_by(product_category::Column::Name, Order::Asc)
            .all(&*self.db)
            .await?;

        // 构建树形结构
        let mut root_nodes = Vec::new();
        let mut children_map: std::collections::HashMap<i32, Vec<CategoryTreeNode>> =
            std::collections::HashMap::new();

        // 首先创建所有节点并按parent_id分组
        for cat in &all_categories {
            let node = CategoryTreeNode {
                id: cat.id,
                name: cat.name.clone(),
                parent_id: cat.parent_id,
                description: cat.description.clone(),
                children: Vec::new(),
            };

            if let Some(parent_id) = cat.parent_id {
                children_map.entry(parent_id).or_default().push(node);
            } else {
                root_nodes.push(node);
            }
        }

        // 递归构建子树
        fn build_children(
            node: &mut CategoryTreeNode,
            children_map: &mut std::collections::HashMap<i32, Vec<CategoryTreeNode>>,
        ) {
            if let Some(mut children) = children_map.remove(&node.id) {
                for child in &mut children {
                    build_children(child, children_map);
                }
                node.children = children;
            }
        }

        for node in &mut root_nodes {
            build_children(node, &mut children_map);
        }

        Ok(root_nodes)
    }
}

/// 产品类别树节点
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryTreeNode {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
    pub children: Vec<CategoryTreeNode>,
}

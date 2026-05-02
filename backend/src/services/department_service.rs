use chrono::Utc;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order, QueryFilter,
    QueryOrder, Set,
};
use serde::Serialize;
use std::sync::Arc;

use crate::models::department::{self, Entity as DepartmentEntity};

/// 部门树节点（用于返回树形结构）
#[derive(Debug, Serialize, Clone)]
pub struct DepartmentTreeNode {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub children: Vec<DepartmentTreeNode>,
}

/// 部门服务
pub struct DepartmentService {
    db: Arc<DatabaseConnection>,
}

impl DepartmentService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取部门列表（支持分页和过滤）
    pub async fn list(
        &self,
        query: crate::handlers::department_handler::DepartmentListQuery,
    ) -> Result<crate::utils::response::PaginatedResponse<department::Model>, sea_orm::DbErr> {
        let mut q = DepartmentEntity::find();

        // 应用过滤条件
        if let Some(pid) = query.parent_id {
            q = q.filter(department::Column::ParentId.eq(pid));
        }

        if let Some(keyword) = query.search {
            q = q.filter(
                department::Column::Name
                    .like(format!("%{}%", keyword))
                    .or(department::Column::Description.like(format!("%{}%", keyword))),
            );
        }

        // 获取总数
        let total = q.clone().count(&*self.db).await?;
        
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        // 应用分页和排序
        let departments = q
            .order_by(department::Column::Name, Order::Asc)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .into_model::<department::Model>()
            .all(&*self.db)
            .await?;

        Ok(crate::utils::response::PaginatedResponse::new(departments, total, page, page_size))
    }

    /// 获取部门详情
    pub async fn get(&self, id: i32) -> Result<department::Model, sea_orm::DbErr> {
        DepartmentEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("部门 ID {} 不存在", id)))
    }

    /// 创建部门
    pub async fn create(
        &self,
        req: crate::handlers::department_handler::CreateDepartmentRequest,
    ) -> Result<department::Model, sea_orm::DbErr> {
        // 检查父部门是否存在（如果提供了 parent_id）
        if let Some(pid) = req.parent_id {
            let _ = DepartmentEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| {
                    sea_orm::DbErr::RecordNotFound(format!("父部门 ID {} 不存在", pid))
                })?;
        }

        let active_model = department::ActiveModel {
            id: NotSet,
            code: Set(req.name.chars().take(10).collect()),
            name: Set(req.name),
            parent_id: Set(req.parent_id),
            manager_id: Set(None),
            description: Set(req.description),
            sort_order: Set(0),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新部门
    pub async fn update(
        &self,
        id: i32,
        req: crate::handlers::department_handler::UpdateDepartmentRequest,
    ) -> Result<department::Model, sea_orm::DbErr> {
        let mut dept: department::ActiveModel = DepartmentEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("部门 ID {} 不存在", id)))?
            .into();

        if let Some(n) = req.name {
            // 检查部门名称是否已存在
            let existing = DepartmentEntity::find()
                .filter(department::Column::Name.eq(&n))
                .filter(department::Column::Id.ne(id))
                .one(&*self.db)
                .await?;

            if existing.is_some() {
                return Err(sea_orm::DbErr::Custom(format!("部门名称 '{}' 已存在", n)));
            }
            dept.name = Set(n);
        }

        if let Some(d) = req.description {
            dept.description = Set(Some(d));
        }

        if let Some(pid) = req.parent_id {
            // 检查父部门是否存在
            let _ = DepartmentEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| {
                    sea_orm::DbErr::RecordNotFound(format!("父部门 ID {} 不存在", pid))
                })?;
            dept.parent_id = Set(Some(pid));
        }

        dept.updated_at = Set(Utc::now());

        let result = dept.update(&*self.db).await?;
        Ok(result)
    }

    /// 删除部门
    pub async fn delete(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        // 检查是否有子部门
        let children_count = DepartmentEntity::find()
            .filter(department::Column::ParentId.eq(id))
            .count(&*self.db)
            .await?;

        if children_count > 0 {
            return Err(sea_orm::DbErr::Custom(
                "该部门存在子部门，无法删除".to_string(),
            ));
        }

        let result = DepartmentEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(sea_orm::DbErr::RecordNotFound(format!(
                "部门 ID {} 不存在",
                id
            )));
        }
        Ok(())
    }

    /// 获取部门树形结构
    pub async fn get_department_tree(&self) -> Result<Vec<DepartmentTreeNode>, sea_orm::DbErr> {
        let all_departments = DepartmentEntity::find()
            .order_by(department::Column::Name, Order::Asc)
            .all(&*self.db)
            .await?;

        // 构建部门树
        let mut tree: Vec<DepartmentTreeNode> = Vec::new();
        let mut dept_map: std::collections::HashMap<i32, DepartmentTreeNode> =
            std::collections::HashMap::new();

        // 先创建所有节点
        for dept in all_departments {
            dept_map.insert(
                dept.id,
                DepartmentTreeNode {
                    id: dept.id,
                    name: dept.name,
                    description: dept.description,
                    parent_id: dept.parent_id,
                    children: Vec::new(),
                },
            );
        }

        // 构建树形结构
        let dept_ids: Vec<i32> = dept_map.keys().copied().collect();
        for id in dept_ids {
            if let Some(node) = dept_map.get(&id).cloned() {
                if let Some(parent_id) = node.parent_id {
                    if let Some(parent_node) = dept_map.get_mut(&parent_id) {
                        parent_node.children.push(node);
                    }
                } else {
                    tree.push(node);
                }
            }
        }

        Ok(tree)
    }

    /// 根据名称查询部门
    pub async fn find_by_name(&self, name: &str) -> Result<department::Model, sea_orm::DbErr> {
        DepartmentEntity::find()
            .filter(department::Column::Name.eq(name))
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("部门名称 {} 不存在", name)))
    }
}

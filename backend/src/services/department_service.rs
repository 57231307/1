use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, ColumnTrait, QueryFilter, QueryOrder, Set, NotSet, Order};
use sea_orm::prelude::*;
use std::sync::Arc;
use serde::Serialize;
use chrono::Utc;

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
    #[allow(unused_variables)]
    pub async fn list_departments(
        &self,
        page: u64,
        page_size: u64,
        parent_id: Option<i32>,
        search: Option<String>,
    ) -> Result<(Vec<department::Model>, u64), sea_orm::DbErr> {
        let mut query = DepartmentEntity::find();
        
        // 应用过滤条件
        if let Some(pid) = parent_id {
            query = query.filter(department::Column::ParentId.eq(pid));
        }
        
        if let Some(keyword) = search {
            query = query.filter(
                department::Column::Name.like(format!("%{}%", keyword))
                    .or(department::Column::Description.like(format!("%{}%", keyword)))
            );
        }
        
        // 获取总数
        let total = query.clone().count(&*self.db).await?;
        
        // 应用分页和排序
        let departments = query
            .order_by(department::Column::Name, Order::Asc)
            .into_model::<department::Model>()
            .all(&*self.db)
            .await?;
        
        Ok((departments, total))
    }
    
    /// 获取部门详情
    pub async fn get_department(&self, id: i32) -> Result<department::Model, sea_orm::DbErr> {
        DepartmentEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("部门 ID {} 不存在", id)))
    }
    
    /// 创建部门
    pub async fn create_department(
        &self,
        name: String,
        description: Option<String>,
        parent_id: Option<i32>,
    ) -> Result<department::Model, sea_orm::DbErr> {
        // 检查父部门是否存在（如果提供了 parent_id）
        if let Some(pid) = parent_id {
            let _ = DepartmentEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("父部门 ID {} 不存在", pid)))?;
        }
        
        let active_model = department::ActiveModel {
            id: NotSet,
            code: Set(name.chars().take(10).collect()),
            name: Set(name),
            parent_id: Set(parent_id),
            manager_id: Set(None),
            description: Set(description),
            sort_order: Set(0),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };
        
        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }
    
    /// 更新部门
    pub async fn update_department(
        &self,
        id: i32,
        name: Option<String>,
        description: Option<String>,
        parent_id: Option<i32>,
    ) -> Result<department::Model, sea_orm::DbErr> {
        let mut dept: department::ActiveModel = DepartmentEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("部门 ID {} 不存在", id)))?
            .into();
        
        if let Some(n) = name {
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
        
        if let Some(d) = description {
            dept.description = Set(Some(d));
        }
        
        if let Some(pid) = parent_id {
            // 检查父部门是否存在
            let _ = DepartmentEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("父部门 ID {} 不存在", pid)))?;
            dept.parent_id = Set(Some(pid));
        }
        
        dept.updated_at = Set(Utc::now());
        
        let result = dept.update(&*self.db).await?;
        Ok(result)
    }
    
    /// 删除部门
    pub async fn delete_department(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        // 检查是否有子部门
        let children_count = DepartmentEntity::find()
            .filter(department::Column::ParentId.eq(id))
            .count(&*self.db)
            .await?;
        
        if children_count > 0 {
            return Err(sea_orm::DbErr::Custom("该部门存在子部门，无法删除".to_string()));
        }
        
        let result = DepartmentEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(sea_orm::DbErr::RecordNotFound(format!("部门 ID {} 不存在", id)));
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
        let mut dept_map: std::collections::HashMap<i32, DepartmentTreeNode> = std::collections::HashMap::new();
        
        // 先创建所有节点
        for dept in all_departments {
            dept_map.insert(dept.id, DepartmentTreeNode {
                id: dept.id,
                name: dept.name,
                description: dept.description,
                parent_id: dept.parent_id,
                children: Vec::new(),
            });
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
    #[allow(dead_code)]
    pub async fn find_by_name(&self, name: &str) -> Result<department::Model, sea_orm::DbErr> {
        DepartmentEntity::find()
            .filter(department::Column::Name.eq(name))
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("部门名称 {} 不存在", name)))
    }
}

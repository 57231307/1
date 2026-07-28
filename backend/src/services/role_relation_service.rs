//! 角色关系服务（V15 P1 12.2）
//!
//! 实现角色继承与互斥校验：
//! - 角色继承：sales_manager 继承 sales 的所有权限
//! - 权限互斥：finance 与 sales 不能同时拥有（职责分离）
//! - 系统校验：用户分配角色时检查互斥规则
//!
//! 与 role_conflicts 表（SoD 互斥）互补：
//! - role_conflicts 仅用于财务三权分立等 SoD 互斥
//! - role_relations 用于更通用的继承 + 互斥（覆盖业务角色）

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::models::role_relation::{self, relation_type, Entity as RoleRelationEntity};
use crate::utils::error::AppError;

/// 角色关系服务
pub struct RoleRelationService {
    db: Arc<DatabaseConnection>,
}

/// 创建角色关系请求
#[derive(Debug, Deserialize)]
pub struct CreateRoleRelationRequest {
    pub parent_role_code: String,
    pub child_role_code: String,
    pub relation_type: String,
    pub description: Option<String>,
}

impl RoleRelationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// V15 P1 12.2：检查用户角色互斥
    ///
    /// 在用户分配角色时调用，检查新角色是否与用户已有角色互斥
    ///
    /// # 参数
    /// - `existing_role_codes`：用户当前已持有的角色编码列表
    /// - `new_role_code`：待分配的新角色编码
    ///
    /// # 返回
    /// - `Ok(())`：无互斥冲突，可分配
    /// - `Err(AppError)`：存在互斥角色，拒绝分配
    pub async fn check_mutual_exclusive(
        &self,
        existing_role_codes: &[String],
        new_role_code: &str,
    ) -> Result<(), AppError> {
        if existing_role_codes.is_empty() {
            return Ok(());
        }

        // 查询所有互斥关系
        let exclusive_relations = RoleRelationEntity::find()
            .filter(role_relation::Column::RelationType.eq(relation_type::MUTUAL_EXCLUSIVE))
            .all(&*self.db)
            .await?;

        // 检查新角色是否与已有角色互斥
        for existing_code in existing_role_codes {
            for rel in &exclusive_relations {
                let is_conflict = (rel.parent_role_code == *existing_code
                    && rel.child_role_code == new_role_code)
                    || (rel.child_role_code == *existing_code
                        && rel.parent_role_code == new_role_code);

                if is_conflict {
                    return Err(AppError::business(format!(
                        "角色互斥冲突：{} 与 {} 不可同时持有（职责分离原则）",
                        existing_code, new_role_code
                    )));
                }
            }
        }

        Ok(())
    }

    /// V15 P1 12.2：获取角色继承的所有子角色编码
    ///
    /// 递归查询角色继承链，返回该角色继承的所有子角色编码
    /// 例如：sales_manager → [sales_rep]（sales_manager 继承 sales_rep 的权限）
    ///
    /// # 参数
    /// - `role_code`：父角色编码
    ///
    /// # 返回
    /// 返回该角色直接和间接继承的所有子角色编码列表（不含自身）
    pub async fn get_inherited_role_codes(&self, role_code: &str) -> Result<Vec<String>, AppError> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(role_code.to_string());

        // BFS 遍历继承链
        let mut queue = vec![role_code.to_string()];
        while let Some(current) = queue.pop() {
            let children = RoleRelationEntity::find()
                .filter(role_relation::Column::ParentRoleCode.eq(&current))
                .filter(role_relation::Column::RelationType.eq(relation_type::INHERIT))
                .all(&*self.db)
                .await?;

            for child in children {
                if visited.insert(child.child_role_code.clone()) {
                    result.push(child.child_role_code.clone());
                    queue.push(child.child_role_code);
                }
            }
        }

        Ok(result)
    }

    /// V15 P1 12.2：创建角色关系
    pub async fn create_relation(
        &self,
        request: CreateRoleRelationRequest,
    ) -> Result<role_relation::Model, AppError> {
        // 校验关系类型
        if request.relation_type != relation_type::INHERIT
            && request.relation_type != relation_type::MUTUAL_EXCLUSIVE
        {
            return Err(AppError::business(format!(
                "无效的关系类型：{}（仅支持 inherit / mutual_exclusive）",
                request.relation_type
            )));
        }

        // 校验不可自引用
        if request.parent_role_code == request.child_role_code {
            return Err(AppError::business(
                "角色关系不可自引用（parent 和 child 不能相同）".to_string(),
            ));
        }

        let active_model = role_relation::ActiveModel {
            id: Default::default(),
            parent_role_code: Set(request.parent_role_code),
            child_role_code: Set(request.child_role_code),
            relation_type: Set(request.relation_type),
            description: Set(request.description),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let model = active_model.insert(&*self.db).await?;
        Ok(model)
    }

    /// V15 P1 12.2：删除角色关系
    pub async fn delete_relation(&self, relation_id: i64) -> Result<(), AppError> {
        RoleRelationEntity::delete_by_id(relation_id)
            .exec(&*self.db)
            .await?;
        Ok(())
    }

    /// V15 P1 12.2：查询所有角色关系
    pub async fn list_relations(
        &self,
        relation_type_filter: Option<&str>,
    ) -> Result<Vec<role_relation::Model>, AppError> {
        let query = RoleRelationEntity::find();
        let relations = match relation_type_filter {
            Some(rt) => {
                query
                    .filter(role_relation::Column::RelationType.eq(rt))
                    .all(&*self.db)
                    .await?
            }
            None => query.all(&*self.db).await?,
        };
        Ok(relations)
    }

    /// V15 P1 12.2：查询两个角色之间的关系
    ///
    /// 返回 (role_a, role_b) 之间的所有关系（含双向）
    pub async fn get_relation_between(
        &self,
        role_a_code: &str,
        role_b_code: &str,
    ) -> Result<Vec<role_relation::Model>, AppError> {
        use sea_orm::Condition;
        let relations = RoleRelationEntity::find()
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(role_relation::Column::ParentRoleCode.eq(role_a_code))
                            .add(role_relation::Column::ChildRoleCode.eq(role_b_code)),
                    )
                    .add(
                        Condition::all()
                            .add(role_relation::Column::ParentRoleCode.eq(role_b_code))
                            .add(role_relation::Column::ChildRoleCode.eq(role_a_code)),
                    ),
            )
            .all(&*self.db)
            .await?;
        Ok(relations)
    }
}

//! 数据权限服务
//!
//! 提供数据范围控制和字段级权限管理功能
use crate::models::data_permission::{self, Entity as DataPermissionEntity};
use crate::utils::admin_checker;
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use serde_json::Value;
use std::sync::Arc;

/// 数据范围类型常量
/// V15 P2 B12-P2-7：扩展为完整 4 档分级常量（ALL/DEPT/SELF/CUSTOM），
/// 与 data_permission 表 scope_type 字段对齐，供服务层显式引用而非硬编码字符串
pub mod data_scope {
    /// 全部数据（管理员）
    pub const ALL: &str = "ALL";
    /// 本部门数据
    pub const DEPT: &str = "DEPT";
    /// 仅本人数据
    pub const SELF: &str = "SELF";
    /// 自定义数据范围
    pub const CUSTOM: &str = "CUSTOM";
}

/// 数据权限查询结果
#[derive(Debug, Clone)]
pub struct DataPermissionResult {
    /// 数据范围类型
    pub scope_type: String,
    /// 自定义条件
    pub custom_condition: Option<Value>,
    /// 允许的字段
    pub allowed_fields: Option<Vec<String>>,
    /// 隐藏的字段
    pub hidden_fields: Option<Vec<String>>,
}

/// 数据权限服务
pub struct DataPermissionService {
    db: Arc<DatabaseConnection>,
}

impl DataPermissionService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取角色的数据权限
    pub async fn get_role_data_permission(
        &self,
        role_id: i32,
        resource_type: &str,
    ) -> Result<Option<DataPermissionResult>, AppError> {
        // Admin 角色拥有全部权限（从数据库查询角色编码）
        if self.is_admin_role(role_id).await? {
            return Ok(Some(DataPermissionResult {
                scope_type: data_scope::ALL.to_string(),
                custom_condition: None,
                allowed_fields: None,
                hidden_fields: None,
            }));
        }

        let permission = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(resource_type))
            .filter(data_permission::Column::IsEnabled.eq(true))
            .one(&*self.db)
            .await?;

        Ok(permission.map(|p| DataPermissionResult {
            scope_type: p.scope_type,
            custom_condition: p.custom_condition,
            allowed_fields: p.allowed_fields.and_then(|f| {
                f.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }),
            hidden_fields: p.hidden_fields.and_then(|f| {
                f.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }),
        }))
    }

    /// 检查角色是否为管理员角色（带缓存）
    async fn is_admin_role(&self, role_id: i32) -> Result<bool, AppError> {
        Ok(admin_checker::is_admin_role(&self.db, role_id).await)
    }

    /// 设置数据权限
    /// 批次 85 v2 复审 P1-8 修复：find + update/insert 移入单一事务 + lock_exclusive 串行化；原实现 find + update/insert 在 self.db 上分别执行，无 txn 无 lock，并发设置相同权限会基于过期状态 upsert
    pub async fn set_data_permission(
        &self,
        role_id: i32,
        resource_type: String,
        scope_type: String,
        custom_condition: Option<Value>,
        allowed_fields: Option<Value>,
        hidden_fields: Option<Value>,
    ) -> Result<data_permission::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发 upsert
        let existing = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(&resource_type))
            .lock_exclusive()
            .one(&txn)
            .await?;

        let permission = if let Some(existing) = existing {
            let mut active_model: data_permission::ActiveModel = existing.into();
            active_model.scope_type = Set(scope_type);
            active_model.custom_condition = Set(custom_condition);
            active_model.allowed_fields = Set(allowed_fields);
            active_model.hidden_fields = Set(hidden_fields);
            active_model.is_enabled = Set(true);
            active_model.updated_at = Set(Utc::now());
            active_model.update(&txn).await?
        } else {
            let active_model = data_permission::ActiveModel {
                id: Default::default(),
                role_id: Set(role_id),
                resource_type: Set(resource_type),
                scope_type: Set(scope_type),
                custom_condition: Set(custom_condition),
                allowed_fields: Set(allowed_fields),
                hidden_fields: Set(hidden_fields),
                is_enabled: Set(true),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            active_model.insert(&txn).await?
        };

        txn.commit().await?;
        Ok(permission)
    }

    /// 删除数据权限
    pub async fn delete_data_permission(
        &self,
        role_id: i32,
        resource_type: &str,
    ) -> Result<(), AppError> {
        let existing = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(resource_type))
            .one(&*self.db)
            .await?;

        if let Some(existing) = existing {
            let mut active_model: data_permission::ActiveModel = existing.into();
            active_model.is_enabled = Set(false);
            active_model.updated_at = Set(Utc::now());
            active_model.update(&*self.db).await?;
        }

        Ok(())
    }

    /// 获取角色的所有数据权限
    pub async fn list_role_data_permissions(
        &self,
        role_id: i32,
    ) -> Result<Vec<data_permission::Model>, AppError> {
        let permissions = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::IsEnabled.eq(true))
            .all(&*self.db)
            .await?;

        Ok(permissions)
    }

    /// 获取所有数据权限列表
    pub async fn list_all_data_permissions(&self) -> Result<Vec<data_permission::Model>, AppError> {
        let permissions = DataPermissionEntity::find()
            .filter(data_permission::Column::IsEnabled.eq(true))
            .all(&*self.db)
            .await?;

        Ok(permissions)
    }

    /// 过滤字段（根据字段权限）
    pub fn filter_fields(
        &self,
        data: &mut serde_json::Value,
        allowed_fields: &Option<Vec<String>>,
        hidden_fields: &Option<Vec<String>>,
    ) {
        if let Some(obj) = data.as_object_mut() {
            // 如果有允许的字段列表，只保留允许的字段
            if let Some(allowed) = allowed_fields {
                let allowed_set: std::collections::HashSet<_> = allowed.iter().cloned().collect();
                obj.retain(|key, _| allowed_set.contains(key));
            }

            // 移除隐藏的字段
            if let Some(hidden) = hidden_fields {
                for field in hidden {
                    obj.remove(field);
                }
            }
        }
    }

    /// 批量过滤字段
    pub fn filter_fields_batch(
        &self,
        data_list: &mut [serde_json::Value],
        allowed_fields: &Option<Vec<String>>,
        hidden_fields: &Option<Vec<String>>,
    ) {
        for data in data_list {
            self.filter_fields(data, allowed_fields, hidden_fields);
        }
    }

    /// V15 P1 10.4-2：应用数据范围（销售/客户/成本数据隔离）
    /// 根据用户角色返回数据范围过滤条件，业务查询层据此追加 WHERE 子句：admin 角色：返回 ALL（不加过滤）；sales/sales_manager 角色：仅可查询自己负责的客户发放记录；（customer.owner_id = user_id 的客户集合）；customer 角色：仅可查询自己的发放记录（customer_id = user.customer_id）；其他角色：仅可查询自己发放的记录（issued_by = user_id）；# 参数；`user_id`：当前登录用户 ID；`role_id`：当前登录用户的角色 ID；`role_code`：角色编码（admin/sales/sales_manager/customer 等）；# 返回；返回 `DataScopeFilter`，业务层据此构造查询条件
    pub async fn apply_data_scope(
        &self,
        user_id: i32,
        role_id: Option<i32>,
        role_code: &str,
    ) -> Result<DataScopeFilter, AppError> {
        // admin 角色拥有全部数据权限
        if let Some(rid) = role_id {
            if self.is_admin_role(rid).await? {
                return Ok(DataScopeFilter::all());
            }
        }

        // 按角色编码分发数据范围
        match role_code {
            // 销售角色：仅可查询自己负责的客户发放记录
            "sales" | "sales_manager" => {
                let customer_ids = self.get_sales_customer_ids(user_id).await?;
                Ok(DataScopeFilter::customer_scope(customer_ids))
            }
            // 客户门户角色：仅可查询自己的发放记录
            // 注意：customer_id 字段在 user 表中暂无，这里返回空列表表示无数据权限
            // 业务层应通过 user → customer 映射获取 customer_id
            "customer" => {
                let customer_id = self.get_customer_id_by_user(user_id).await?;
                match customer_id {
                    Some(cid) => Ok(DataScopeFilter::single_customer(cid)),
                    None => Ok(DataScopeFilter::none()),
                }
            }
            // 其他角色：仅可查询自己发放的记录
            _ => Ok(DataScopeFilter::self_issued(user_id)),
        }
    }

    /// V15 P1 10.4-2：查询销售负责的客户 ID 列表（通过 customers.owner_id = user_id 关联查询（客户主数据的业务负责人））
    async fn get_sales_customer_ids(&self, user_id: i32) -> Result<Vec<i64>, AppError> {
        use crate::models::customer::{self, Entity as CustomerEntity};
        let customers = CustomerEntity::find()
            .filter(customer::Column::OwnerId.eq(user_id))
            .filter(customer::Column::Status.ne("blacklist"))
            .all(&*self.db)
            .await?;
        Ok(customers.into_iter().map(|c| c.id as i64).collect())
    }

    /// V15 P1 10.4-2：根据用户 ID 查询关联的客户 ID
    /// 客户门户场景：当前 user 表无 customer_id 字段，暂返回 None。；后续如需支持客户门户角色，应在 user 表新增 customer_id 字段或；建立 user_customer 映射表，届时在此方法补充查询逻辑。
    async fn get_customer_id_by_user(&self, _user_id: i32) -> Result<Option<i64>, AppError> {
        // 当前 user 表无 customer_id 字段，客户门户角色暂无数据访问权限
        // TODO: 后续 user 表新增 customer_id 字段后补充查询逻辑
        Ok(None)
    }

    /// V15 P1 10.4-2：检查用户是否可查看成本数据
    pub async fn can_view_cost_data(
        &self,
        role_id: Option<i32>,
        role_code: &str,
    ) -> Result<bool, AppError> {
        if let Some(rid) = role_id {
            if self.is_admin_role(rid).await? {
                return Ok(true);
            }
        }
        Ok(matches!(role_code, "finance" | "warehouse_manager"))
    }

    /// V15 P1 batch-19 缺陷 23.1.1：获取用户部门树数据范围（含兼职部门及子部门）
    pub async fn get_user_dept_scope_ids(&self, user_id: i32) -> Result<Vec<i32>, AppError> {
        use crate::models::user_department::{self, Entity as UserDeptEntity};
        let user_depts = UserDeptEntity::find()
            .filter(user_department::Column::UserId.eq(user_id))
            .all(&*self.db)
            .await?;
        let mut dept_ids = Vec::new();
        for ud in user_depts {
            let subtree = self.collect_dept_subtree(ud.department_id).await?;
            dept_ids.extend(subtree);
        }
        Ok(dept_ids)
    }

    /// 收集部门子树 ID（含自身，迭代实现避免 async 递归 boxing）
    async fn collect_dept_subtree(&self, dept_id: i32) -> Result<Vec<i32>, AppError> {
        use crate::models::department::{self, Entity as DeptEntity};
        let mut result = Vec::new();
        let mut stack = vec![dept_id];
        while let Some(id) = stack.pop() {
            result.push(id);
            let children = DeptEntity::find()
                .filter(department::Column::ParentId.eq(id))
                .all(&*self.db)
                .await?;
            for child in children {
                stack.push(child.id);
            }
        }
        Ok(result)
    }
}

/// V15 P1 10.4-2：数据范围过滤条件（业务查询层追加 WHERE 子句实现行级数据隔离）
#[derive(Debug, Clone)]
pub struct DataScopeFilter {
    /// 数据范围类型
    pub scope: DataScopeType,
    /// 允访问的客户 ID 列表（scope=CustomerScope 时有效）
    pub customer_ids: Vec<i64>,
    /// 单个客户 ID（scope=SingleCustomer 时有效）
    pub customer_id: Option<i64>,
    /// 发放人 ID（scope=SelfIssued 时有效）
    pub issued_by: Option<i32>,
}

/// 数据范围类型
#[derive(Debug, Clone, PartialEq)]
pub enum DataScopeType {
    /// 全部数据（admin 角色）
    All,
    /// 按客户 ID 列表过滤（销售角色）
    CustomerScope,
    /// 单个客户（客户门户角色）
    SingleCustomer,
    /// 仅本人发放的记录（其他角色）
    SelfIssued,
    /// 无数据权限（兜底）
    None,
}

impl DataScopeFilter {
    /// 全部数据（admin）
    pub fn all() -> Self {
        Self {
            scope: DataScopeType::All,
            customer_ids: Vec::new(),
            customer_id: None,
            issued_by: None,
        }
    }

    /// 按客户 ID 列表过滤（销售角色）
    pub fn customer_scope(customer_ids: Vec<i64>) -> Self {
        Self {
            scope: DataScopeType::CustomerScope,
            customer_ids,
            customer_id: None,
            issued_by: None,
        }
    }

    /// 单个客户（客户门户角色）
    pub fn single_customer(customer_id: i64) -> Self {
        Self {
            scope: DataScopeType::SingleCustomer,
            customer_ids: Vec::new(),
            customer_id: Some(customer_id),
            issued_by: None,
        }
    }

    /// 仅本人发放的记录（其他角色）
    pub fn self_issued(user_id: i32) -> Self {
        Self {
            scope: DataScopeType::SelfIssued,
            customer_ids: Vec::new(),
            customer_id: None,
            issued_by: Some(user_id),
        }
    }

    /// 无数据权限（兜底）
    pub fn none() -> Self {
        Self {
            scope: DataScopeType::None,
            customer_ids: Vec::new(),
            customer_id: None,
            issued_by: None,
        }
    }

    /// 是否为全部数据（无需过滤）
    pub fn is_all(&self) -> bool {
        matches!(self.scope, DataScopeType::All)
    }

    /// 是否无数据权限
    pub fn is_none(&self) -> bool {
        matches!(self.scope, DataScopeType::None)
    }
}

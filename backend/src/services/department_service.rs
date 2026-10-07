use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, NotSet, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Serialize;

use crate::models::department::{self, Entity as DepartmentEntity};
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;

/// 部门树节点（用于返回树形结构）
#[derive(Debug, Serialize, Clone)]
pub struct DepartmentTreeNode {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    /// 负责人姓名（契约对齐：前端「负责人」列，查 users 表填充）
    pub manager_name: Option<String>,
    pub children: Vec<DepartmentTreeNode>,
}

crate::define_service!(DepartmentService);

impl DepartmentService {
    /// 批量填充部门负责人姓名（manager_id → users.username）
    ///
    /// 填充失败仅记录 warn，不阻塞列表/详情返回（manager_name 为展示字段）。
    async fn fill_manager_names(
        db: &sea_orm::DatabaseConnection,
        departments: &mut [department::Model],
    ) {
        let manager_ids: std::collections::HashSet<i32> =
            departments.iter().filter_map(|d| d.manager_id).collect();
        if manager_ids.is_empty() {
            return;
        }
        let users = crate::models::user::Entity::find()
            .filter(
                crate::models::user::Column::Id.is_in(manager_ids.into_iter().collect::<Vec<_>>()),
            )
            .all(db)
            .await;
        match users {
            Ok(users) => {
                let name_map: std::collections::HashMap<i32, String> =
                    users.into_iter().map(|u| (u.id, u.username)).collect();
                for d in departments.iter_mut() {
                    if let Some(mid) = d.manager_id {
                        d.manager_name = name_map.get(&mid).cloned();
                    }
                }
            }
            Err(e) => {
                tracing::warn!("填充部门负责人姓名失败（忽略，manager_name 置空）: {}", e);
            }
        }
    }

    /// 校验负责人用户存在（P0 契约修复：前端负责人下拉提交 manager_id，
    /// 后端真实列为 departments.manager_id；此前无任何存在性校验，悬挂 ID 会直接落库/触发 FK 500）。
    /// manager_id 为 Option：None 表示未指派/清除负责人，不校验。
    async fn validate_manager_exists(
        db: &sea_orm::DatabaseConnection,
        manager_id: Option<i32>,
    ) -> Result<(), AppError> {
        if let Some(mid) = manager_id {
            let exists = crate::models::user::Entity::find_by_id(mid)
                .one(db)
                .await?
                .is_some();
            if !exists {
                return Err(AppError::business_displayable(format!(
                    "负责人用户不存在（manager_id={}）",
                    mid
                )));
            }
        }
        Ok(())
    }

    /// 获取部门列表（支持分页和过滤）
    pub async fn list(
        &self,
        query: crate::handlers::department_handler::DepartmentListQuery,
    ) -> Result<crate::utils::response::PaginatedResponse<department::Model>, AppError> {
        let mut q = DepartmentEntity::find();

        // 应用过滤条件
        if let Some(pid) = query.parent_id {
            q = q.filter(department::Column::ParentId.eq(pid));
        }

        if let Some(keyword) = query.search {
            let pattern = safe_like_pattern(&keyword);
            q = q.filter(
                department::Column::Name
                    .like(&pattern)
                    .or(department::Column::Description.like(&pattern)),
            );
        }

        // 获取总数
        let total = q.clone().count(&*self.db).await?;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        // 应用分页和排序
        let mut departments = q
            .order_by(department::Column::Name, Order::Asc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .into_model::<department::Model>()
            .all(&*self.db)
            .await?;

        Self::fill_manager_names(&self.db, &mut departments).await;

        Ok(crate::utils::response::PaginatedResponse::new(
            departments,
            total,
            page,
            page_size,
        ))
    }

    /// 获取部门详情
    pub async fn get(&self, id: i32) -> Result<department::Model, AppError> {
        let mut dept = DepartmentEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("部门 ID {} 不存在", id)))?;

        Self::fill_manager_names(&self.db, std::slice::from_mut(&mut dept)).await;

        Ok(dept)
    }

    /// 创建部门
    pub async fn create(
        &self,
        req: crate::handlers::department_handler::CreateDepartmentRequest,
        user_id: i32,
    ) -> Result<department::Model, AppError> {
        // 检查部门名称是否已存在
        let existing = DepartmentEntity::find()
            .filter(department::Column::Name.eq(&req.name))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!(
                "部门名称 '{}' 已存在",
                req.name
            )));
        }

        // 检查父部门是否存在（如果提供了 parent_id）
        // 批次 98 P2-C 修复（v5 复审）：去掉冗余 let _ = ，父级校验通过 ? 传播
        if let Some(pid) = req.parent_id {
            DepartmentEntity::find_by_id(pid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("父部门 ID {} 不存在", pid)))?;
        }

        // P0 契约修复：负责人按 manager_id 校验用户存在（前端负责人下拉提交真实键 manager_id）
        Self::validate_manager_exists(&self.db, req.manager_id).await?;

        let active_model = department::ActiveModel {
            id: NotSet,
            // 契约对齐：前端 DepartmentCreateRequest.code 优先；不传/为空时自动生成
            code: Set(req
                .code
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| format!("DEPT_{}", Utc::now().timestamp_millis()))),
            name: Set(req.name),
            parent_id: Set(req.parent_id),
            manager_id: Set(req.manager_id),
            description: Set(req.description),
            // 契约对齐：前端 DepartmentCreateRequest.sort_order 优先；不传时默认 0
            sort_order: Set(req.sort_order.unwrap_or(0)),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let mut result = active_model.insert(&*self.db).await?;

        Self::fill_manager_names(&self.db, std::slice::from_mut(&mut result)).await;

        // 审计日志：记录部门创建操作
        let after_snapshot = serde_json::to_value(&result).ok();
        let audit_svc = std::sync::Arc::new(
            crate::services::audit_log_service::AuditLogService::new(self.db.clone()),
        );
        let event = crate::services::audit_log_service::AuditEvent {
            user_id: Some(user_id),
            username: None,
            operation_type: crate::models::audit_log::OperationType::Create,
            severity: crate::models::audit_log::Severity::Info,
            resource_type: Some("department".to_string()),
            resource_id: Some(result.id.to_string()),
            resource_name: Some(result.name.clone()),
            description: Some(format!("创建部门: {}", result.name)),
            request_method: Some("POST".to_string()),
            request_path: None,
            before_snapshot: None,
            after_snapshot,
        };
        audit_svc.record_async(event, None);

        Ok(result)
    }

    /// 更新部门（批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，；保证审计日志能追溯实际更新人。）
    ///
    /// 字段三态语义（对齐 RFC 7386 JSON Merge Patch）：
    /// None（键缺席）=保持原值、Some(None)（显式 null）=置 NULL、Some(Some(v))=覆盖。
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        req: crate::handlers::department_handler::UpdateDepartmentRequest,
    ) -> Result<department::Model, AppError> {
        // NOT NULL 列门控（departments.name/code NOT NULL，m0001 DDL；sort_order/is_active
        // 实体 Model 列为非 Option i32/bool，置 NULL 后该行按模型不可读）：
        // 显式 null 是调用方错误，不是"保持原值"；在任何 DB 访问之前拒绝，错误外显不脱敏。
        if matches!(req.name, Some(None)) {
            return Err(AppError::business_displayable(
                "部门名称不能清空：该字段为必填项",
            ));
        }
        if matches!(req.code, Some(None)) {
            return Err(AppError::business_displayable(
                "部门编码不能清空：该字段为必填项",
            ));
        }
        if matches!(req.sort_order, Some(None)) {
            return Err(AppError::business_displayable(
                "排序号不能清空：该字段为必填项",
            ));
        }
        if matches!(req.is_active, Some(None)) {
            return Err(AppError::business_displayable(
                "启用状态不能清空：该字段为必填项",
            ));
        }

        let mut dept: department::ActiveModel = DepartmentEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("部门 ID {} 不存在", id)))?
            .into();

        // 三态写入规则：None=不 Set（列保持 Unset，UPDATE 不含该列，原值不动）；
        // Some(None)=Set(None) 置 NULL；Some(Some(v))=Set(v) 覆盖。
        // 不得塌成单层 Option<T>：塌层后"清空"与"保持"共用同一表示，
        // description/parent_id/manager_id 将无法置 NULL（清空请求会被静默丢弃）。
        // name/code 为 NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(n) = req.name.flatten() {
            // 检查部门名称是否已存在
            let existing = DepartmentEntity::find()
                .filter(department::Column::Name.eq(&n))
                .filter(department::Column::Id.ne(id))
                .one(&*self.db)
                .await?;

            if existing.is_some() {
                return Err(AppError::business_displayable(format!(
                    "部门名称 '{}' 已存在",
                    n
                )));
            }
            dept.name = Set(n);
        }

        // P0 契约修复：code 原不在 UpdateDepartmentRequest 内，前端编辑编码被静默丢弃
        // （真实列 departments.code NOT NULL UNIQUE，非空变更时查重排除自身）
        if let Some(c) = req.code.flatten() {
            let c = c.trim().to_string();
            if !c.is_empty() {
                let existing = DepartmentEntity::find()
                    .filter(department::Column::Code.eq(&c))
                    .filter(department::Column::Id.ne(id))
                    .one(&*self.db)
                    .await?;
                if existing.is_some() {
                    return Err(AppError::business_displayable(format!(
                        "部门编码 '{}' 已存在",
                        c
                    )));
                }
                dept.code = Set(c);
            }
        }

        // description：DB 可空列（TEXT，m0001 DDL 核实）——显式 null 清空
        if let Some(d) = req.description {
            dept.description = Set(d);
        }

        // parent_id：DB 可空列（自引用外键）——Some(None)=脱离父级成为顶级部门
        if let Some(pid) = req.parent_id {
            match pid {
                Some(pid) => {
                    // 检查父部门存在（P2-C 修复 v5 复审：去掉冗余 let _ =）
                    DepartmentEntity::find_by_id(pid)
                        .one(&*self.db)
                        .await?
                        .ok_or_else(|| AppError::not_found(format!("父部门 ID {} 不存在", pid)))?;
                    dept.parent_id = Set(Some(pid));
                }
                None => {
                    dept.parent_id = Set(None);
                }
            }
        }

        // manager_id：DB 可空列——Some(None)=清除负责人；Some(Some(mid)) 先校验用户存在再落库
        if let Some(mid) = req.manager_id {
            if mid.is_some() {
                Self::validate_manager_exists(&self.db, mid).await?;
            }
            dept.manager_id = Set(mid);
        }

        // sort_order/is_active 对应实体 Model 非 Option 列（NOT NULL 语义）：仅覆盖/保持
        if let Some(so) = req.sort_order.flatten() {
            dept.sort_order = Set(so);
        }

        if let Some(ia) = req.is_active.flatten() {
            dept.is_active = Set(ia);
        }

        dept.updated_at = Set(Utc::now());

        // 审计：update_with_audit 以更新后回读的 Model 生成 after_snapshot，
        // Set(None) 的列在 UPDATE 真实落 NULL 后进入快照——审计反映变更后真实值，不留假旧值。
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            self.db.as_ref(),
            "departments",
            dept,
            Some(user_id),
        )
        .await?;
        Ok(result)
    }

    /// 删除部门（批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，；保证审计日志能追溯实际删除人。）
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // 检查是否有子部门
        let children_count = DepartmentEntity::find()
            .filter(department::Column::ParentId.eq(id))
            .count(&*self.db)
            .await?;

        if children_count > 0 {
            return Err(AppError::business("该部门存在子部门，无法删除".to_string()));
        }

        // 检查是否有用户关联此部门
        let user_count = crate::models::user::Entity::find()
            .filter(crate::models::user::Column::DepartmentId.eq(id))
            .count(&*self.db)
            .await?;

        if user_count > 0 {
            return Err(AppError::business(format!(
                "该部门下有 {} 个用户，请先移除用户的部门关联后再删除",
                user_count
            )));
        }

        // P0 8-3 修复：delete 操作补审计日志
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            DepartmentEntity,
            _,
        >(&*self.db, "department", id, Some(user_id))
        .await
    }

    /// 获取部门树形结构
    pub async fn get_department_tree(&self) -> Result<Vec<DepartmentTreeNode>, AppError> {
        let mut all_departments = DepartmentEntity::find()
            .order_by(department::Column::Name, Order::Asc)
            .all(&*self.db)
            .await?;

        Self::fill_manager_names(&self.db, &mut all_departments).await;

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
                    manager_name: dept.manager_name,
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
    pub async fn find_by_name(&self, name: &str) -> Result<department::Model, AppError> {
        DepartmentEntity::find()
            .filter(department::Column::Name.eq(name))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("部门名称 {} 不存在", name)))
    }

    /// V15 P1 batch-19 缺陷 23.1.2：为用户分配部门（主部门 + 兼职）
    pub async fn assign_user_departments(
        &self,
        user_id: i32,
        primary_dept_id: i32,
        secondary_dept_ids: Vec<i32>,
    ) -> Result<Vec<crate::models::user_department::Model>, AppError> {
        use crate::models::user_department::{self, ActiveModel as UserDeptActive};
        use sea_orm::EntityTrait;

        let txn = self.db.begin().await?;
        let now = Utc::now();
        let mut results = Vec::new();

        // 先清除用户已有部门关联
        user_department::Entity::delete_many()
            .filter(user_department::Column::UserId.eq(user_id))
            .exec(&txn)
            .await?;

        // 插入主部门
        let primary = UserDeptActive {
            id: Default::default(),
            user_id: sea_orm::Set(user_id),
            department_id: sea_orm::Set(primary_dept_id),
            is_primary: sea_orm::Set(true),
            start_date: sea_orm::Set(None),
            end_date: sea_orm::Set(None),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        results.push(primary.insert(&txn).await?);

        // 插入兼职部门
        for dept_id in secondary_dept_ids {
            if dept_id == primary_dept_id {
                continue;
            }
            let sec = UserDeptActive {
                id: Default::default(),
                user_id: sea_orm::Set(user_id),
                department_id: sea_orm::Set(dept_id),
                is_primary: sea_orm::Set(false),
                start_date: sea_orm::Set(None),
                end_date: sea_orm::Set(None),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            results.push(sec.insert(&txn).await?);
        }

        txn.commit().await?;
        // m_rls_dept_domain：部门变更失效可见部门集合缓存
        crate::services::data_permission_service::DataPermissionService::invalidate_dept_scope_cache(
            user_id,
        );
        Ok(results)
    }

    /// V15 P1 batch-19 缺陷 23.1.2：查询用户所有部门（含兼职）
    pub async fn list_user_departments(
        &self,
        user_id: i32,
    ) -> Result<Vec<crate::models::user_department::Model>, AppError> {
        use crate::models::user_department::{self, Entity as UserDeptEntity};
        let depts = UserDeptEntity::find()
            .filter(user_department::Column::UserId.eq(user_id))
            .all(&*self.db)
            .await?;
        Ok(depts)
    }
}

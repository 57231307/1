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
///
/// 出参键集合 = 部门档案中树与列表两侧都要展示的那几列；serde 无 rename，
/// 键名按字段名原样输出且恒存在（无 skip_serializing_if），前端可直接按列绑定，
/// 不必再用分页列表拼全量。
#[derive(Debug, Serialize, Clone)]
pub struct DepartmentTreeNode {
    pub id: i32,
    pub name: String,
    /// 部门编码（departments.code 真实列，NOT NULL）
    pub code: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    /// 负责人姓名（契约对齐：前端「负责人」列，查 users 表填充）
    pub manager_name: Option<String>,
    /// 排序号（departments.sort_order 真实列，DDL 可空带 DEFAULT 0，实体声明为非 Option i32）
    pub sort_order: i32,
    /// 启用状态（departments.is_active 真实列，DDL 可空带 DEFAULT true，实体声明为非 Option bool）
    pub is_active: bool,
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

    /// 校验把部门的 parent_id 写成 new_parent_id 不会让层级成环（拒绝指向自身或自身的后代）
    ///
    /// 功能：部门更新的父级判环门。从待写入的上级沿祖先链上溯，链上出现被更新部门自身，
    /// 说明该上级是本部门的后代，写入后父级关系成环，部门可见域展开与层级组装都会失稳，
    /// 故在落库前拒绝。上溯带已访问集合，库内存量数据若已含成环行，判重使上溯在有限步数
    /// 内终止，不会死循环；该脏环不含本部门时本次写入不构成新环，放行。
    /// 调用方：DepartmentService::update 的 parent_id 写入分支（仅显式指定新上级的请求）。
    /// 入参：数据库连接、被更新部门的主键、待写入的上级部门主键（上级行存在性由调用方先行校验）。
    /// 传给谁：判定结果返回给 update；拒绝时携带可外显的校验错误经端点回给用户，
    /// 主键只进日志不进文案，通过与拒绝都有显式日志。
    /// 存什么：只读 departments 表全部行的主键 id 与 parent_id 两列，按 parent_id 逐级上溯。
    /// 存哪里：不写任何表；上溯在内存完成，结果只进日志与错误返回。
    async fn validate_parent_no_cycle(
        db: &sea_orm::DatabaseConnection,
        id: i32,
        new_parent_id: i32,
    ) -> Result<(), AppError> {
        // 一次读全表建父级映射后内存上溯，代价与层级深度成正比，部门为主数据量级有限
        let parent_of: std::collections::HashMap<i32, Option<i32>> = DepartmentEntity::find()
            .all(db)
            .await?
            .into_iter()
            .map(|d| (d.id, d.parent_id))
            .collect();

        let mut cursor = new_parent_id;
        let mut visited: std::collections::HashSet<i32> = std::collections::HashSet::new();
        loop {
            if cursor == id {
                tracing::warn!(
                    dept_id = id,
                    new_parent_id,
                    "部门层级判环拒绝：待写入上级沿祖先链可回到本部门自身"
                );
                return Err(AppError::validation_displayable(
                    "上级部门不能选择本部门或本部门的下级部门，请重新选择父部门",
                ));
            }
            if !visited.insert(cursor) {
                // 回到已访问节点：上溯进入存量脏环且环不含本部门，终止上溯并放行
                break;
            }
            match parent_of.get(&cursor).copied().flatten() {
                Some(parent) => cursor = parent,
                None => break, // 上溯到顶级，链路无环
            }
        }

        tracing::info!(
            dept_id = id,
            new_parent_id,
            "部门层级判环通过：待写入上级不在本部门的后代链上"
        );
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
        // 排序必须构成全序：name 在数据库层无唯一约束（并发创建可越过应用层重名检查写入同名行），
        // 只按 name 排时同名行的先后不保证，offset 翻页会在页边界重复取到同一行、同时漏掉另一行。
        // 补 id 作次级键后每页内容是确定的全序切片，total 在并发翻页之间漂移也不再造成重复与漏取。
        let mut departments = q
            .order_by(department::Column::Name, Order::Asc)
            .order_by(department::Column::Id, Order::Asc)
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
                    // 判环：新上级不得是本部门自身或本部门后代，防父级链成环
                    Self::validate_parent_no_cycle(&self.db, id, pid).await?;
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
            // 删除腿引用门：存在未清理的子部门即拒绝，归业务族；
            // 文案点名公开规则与出路，不含计数与标识，可外显
            return Err(AppError::business_displayable(
                "该部门存在子部门，无法删除，请先处理子部门".to_string(),
            ));
        }

        // 检查是否有用户关联此部门
        let user_count = crate::models::user::Entity::find()
            .filter(crate::models::user::Column::DepartmentId.eq(id))
            .count(&*self.db)
            .await?;

        if user_count > 0 {
            // 删除腿引用门：仍有用户挂靠即拒绝，归业务族；
            // 文案点名公开规则与出路，去掉查询所得用户计数（只进 tracing），可外显
            return Err(AppError::business_displayable(
                "该部门下仍有用户，请先移除用户的部门关联后再删除",
            ));
        }

        // P0 8-3 修复：delete 操作补审计日志
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            DepartmentEntity,
            _,
        >(&*self.db, "department", id, Some(user_id))
        .await
    }

    /// 获取部门树形结构
    ///
    /// 出参覆盖 departments 全表每一行：先从 parent_id 为空的根节点递归组装，再把因
    /// 父级缺席或层级成环而从任何根都走不到的节点按根节点形态补入结果（parent_id 保持
    /// 数据库原值，只表示"父级不可达"，不代表层级正确），并逐类记 WARN。
    pub async fn get_department_tree(&self) -> Result<Vec<DepartmentTreeNode>, AppError> {
        let mut all_departments = DepartmentEntity::find()
            .order_by(department::Column::Name, Order::Asc)
            .all(&*self.db)
            .await?;

        Self::fill_manager_names(&self.db, &mut all_departments).await;

        // 构建部门树
        let mut dept_map: std::collections::HashMap<i32, DepartmentTreeNode> =
            std::collections::HashMap::new();

        // 阶段 1：创建所有节点，children 初始为空
        // known_dept_ids 记录表里真实存在的部门主键，供回收阶段区分"父级行不存在"
        // 与"父级行存在但从根走不到"两类完全不同的根因
        let mut known_dept_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for dept in all_departments {
            known_dept_ids.insert(dept.id);
            dept_map.insert(
                dept.id,
                DepartmentTreeNode {
                    id: dept.id,
                    name: dept.name,
                    code: dept.code,
                    description: dept.description,
                    parent_id: dept.parent_id,
                    manager_name: dept.manager_name,
                    sort_order: dept.sort_order,
                    is_active: dept.is_active,
                    children: Vec::new(),
                },
            );
        }

        // 阶段 2：先定出一份全局顺序，根级与每一层 children 都按它排列。
        // dept_map 是 HashMap，迭代序不保证：直接 iter() 收集会让同一份数据两次请求给出
        // 不同的同级顺序，前端一次取树直渲时表现为部门先后跳变。排序口径 sort_order 升序、
        // id 升序，sort_order 并列时由 id 兜底成全序。
        let mut order_keys: Vec<(i32, i32)> = dept_map
            .iter()
            .map(|(id, node)| (*id, node.sort_order))
            .collect();
        order_keys.sort_unstable_by_key(|&(id, sort_order)| (sort_order, id));
        let ordered_ids: Vec<i32> = order_keys.into_iter().map(|(id, _)| id).collect();

        let roots: Vec<i32> = ordered_ids
            .iter()
            .copied()
            .filter(|id| dept_map[id].parent_id.is_none())
            .collect();

        // parent_to_children 记录每个父节点的直接子节点 id 列表（按上面的全局顺序入表，
        // 因此 assemble 依序 push 出来的 children 已经是 sort_order,id 序）
        let mut parent_to_children: std::collections::HashMap<i32, Vec<i32>> =
            std::collections::HashMap::new();
        for id in &ordered_ids {
            if let Some(pid) = dept_map[id].parent_id {
                parent_to_children.entry(pid).or_default().push(*id);
            }
        }

        // 阶段 3：从根开始递归组装，将子节点从 dept_map 取出并挂入父 children
        fn assemble(
            id: i32,
            dept_map: &mut std::collections::HashMap<i32, DepartmentTreeNode>,
            parent_to_children: &std::collections::HashMap<i32, Vec<i32>>,
        ) -> Option<DepartmentTreeNode> {
            let mut node = dept_map.remove(&id)?;
            if let Some(child_ids) = parent_to_children.get(&id) {
                for &cid in child_ids {
                    if let Some(child) = assemble(cid, dept_map, parent_to_children) {
                        node.children.push(child);
                    }
                }
            }
            Some(node)
        }

        let mut tree: Vec<DepartmentTreeNode> = roots
            .into_iter()
            .filter_map(|rid| assemble(rid, &mut dept_map, &parent_to_children))
            .collect();

        // 阶段 4：回收从任何根都走不到的节点。
        // 可达的节点都已被 assemble 从 dept_map 取走，因此滞留在 dept_map 里的只有四类：
        // 父级部门行不存在、父级指向自身、父级落在环内、父级本身也滞留（环下或悬挂下的子树）。
        // 这些行在列表端点可见，在树端点同样必须可见；直接丢掉等于静默丢数据，
        // 故一律按根节点补入结果并逐类记 WARN。
        if !dept_map.is_empty() {
            // 分类要在再次组装之前完成（assemble 会把节点从 dept_map 移走）。
            // 补入结果沿用阶段 2 的全局顺序（sort_order 升序、id 升序），与正常根节点同一口径，
            // 不随 HashMap 迭代顺序漂移。
            let stuck_nodes: Vec<(i32, Option<i32>)> = ordered_ids
                .iter()
                .copied()
                .filter(|id| dept_map.contains_key(id))
                .map(|id| (id, dept_map[&id].parent_id))
                .collect();
            let parent_of: std::collections::HashMap<i32, Option<i32>> =
                stuck_nodes.iter().cloned().collect();
            let stuck_id_set: std::collections::HashSet<i32> =
                stuck_nodes.iter().map(|(id, _)| *id).collect();

            // 找出滞留集合里真正位于环上的节点：每个节点只有一条指向父级的边，沿父级链走，
            // 重新回到本次已走过的节点即成环，回到之处到链尾这一段就是环上节点。
            // 只有先分清"环上"与"环下"，WARN 才能如实描述父级指向的是环内部门还是
            // 一个同样不可达的父级，不把环下子树谎报成环节点。
            let mut on_cycle_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
            let mut judged_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
            for (start, _) in &stuck_nodes {
                if judged_ids.contains(start) {
                    continue;
                }
                let mut order: Vec<i32> = Vec::new();
                let mut seen_at: std::collections::HashMap<i32, usize> =
                    std::collections::HashMap::new();
                let mut cursor = *start;
                let mut cycle_from: Option<usize> = None;
                loop {
                    if let Some(idx) = seen_at.get(&cursor) {
                        cycle_from = Some(*idx);
                        break;
                    }
                    // 走进上一轮已走过的节点：那条链已经判定过成环或终止，本段前缀不在环上
                    if judged_ids.contains(&cursor) {
                        break;
                    }
                    seen_at.insert(cursor, order.len());
                    order.push(cursor);
                    match parent_of.get(&cursor).copied().flatten() {
                        Some(pid) if stuck_id_set.contains(&pid) => cursor = pid,
                        // 父级为空、父级行不存在或父级不在滞留集合：沿父级上溯即终止，不再有环
                        _ => break,
                    }
                }
                if let Some(idx) = cycle_from {
                    on_cycle_ids.extend(order[idx..].iter().copied());
                }
                judged_ids.extend(order.iter().copied());
            }

            let mut self_reference_ids: Vec<i32> = Vec::new();
            let mut cycle_ids: Vec<i32> = Vec::new();
            let mut unreachable_parent_ids: Vec<i32> = Vec::new();
            let mut dangling_parent_ids: Vec<(i32, i32)> = Vec::new();
            let mut mapping_anomalies: Vec<String> = Vec::new();
            for (id, parent) in &stuck_nodes {
                match parent {
                    // parent_id 为空的节点在阶段 2 已全部收进 roots，滞留到这里说明根收集失守
                    None => mapping_anomalies.push(format!("id={id} 的父级为空却未被收为根节点")),
                    Some(pid) if pid == id => self_reference_ids.push(*id),
                    Some(pid) if !known_dept_ids.contains(pid) => {
                        dangling_parent_ids.push((*id, *pid));
                    }
                    Some(pid) if on_cycle_ids.contains(pid) => cycle_ids.push(*id),
                    Some(pid) if stuck_id_set.contains(pid) => unreachable_parent_ids.push(*id),
                    // 依据：父级行存在且可达时，父级的组装必然遍历到本节点并把本节点取走，
                    // 本节点不会滞留；走到这里说明父子映射与根收集不一致，如实点名不并入正常类别。
                    Some(pid) => mapping_anomalies.push(format!(
                        "id={id} 的父级 {pid} 可达而本节点却滞留，根收集与父子映射不一致"
                    )),
                }
            }
            let joined_ids = |ids: &[i32]| -> String {
                ids.iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let stuck_detail: Vec<String> = stuck_nodes
                .iter()
                .map(|(id, parent)| match parent {
                    Some(pid) => format!("id={id} 的父级 parent_id={pid}"),
                    None => format!("id={id} 的父级为空"),
                })
                .collect();
            tracing::warn!(
                "部门树组装：{} 个部门父级缺席或层级成环，从根节点不可达，按根节点返回（parent_id 保持原值，不代表层级正确）：{}",
                stuck_nodes.len(),
                stuck_detail.join("，")
            );
            if !self_reference_ids.is_empty() {
                tracing::warn!(
                    "部门树组装：其中 {} 个节点父级指向自身（自引用成环），id={}",
                    self_reference_ids.len(),
                    joined_ids(&self_reference_ids)
                );
            }
            if !cycle_ids.is_empty() {
                tracing::warn!(
                    "部门树组装：其中 {} 个节点父级指向环内其他部门（层级成环），id={}",
                    cycle_ids.len(),
                    joined_ids(&cycle_ids)
                );
            }
            if !unreachable_parent_ids.is_empty() {
                tracing::warn!(
                    "部门树组装：其中 {} 个节点自身不在环上、但父级同样从根不可达（沿父级上溯终止于父级缺席或成环），随父级按根节点返回，id={}",
                    unreachable_parent_ids.len(),
                    joined_ids(&unreachable_parent_ids)
                );
            }
            if !dangling_parent_ids.is_empty() {
                let dangling_detail: Vec<String> = dangling_parent_ids
                    .iter()
                    .map(|(id, pid)| format!("id={id} 的父部门 {pid} 不存在"))
                    .collect();
                tracing::warn!(
                    "部门树组装：其中 {} 个节点的父级部门不存在（悬挂外键）：{}",
                    dangling_parent_ids.len(),
                    dangling_detail.join("，")
                );
            }
            if !mapping_anomalies.is_empty() {
                tracing::warn!(
                    "部门树组装异常：{} 个节点的父子映射与根收集不一致，已按根节点返回：{}",
                    mapping_anomalies.len(),
                    mapping_anomalies.join("，")
                );
            }

            // 回收：对每个滞留 id 复用同一个 assemble。assemble 以 dept_map.remove 取节点，
            // 已被取走的 id 再进来只会返回 None，因此每个节点最多被取走一次：环上后继节点
            // 不会因成环被无限展开，也不会既作为父节点的子节点、又作为顶层节点各出现一次；
            // 环下整条子树随环上首个被取走的节点一起带出，不丢失。
            for (id, _) in &stuck_nodes {
                if let Some(node) = assemble(*id, &mut dept_map, &parent_to_children) {
                    tree.push(node);
                }
            }

            // 回收后 dept_map 必须被取空：每个滞留 id 都被尝试取走一次，取不到的只会是
            // 已被同批祖先带走的。仍有残留就说明有节点没被带出，如实点名而不是悄悄少给。
            if !dept_map.is_empty() {
                let mut leaked_ids: Vec<i32> = dept_map.keys().copied().collect();
                leaked_ids.sort_unstable();
                tracing::warn!(
                    "部门树组装异常：回收后仍有 {} 个节点未被带出，id={}",
                    leaked_ids.len(),
                    joined_ids(&leaked_ids)
                );
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

// V15 P0-S01 修复：行级数据权限工具模块
//
// 提供 apply_data_scope 工具函数，在 service 查询入口注入行级过滤条件。
// 数据范围三级模型：
//   all  - 全部数据（管理员/总经理）
//   dept - 本部门数据（部门经理）
//   self - 仅本人数据（普通员工）
//
// 使用方式：
//   let scope = DataScope::from_role(&role);
//   let condition = apply_data_scope(scope, auth.user_id, auth.department_id, "created_by", "department_id");
//   let query = Entity::find().filter(condition);

use sea_orm::{ColumnTrait, Condition, QueryFilter, Value};

/// 数据范围枚举（行级数据权限，取值与 role 表 data_scope 对应：All 全部/Dept 本部门/Self 仅本人）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataScope {
    /// 全部数据（管理员/总经理）
    All,
    /// 本部门数据（部门经理）
    Dept,
    /// 仅本人数据（普通员工）
    Self_,
}

impl DataScope {
    /// 从 role 表 data_scope 字段字符串解析（支持 all/dept/self 不区分大小写，未知值回退 Self_；方法名 parse_scope 避免 FromStr 冲突）
    pub fn parse_scope(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "all" => DataScope::All,
            "dept" => DataScope::Dept,
            _ => DataScope::Self_,
        }
    }

    /// 从 role model 提取数据范围
    pub fn from_role(role: &crate::models::role::Model) -> Self {
        Self::parse_scope(&role.data_scope)
    }

    /// 转为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            DataScope::All => "all",
            DataScope::Dept => "dept",
            DataScope::Self_ => "self",
        }
    }
}

/// 行级数据权限过滤参数（封装数据范围和身份信息用于 apply_data_scope）
#[derive(Debug, Clone)]
pub struct DataScopeContext {
    /// 数据范围（all/dept/self）
    pub scope: DataScope,
    /// 当前用户 ID
    pub user_id: i32,
    /// 当前用户部门 ID（dept 范围时使用，None 时退化为 self）
    pub department_id: Option<i32>,
    /// 可见部门 ID 集合（m_rls_dept_domain：主部门 + 兼职 + 子部门，仅 dept 用户加载）。
    /// 供 check_resource_owner 校验资源 department_id 归属，与 RLS 策略
    /// `department_id = ANY(app_dept_ids())` 同口径。
    pub dept_ids: Vec<i32>,
    /// 可见部门的成员用户 ID 集合（含本人；dept_ids 为空时退化为 [user_id]）。
    /// 应用层列表过滤统一按「归属人 ∈ 成员集合」判断——因 DB 触发器保证
    /// RLS 表 department_id 恒等于归属人部门，该语义与 RLS 策略等价，且对
    /// 无 department_id 列的非 RLS 表同样正确。
    pub dept_member_user_ids: Vec<i32>,
}

/// 应用行级数据权限过滤条件（All 返回空 Condition，Dept 按归属人 IN 可见部门成员集合过滤
/// （无成员时退化为 self），Self_ 按 owner 列过滤；参数 ctx/owner_column/department_column，
/// 返回 Condition 可直接用于 .filter()）
///
/// m_rls_dept_domain 修复两段历史缺陷：
/// 1. 原 dept 分支用 dept_column.eq(dept_id) 把 owner 列误传为 dept 列（created_by =
///    department_id 错位，恒空集）；
/// 2. dept 分支语义统一为「归属人 ∈ 可见部门成员用户集合」——因 DB 触发器保证
///    RLS 表 department_id 恒等于归属人部门，该过滤与 RLS 策略
///    `department_id = ANY(app_dept_ids())` 等价；department_column 参数保留用于
///    RLS 表调用点直接按列过滤（见 apply_data_scope_on_department）。
pub fn build_data_scope_condition<T, U>(
    ctx: &DataScopeContext,
    owner_column: T,
    _department_column: U,
) -> Condition
where
    T: ColumnTrait,
    U: ColumnTrait,
{
    match ctx.scope {
        DataScope::All => {
            // 全部数据：不添加任何过滤条件
            Condition::all()
        }
        DataScope::Dept => {
            // 本部门数据：归属人 ∈ 可见部门成员用户集合（含本人）
            if ctx.dept_member_user_ids.is_empty() {
                // 无可见成员退化为 self
                Condition::all().add(owner_column.eq(ctx.user_id))
            } else {
                Condition::all().add(owner_column.is_in(ctx.dept_member_user_ids.clone()))
            }
        }
        DataScope::Self_ => {
            // 仅本人数据：按用户 ID 过滤
            Condition::all().add(owner_column.eq(ctx.user_id))
        }
    }
}

/// RLS 表专用：dept 分支按数据部门列 IN 可见部门集合过滤（列上可走索引）。
/// 仅适用于 m_rls_dept_domain 已加 department_id 列的 5 张表调用点；
/// 非 RLS 表调用 build_data_scope_condition（归属人成员集合语义）。
pub fn build_department_scope_condition<T, U>(
    ctx: &DataScopeContext,
    owner_column: T,
    department_column: U,
) -> Condition
where
    T: ColumnTrait,
    U: ColumnTrait,
{
    use sea_orm::sea_query::ExprTrait;

    match ctx.scope {
        DataScope::All => Condition::all(),
        DataScope::Dept => {
            // 本人行 OR 数据部门 ∈ 可见部门集合（与 RLS 策略 USING 的 OR 组合一致）
            let self_branch = owner_column.eq(ctx.user_id);
            let dept_branch = if ctx.dept_ids.is_empty() {
                // 无可见部门：仅本人（退化为 self）
                self_branch
            } else {
                self_branch.or(department_column.is_in(ctx.dept_ids.clone()))
            };
            Condition::all().add(dept_branch)
        }
        DataScope::Self_ => Condition::all().add(owner_column.eq(ctx.user_id)),
    }
}

/// 校验资源归属（IDOR 防护）：用于 /:id handler 校验访问权限，参数 ctx/resource_owner_id/resource_dept_id
/// 规则：All=始终通过；Dept=资源部门 ID ∈ 可见部门集合通过；Self_=资源归属人 ID 与用户 ID 匹配通过；false 应返回 403
///
/// m_rls_dept_domain：Dept 分支从「单部门 ID 匹配」改为「资源部门 ID ∈ 可见部门集合」，
/// 与 RLS 策略 dept 分支口径一致。
pub fn check_resource_owner(
    ctx: &DataScopeContext,
    resource_owner_id: Option<i32>,
    resource_dept_id: Option<i32>,
) -> bool {
    match ctx.scope {
        DataScope::All => true,
        DataScope::Dept => {
            // 本部门数据：资源部门 ID ∈ 可见部门集合
            match resource_dept_id {
                Some(dept_id) => ctx.dept_ids.contains(&dept_id),
                None => false,
            }
        }
        DataScope::Self_ => {
            // 仅本人数据：归属人 ID 匹配
            match resource_owner_id {
                Some(owner_id) => owner_id == ctx.user_id,
                None => false,
            }
        }
    }
}

/// 为查询构建器应用数据范围过滤（便捷方法，= build_data_scope_condition + query.filter）。
/// dept 分支按「归属人 ∈ 可见部门成员用户集合」过滤，对全部业务表通用（含无
/// department_id 列的表）；RLS 5 表可改用 apply_department_scope 走 department_id
/// 列过滤（可索引，与 RLS 策略同形态）。
pub fn apply_data_scope<E, T, U>(
    query: sea_orm::Select<E>,
    ctx: &DataScopeContext,
    owner_column: T,
    dept_column: U,
) -> sea_orm::Select<E>
where
    E: sea_orm::EntityTrait,
    T: ColumnTrait,
    U: ColumnTrait,
{
    let condition = build_data_scope_condition(ctx, owner_column, dept_column);
    query.filter(condition)
}

/// RLS 5 表专用（m_rls_dept_domain）：dept 分支按 department_id 列 IN 可见部门集合
/// + self 分支 OR 组合，与 RLS 策略 USING 同形态（列上可走索引）。
/// 适用于无公海语义的 RLS 表（suppliers/sales_orders/crm_opportunity）；
/// 有公海分支的表（customers/crm_lead）用 apply_department_scope_with_pool。
pub fn apply_department_scope<E, T, U>(
    query: sea_orm::Select<E>,
    ctx: &DataScopeContext,
    owner_column: T,
    department_column: U,
) -> sea_orm::Select<E>
where
    E: sea_orm::EntityTrait,
    T: ColumnTrait,
    U: ColumnTrait,
{
    let condition = build_department_scope_condition(ctx, owner_column, department_column);
    query.filter(condition)
}

/// RLS 5 表专用（m_rls_dept_domain）：dept 分支按 department_id 列 IN 可见部门集合
/// + self 分支 + 公海分支 OR 组合，与 RLS 策略 USING 完全同形态（列上可走索引）。
/// 公海行（customers owner_id=0 / crm_lead lead_status='pool'）由 RLS 放行，
/// 应用层需同口径放行，避免列表过滤遮蔽公海数据。
/// 示例：apply_department_scope_with_pool(customer::Entity::find(), &ctx,
///         customer::Column::OwnerId, customer::Column::DepartmentId,
///         customer::Column::OwnerId.eq(0))
pub fn apply_department_scope_with_pool<E, T, U>(
    query: sea_orm::Select<E>,
    ctx: &DataScopeContext,
    owner_column: T,
    department_column: U,
    pool_condition: sea_orm::sea_query::Expr,
) -> sea_orm::Select<E>
where
    E: sea_orm::EntityTrait,
    T: ColumnTrait,
    U: ColumnTrait,
{
    let mut condition = build_department_scope_condition(ctx, owner_column, department_column);
    if ctx.scope == DataScope::Dept && !ctx.dept_ids.is_empty() {
        // 公海行放行（与 RLS 策略公海分支同口径）
        condition = condition.add(pool_condition);
    }
    query.filter(condition)
}

/// V15 P0-B10：为 raw SQL 查询构建数据范围过滤片段（用于 Statement::from_sql_and_values 场景，返回可拼接到 WHERE 的 SQL 片段 + 绑定参数）
/// BI 模块 16 个 raw SQL 查询统一过滤；参数 ctx/table_alias(s|sales_orders|""→AND <alias>.created_by=$N)/next_index；行为 All=空片段/Dept=EXISTS 关联 users 过滤部门/Self_=created_by=user_id
pub fn build_data_scope_sql(
    ctx: &DataScopeContext,
    table_alias: &str,
    next_index: usize,
) -> (String, Vec<Value>) {
    let prefix = if table_alias.is_empty() {
        String::new()
    } else {
        format!("{}.", table_alias)
    };

    match ctx.scope {
        DataScope::All => {
            // 全部数据：不添加任何过滤条件
            (String::new(), Vec::new())
        }
        DataScope::Dept => {
            // 本部门数据：与 RLS 策略 USING 同形态（OR 组合）——
            // 本人行 OR 数据部门 ∈ 可见部门集合（m_rls_dept_domain，department_id
            // 为 5 表新增冗余列，触发器保证恒等于归属人部门）。
            // user_id 以字面量拼入（i32，无注入面），dept_ids 走单占位符——
            // 固定消耗 1 个参数位，保证与调用方的 next_index 编号约定兼容
            //（profit.rs 等存在相邻 scope_sql 调用，双占位符会撞号）。
            let self_sql = format!(
                "{prefix}created_by = {user_id}",
                prefix = prefix,
                user_id = ctx.user_id,
            );
            if ctx.dept_ids.is_empty() {
                // 无可见部门退化为 self
                (format!("AND {self_sql}"), Vec::new())
            } else {
                // PG int[] 数组绑定：Value::Array(ArrayType::Int, Some(Box(vec)))
                use sea_orm::sea_query::ArrayType;
                let arr = Value::Array(
                    ArrayType::Int,
                    Some(Box::new(
                        ctx.dept_ids.iter().map(|id| Value::Int(Some(*id))).collect(),
                    )),
                );
                (
                    format!(
                        "AND ({self_sql} OR {prefix}department_id = ANY(${next_index}::int[]))",
                        self_sql = self_sql,
                        prefix = prefix,
                        next_index = next_index,
                    ),
                    vec![arr],
                )
            }
        }
        DataScope::Self_ => {
            // 仅本人数据：按 created_by = user_id 过滤
            let sql = format!(
                "AND {prefix}created_by = ${next_index}",
                prefix = prefix,
                next_index = next_index,
            );
            (sql, vec![Value::Int(Some(ctx.user_id))])
        }
    }
}

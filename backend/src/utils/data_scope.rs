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
///
/// 与公海可见集（`PoolVisibility::Open`）的关系（写侧边界，**刻意不随读侧放开**）：
/// 本函数只作用于**私海归属语义**的行；公海行在各写入口（如
/// `crm_pool_handler::claim_from_pool`）有自己的公海分支，不套用本门——读侧
/// "多看到公海行"与写侧"能改哪一行"是两个判定，领取/回收/合并的越权门均维持
/// 现状（All 可越界、Dept 需资源部门在可见集合内、Self 仅本人行），未做任何放松。
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

/// 写侧归属门（用户 2026-10-02 裁定**方案 A**：读可 All，写须 owner 或显式
/// 「管理员代操作」权限键 + 留痕）。
///
/// 为什么读侧和写侧必须分家：`DataScope::All` 的本职是"能看全库"（报表/审计/主管
/// 复核），把它同时当成"能改任何人的行"就把可见性升级成了可变更权——水平越权缺陷
/// 正是从这里长出来的。ERP 惯例（超管/审计角色可读全域，代他人操作须显式授权并
/// 留痕）也按此分家。
///
/// 判定：
/// - **本人行恒可写**（三种 scope 都一样，own row 不需要代操作键）；
/// - `All`：跨 owner 写只有 `behalf_granted`（持有 `crm_data/cross_owner_write` 键）
///   才放行；未授予即 403（出参仍是固定脱敏常量，权限文案永久脱敏是硬令）；
/// - `Dept`：资源部门 ∈ 可见部门集合（部门经理代管本部门是 dept 的本职，不需额外键）；
/// - `Self_`：仅本人行。
///
/// 留痕：走 `behalf_granted` 的跨 owner 写由调用方（handler）在写前打
/// `crm 代操作写` 结构化日志（含 actor/role/resource/owner），且各写入口本身都经
/// `AuditLogService::update_with_audit` 落审计行，actor 即操作人，可回溯。
pub fn check_resource_write_owner(
    ctx: &DataScopeContext,
    resource_owner_id: Option<i32>,
    resource_dept_id: Option<i32>,
    behalf_granted: bool,
) -> bool {
    if resource_owner_id.is_some_and(|owner| owner == ctx.user_id) {
        return true;
    }
    match ctx.scope {
        DataScope::All => behalf_granted,
        DataScope::Dept => match resource_dept_id {
            Some(dept_id) => ctx.dept_ids.contains(&dept_id),
            None => false,
        },
        DataScope::Self_ => false,
    }
}

/// 公海行可见性组合方式（`apply_department_scope_with_pool` 的显式口径参数）。
///
/// 存在意义：公海放行与 DB 层 RLS 的组合形态必须逐表拍板，不能靠函数隐含。
/// DB 层 RLS 的公海分支本来就是**独立的 OR 支**（`lead_status='pool'` /
/// `owner_id=0`，migration `domain/rls_dept/mod.rs:146-159`、`:195-207`），
/// 应用层若把公海条件 AND 进归属条件，只会遮蔽 DB 本可放行的行（收窄可见集），
/// 永远不会放大 DB 边界——因此 `Open` 不是放松权限，而是与 DB 同源纠偏。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolVisibility {
    /// 公海行可见性与归属条件**相互独立**（OR 组合，与 RLS USING 的公海 OR 支
    /// 同形态）：`归属条件 OR 公海条件`。Dept/Self 用户都能看全公海行；
    /// All 本就不加行级过滤。`crm_lead` 调用点使用（用户 2026-10-02 拍板 ②）。
    Open,
    /// 历史组合形态：公海条件仅在 Dept 分支（有可见部门时）AND 进归属条件，
    /// Self/All 分支无公海放行。`customers` 调用点本轮保持该口径——
    /// **客户公海（owner_id=0）是否同步放开另待拍板，本轮可见面零变化**。
    Scoped,
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

/// RLS 5 表专用（m_rls_dept_domain）：dept 分支按 department_id 列 IN 可见部门集合，
/// 与 self 分支做 OR 组合，与 RLS 策略 USING 同形态（列上可走索引）。
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

/// RLS 5 表专用（m_rls_dept_domain）：带公海分支的数据范围过滤，组合口径由
/// `PoolVisibility` **显式传入**，函数本身不再隐含公海语义：
/// - [`PoolVisibility::Open`]（crm_lead 调用点，用户 2026-10-02 拍板 ②）：
///   `归属条件 OR 公海条件`——公海行可见性与归属条件相互独立，与 DB 层 RLS
///   USING 的独立公海 OR 支同形态（`rls_dept/mod.rs:195-207`
///   `lead_status='pool'`、`:146-159` customers `owner_id=0`）。修复前 Dept 分支
///   为 `(self ∪ dept) AND 公海条件`：公海页查不到本应可见的公海行，且 Dept 用户
///   **领取后**该行 `lead_status` 变 `new` 反而跌出可见集（回收→领取→再查看
///   的合法链被遮蔽）。改 OR **不扩大 DB 边界**（DB 早已放行），只是应用层不再
///   遮蔽本可出行的；写侧越权门 `check_resource_owner` 逐字不变（见其文档）。
/// - [`PoolVisibility::Scoped`]（customers 调用点，本轮**可见面零变化**）：
///   保持历史组合——公海条件仅在 Dept 分支且有可见部门时 AND 进归属条件，
///   Self/All 无公海放行。客户公海（`owner_id=0`）是否同步放开属另一待拍板项，
///   不得顺手套用 Open。
///
/// 示例：`apply_department_scope_with_pool(crm_lead::Entity::find(), &ctx,
///         crm_lead::Column::OwnerId, crm_lead::Column::DepartmentId,
///         crm_lead::Column::LeadStatus.eq(lead_status::POOL),
///         PoolVisibility::Open)`
pub fn apply_department_scope_with_pool<E, T, U>(
    query: sea_orm::Select<E>,
    ctx: &DataScopeContext,
    owner_column: T,
    department_column: U,
    pool_condition: sea_orm::sea_query::Expr,
    pool_visibility: PoolVisibility,
) -> sea_orm::Select<E>
where
    E: sea_orm::EntityTrait,
    T: ColumnTrait,
    U: ColumnTrait,
{
    let condition = build_department_scope_with_pool_condition(
        ctx,
        owner_column,
        department_column,
        pool_condition,
        pool_visibility,
    );
    query.filter(condition)
}

/// `apply_department_scope_with_pool` 的条件构造内核（与外层过滤应用分离，
/// 谓词形态可被 `tests/utils_data_scope_test.rs` 直接断言——防"组合形态只在
/// 文档里、实际 SQL 无人核验"的假绿）。
pub fn build_department_scope_with_pool_condition<T, U>(
    ctx: &DataScopeContext,
    owner_column: T,
    department_column: U,
    pool_condition: sea_orm::sea_query::Expr,
    pool_visibility: PoolVisibility,
) -> Condition
where
    T: ColumnTrait,
    U: ColumnTrait,
{
    let scope_condition = build_department_scope_condition(ctx, owner_column, department_column);
    match pool_visibility {
        PoolVisibility::Open => match ctx.scope {
            // All：行级本无归属过滤，公海放行无从再 OR（空 Condition = 全量可见）
            DataScope::All => Condition::all(),
            // 归属条件与公海条件相互独立（OR），与 RLS USING 同形态
            _ => Condition::any().add(scope_condition).add(pool_condition),
        },
        // 历史组合，逐字符保持现状（customers 可见面本轮零变化）
        PoolVisibility::Scoped => {
            let mut condition = scope_condition;
            if ctx.scope == DataScope::Dept && !ctx.dept_ids.is_empty() {
                condition = condition.add(pool_condition);
            }
            condition
        }
    }
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
                        ctx.dept_ids
                            .iter()
                            .map(|id| Value::Int(Some(*id)))
                            .collect(),
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

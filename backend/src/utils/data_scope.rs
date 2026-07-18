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

/// 数据范围枚举（行级数据权限）
///
/// 取值与 role 表 data_scope 字段对应：
/// - All：全部数据（管理员/总经理）
/// - Dept：本部门数据（部门经理）
/// - Self：仅本人数据（普通员工）
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
    /// 从 role 表 data_scope 字段字符串解析
    ///
    /// 支持的值：all / dept / self（不区分大小写）
    /// 未知值默认回退到 Self_（最小权限原则）
    ///
    /// V15 clippy 修复：方法名从 from_str 改为 parse_scope，
    /// 避免与标准库 trait std::str::FromStr::from_str 冲突。
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

/// 行级数据权限过滤参数
///
/// 封装当前用户的数据范围和身份信息，用于 apply_data_scope 函数。
#[derive(Debug, Clone)]
pub struct DataScopeContext {
    /// 数据范围（all/dept/self）
    pub scope: DataScope,
    /// 当前用户 ID
    pub user_id: i32,
    /// 当前用户部门 ID（dept 范围时使用，None 时退化为 self）
    pub department_id: Option<i32>,
}

/// 应用行级数据权限过滤条件
///
/// 根据数据范围生成对应的过滤条件：
/// - All：不添加任何过滤（返回空 Condition，查询全部数据）
/// - Dept：按部门 ID 过滤（department_id 为 None 时退化为 self）
/// - Self_：按用户 ID 过滤（created_by = user_id）
///
/// 参数说明：
/// - `ctx`：数据范围上下文（scope + user_id + department_id）
/// - `owner_column`：资源归属人列（如 customer::Column::CreatedBy）
/// - `dept_column`：资源归属部门列（如 customer::Column::DepartmentId）
///
/// 返回 Condition，可直接用于 .filter() 或 .filter(condition.add(...))
///
/// 使用示例：
/// ```ignore
/// let ctx = DataScopeContext { scope: DataScope::Self_, user_id: 1, department_id: Some(10) };
/// let condition = build_data_scope_condition(
///     &ctx,
///     customer::Column::CreatedBy,
///     customer::Column::DepartmentId,
/// );
/// let query = customer::Entity::find().filter(condition);
/// ```
pub fn build_data_scope_condition<T, U>(
    ctx: &DataScopeContext,
    owner_column: T,
    dept_column: U,
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
            // 本部门数据：按部门 ID 过滤
            // 若用户无部门，退化为 self（按用户 ID 过滤）
            if let Some(dept_id) = ctx.department_id {
                Condition::all().add(dept_column.eq(dept_id))
            } else {
                Condition::all().add(owner_column.eq(ctx.user_id))
            }
        }
        DataScope::Self_ => {
            // 仅本人数据：按用户 ID 过滤
            Condition::all().add(owner_column.eq(ctx.user_id))
        }
    }
}

/// 校验资源归属（IDOR 防护）
///
/// 用于 /:id handler，校验当前用户是否有权访问指定资源。
///
/// 参数说明：
/// - `ctx`：数据范围上下文
/// - `resource_owner_id`：资源的归属人 ID（如 customer.created_by）
/// - `resource_dept_id`：资源的归属部门 ID（如 customer.department_id）
///
/// 返回 true 表示有权访问，false 表示无权访问（应返回 403）。
///
/// 规则：
/// - All：始终返回 true
/// - Dept：资源部门 ID 与用户部门 ID 匹配时返回 true
/// - Self_：资源归属人 ID 与用户 ID 匹配时返回 true
pub fn check_resource_owner(
    ctx: &DataScopeContext,
    resource_owner_id: Option<i32>,
    resource_dept_id: Option<i32>,
) -> bool {
    match ctx.scope {
        DataScope::All => true,
        DataScope::Dept => {
            // 本部门数据：部门 ID 匹配
            match (ctx.department_id, resource_dept_id) {
                (Some(user_dept), Some(res_dept)) => user_dept == res_dept,
                _ => false,
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

/// 为查询构建器应用数据范围过滤（便捷方法）
///
/// 这是 build_data_scope_condition + query.filter 的组合便捷方法。
///
/// 使用示例：
/// ```ignore
/// let ctx = DataScopeContext { scope: DataScope::Self_, user_id: 1, department_id: Some(10) };
/// let query = apply_data_scope(
///     customer::Entity::find(),
///     &ctx,
///     customer::Column::CreatedBy,
///     customer::Column::DepartmentId,
/// );
/// ```
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

/// V15 P0-B10：为 raw SQL 查询构建数据范围过滤片段
///
/// 与 `apply_data_scope` 不同，此函数用于 `Statement::from_sql_and_values` 的 raw SQL 场景，
/// 返回可直接拼接到 WHERE 子句的 SQL 片段和对应的绑定参数。
///
/// 业务背景：
///   BI 模块的 16 个查询方法使用 raw SQL（Statement::from_sql_and_values），
///   原实现无任何数据权限过滤，所有用户都能看到全部销售数据。
///   此函数为这些 raw SQL 查询提供统一的数据范围过滤能力。
///
/// 参数说明：
/// - `ctx`：数据范围上下文（scope + user_id + department_id）
/// - `table_alias`：sales_orders 表在 SQL 中的别名（如 "s"、"sales_orders"、""）
///   - "s" → 生成 "AND s.created_by = $N"
///   - "sales_orders" → 生成 "AND sales_orders.created_by = $N"
///   - "" → 生成 "AND created_by = $N"
/// - `next_index`：下一个可用的参数占位符索引（如现有查询用 $1/$2，则传 3）
///
/// 返回 (sql_fragment, bind_values)：
/// - sql_fragment：可直接拼接到 WHERE 子句的 SQL 片段（All 范围为空字符串）
/// - bind_values：对应的参数值（All 范围为空 Vec），需追加到原查询的参数列表
///
/// 数据范围行为：
/// - All：返回空片段（不过滤）
/// - Dept：通过 EXISTS 子查询关联 users 表过滤部门（用户无部门时退化为 self）
/// - Self_：按 created_by = user_id 过滤
///
/// 使用示例：
/// ```ignore
/// let ctx = auth.to_data_scope_context();
/// let (scope_sql, scope_values) = build_data_scope_sql(&ctx, "s", 3);
/// let sql = format!(
///     "SELECT ... FROM sales_orders s WHERE s.status != 'CANCELLED' {scope_sql}",
///     scope_sql = scope_sql,
/// );
/// let mut values = vec![start_date.into(), end_date.into()];
/// values.extend(scope_values);
/// let stmt = Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values);
/// ```
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
            // 本部门数据：通过 EXISTS 子查询关联 users 表
            // 若用户无部门，退化为 self（按 created_by = user_id 过滤）
            if let Some(dept_id) = ctx.department_id {
                let sql = format!(
                    "AND EXISTS (SELECT 1 FROM users u WHERE u.id = {prefix}created_by AND u.department_id = ${next_index})",
                    prefix = prefix,
                    next_index = next_index,
                );
                (sql, vec![Value::Int(Some(dept_id))])
            } else {
                // 用户无部门时退化为 self（最小权限原则）
                let sql = format!(
                    "AND {prefix}created_by = ${next_index}",
                    prefix = prefix,
                    next_index = next_index,
                );
                (sql, vec![Value::Int(Some(ctx.user_id))])
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== DataScope::parse_scope 测试 =====

    #[test]
    fn test_data_scope_parse_scope_all() {
        assert_eq!(DataScope::parse_scope("all"), DataScope::All);
        assert_eq!(DataScope::parse_scope("ALL"), DataScope::All);
        assert_eq!(DataScope::parse_scope("All"), DataScope::All);
    }

    #[test]
    fn test_data_scope_parse_scope_dept() {
        assert_eq!(DataScope::parse_scope("dept"), DataScope::Dept);
        assert_eq!(DataScope::parse_scope("DEPT"), DataScope::Dept);
    }

    #[test]
    fn test_data_scope_parse_scope_self() {
        assert_eq!(DataScope::parse_scope("self"), DataScope::Self_);
        assert_eq!(DataScope::parse_scope("SELF"), DataScope::Self_);
    }

    #[test]
    fn test_data_scope_parse_scope_未知值默认self() {
        // 未知值应回退到 Self_（最小权限原则）
        assert_eq!(DataScope::parse_scope("unknown"), DataScope::Self_);
        assert_eq!(DataScope::parse_scope(""), DataScope::Self_);
        assert_eq!(DataScope::parse_scope("admin"), DataScope::Self_);
    }

    #[test]
    fn test_data_scope_as_str() {
        assert_eq!(DataScope::All.as_str(), "all");
        assert_eq!(DataScope::Dept.as_str(), "dept");
        assert_eq!(DataScope::Self_.as_str(), "self");
    }

    // ===== check_resource_owner 测试 =====

    #[test]
    fn test_check_resource_owner_all始终返回true() {
        let ctx = DataScopeContext {
            scope: DataScope::All,
            user_id: 1,
            department_id: Some(10),
        };
        // 无论资源归属如何，all 范围始终返回 true
        assert!(check_resource_owner(&ctx, Some(999), Some(999)));
        assert!(check_resource_owner(&ctx, None, None));
        assert!(check_resource_owner(&ctx, Some(1), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_dept部门匹配返回true() {
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(check_resource_owner(&ctx, Some(999), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_dept部门不匹配返回false() {
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, Some(1), Some(20)));
    }

    #[test]
    fn test_check_resource_owner_dept资源无部门返回false() {
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, Some(1), None));
    }

    #[test]
    fn test_check_resource_owner_dept用户无部门退化为false() {
        // 用户无部门时，dept 范围无法匹配，返回 false
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: None,
        };
        assert!(!check_resource_owner(&ctx, Some(1), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_self归属人匹配返回true() {
        let ctx = DataScopeContext {
            scope: DataScope::Self_,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(check_resource_owner(&ctx, Some(1), Some(20)));
    }

    #[test]
    fn test_check_resource_owner_self归属人不匹配返回false() {
        let ctx = DataScopeContext {
            scope: DataScope::Self_,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, Some(999), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_self资源无归属人返回false() {
        let ctx = DataScopeContext {
            scope: DataScope::Self_,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, None, Some(10)));
    }
}

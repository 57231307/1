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
}

/// 应用行级数据权限过滤条件（All 返回空 Condition，Dept 按 department_id 过滤 None 退化为 self，Self_ 按 created_by=user_id 过滤；ctx 上下文，owner_column 归属人列，dept_column 归属部门列，返回 Condition 可直接用于 .filter()）
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

/// 校验资源归属（IDOR 防护）：用于 /:id handler 校验访问权限，参数 ctx/resource_owner_id/resource_dept_id
/// 规则：All=始终通过；Dept=资源部门 ID 与用户部门匹配通过；Self_=资源归属人 ID 与用户 ID 匹配通过；false 应返回 403
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

/// 为查询构建器应用数据范围过滤（便捷方法，= build_data_scope_condition + query.filter）
/// 示例：apply_data_scope(customer::Entity::find(), &ctx, customer::Column::CreatedBy, customer::Column::DepartmentId)
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
    fn test_data_scope_parse_scope_wzzmr_self() {
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
    fn test_check_resource_owner_all_szfh_true() {
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
    fn test_check_resource_owner_dept_bmppfh_true() {
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(check_resource_owner(&ctx, Some(999), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_dept_bmbppfh_false() {
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, Some(1), Some(20)));
    }

    #[test]
    fn test_check_resource_owner_dept_zywbmfh_false() {
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, Some(1), None));
    }

    #[test]
    fn test_check_resource_owner_dept_yhwbmthw_false() {
        // 用户无部门时，dept 范围无法匹配，返回 false
        let ctx = DataScopeContext {
            scope: DataScope::Dept,
            user_id: 1,
            department_id: None,
        };
        assert!(!check_resource_owner(&ctx, Some(1), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_self_gsrppfh_true() {
        let ctx = DataScopeContext {
            scope: DataScope::Self_,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(check_resource_owner(&ctx, Some(1), Some(20)));
    }

    #[test]
    fn test_check_resource_owner_self_gsrbppfh_false() {
        let ctx = DataScopeContext {
            scope: DataScope::Self_,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, Some(999), Some(10)));
    }

    #[test]
    fn test_check_resource_owner_self_zywgsrfh_false() {
        let ctx = DataScopeContext {
            scope: DataScope::Self_,
            user_id: 1,
            department_id: Some(10),
        };
        assert!(!check_resource_owner(&ctx, None, Some(10)));
    }
}

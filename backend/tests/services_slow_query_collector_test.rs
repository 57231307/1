    use super::*;
#[cfg(test)]
mod tests {

    /// SQL 拼接：默认阈值与 limit（L2 修复后使用 $1/$2 占位符）
    #[test]
    fn test_build_query_sql_default() {
        let sql = build_query_sql(100.0, 100);
        assert!(sql.contains("WHERE mean_exec_time > $1"));
        assert!(sql.contains("LIMIT $2"));
        assert!(sql.contains("ORDER BY mean_exec_time DESC"));
        assert!(sql.starts_with("SELECT query, mean_exec_time, calls, rows"));
        assert!(sql.contains("FROM pg_stat_statements"));
    }

    /// SQL 拼接：自定义阈值与 limit（参数化后 SQL 固定不变）
    #[test]
    fn test_build_query_sql_custom() {
        let sql = build_query_sql(250.5, 50);
        assert!(sql.contains("> $1"));
        assert!(sql.contains("LIMIT $2"));
    }

    /// SQL 拼接：极值（参数化后 SQL 固定，与参数值无关）
    #[test]
    fn test_build_query_sql_extreme_values() {
        // 极小
        let sql_min = build_query_sql(0.001, 1);
        assert!(sql_min.contains("> $1"));
        assert!(sql_min.contains("LIMIT $2"));

        // 极大
        let sql_max = build_query_sql(1_000_000.0, 1_000_000);
        assert!(sql_max.contains("> $1"));
        assert!(sql_max.contains("LIMIT $2"));

        // 零值
        let sql_zero = build_query_sql(0.0, 0);
        assert!(sql_zero.contains("> $1"));
        assert!(sql_zero.contains("LIMIT $2"));
    }
}
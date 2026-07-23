//! OLAP 操作 Service impl 子模块（bi_analysis_ops/olap）
//!
//! 批次 490 D10-3a 拆分：从原 `bi_analysis_service.rs` L1271-1537 迁移。
//! 包含 BiAnalysisService 的 4 个公开方法 + 3 个私有 helper：
//! - slice / dice / rollup / pivot（4 公开方法）
//! - validate_pivot_params / execute_pivot_query / build_pivot_matrix（3 私有 helper）
//!
//! 业务规则：
//! - 切片（slice）：固定其他维度，单独分析一个维度
//! - 切块（dice）：多维范围筛选（按日期范围返回按日聚合）
//! - 上卷（rollup）：细粒度 → 粗粒度聚合（day → month 等）
//! - 透视（pivot）：行列转换，按 row_dim × col_dim 构建二维聚合矩阵
//! - V15 P0-B10：pivot 查询注入行级数据权限过滤

use sea_orm::{FromQueryResult, Statement};

use crate::services::bi_analysis_ops::types::PivotRow;
use crate::services::bi_analysis_service::{dec_to_f64, dim_to_expr, measure_to_expr, BiAnalysisService};
use crate::utils::error::AppError;

impl BiAnalysisService {
    /// 切片（固定其他维度，单独分析一个维度）
    ///
    /// 根据 dimension 调用对应的聚合方法，filters 作为附加过滤条件（当前实现忽略 filters，
    /// 仅按 dimension 返回聚合数据；后续迭代可解析 filters 构建动态 WHERE 子句）。
    pub async fn slice(
        &self,
        dimension: &str,
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let valid_dims = ["time", "customer", "product", "region", "category"];
        if !valid_dims.contains(&dimension) {
            return Err(AppError::validation(format!("不支持的维度: {}", dimension)));
        }

        let result = match dimension {
            "time" => {
                let end = chrono::Local::now().date_naive();
                let start = end - chrono::Duration::days(30);
                serde_json::to_value(self.sales_by_time(start, end, "day").await?)?
            }
            "customer" => serde_json::to_value(self.sales_by_customer(10).await?)?,
            "product" => serde_json::to_value(self.sales_by_product(10).await?)?,
            "region" => serde_json::to_value(self.sales_by_region().await?)?,
            "category" => serde_json::to_value(self.sales_by_category().await?)?,
            _ => serde_json::Value::Null,
        };

        Ok(serde_json::json!({
            "dimension": dimension,
            "filters": filters,
            "result": result,
        }))
    }

    /// 切块（多维范围筛选）
    ///
    /// 解析 filters 中的 date_from/date_to/customer_ids/product_ids 等条件，
    /// 返回符合所有条件的订单聚合数据。当前实现：返回指定日期范围内的按日聚合。
    pub async fn dice(
        &self,
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        // 解析可选的日期范围
        let date_from = filters
            .get("date_from")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let date_to = filters
            .get("date_to")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let end = date_to.unwrap_or(chrono::Local::now().date_naive());
        let start = date_from.unwrap_or(end - chrono::Duration::days(30));

        let data = self.sales_by_time(start, end, "day").await?;

        Ok(serde_json::json!({
            "filters": filters,
            "date_range": {
                "from": start.format("%Y-%m-%d").to_string(),
                "to": end.format("%Y-%m-%d").to_string(),
            },
            "result": data,
        }))
    }

    /// 上卷（细粒度 → 粗粒度）
    ///
    /// from_level → to_level 粒度聚合，例如 day → month。
    /// 当前实现：返回最近 90 天按 to_level 粒度的聚合数据。
    pub async fn rollup(
        &self,
        from_level: &str,
        to_level: &str,
    ) -> Result<serde_json::Value, AppError> {
        let valid_levels = ["day", "week", "month", "quarter", "year"];
        if !valid_levels.contains(&from_level) || !valid_levels.contains(&to_level) {
            return Err(AppError::validation("无效的粒度级别"));
        }

        let end = chrono::Local::now().date_naive();
        let start = end - chrono::Duration::days(90);
        let data = self.sales_by_time(start, end, to_level).await?;

        Ok(serde_json::json!({
            "from": from_level,
            "to": to_level,
            "date_range": {
                "from": start.format("%Y-%m-%d").to_string(),
                "to": end.format("%Y-%m-%d").to_string(),
            },
            "result": data,
        }))
    }

    /// 校验 pivot 参数（行/列维度、度量）
    fn validate_pivot_params(row_dim: &str, col_dim: &str, measure: &str) -> Result<(), AppError> {
        let valid_dims = ["customer", "product", "region", "category", "time"];
        if !valid_dims.contains(&row_dim) {
            return Err(AppError::validation(format!(
                "不支持的行维度: {}",
                row_dim
            )));
        }
        if !valid_dims.contains(&col_dim) {
            return Err(AppError::validation(format!(
                "不支持的列维度: {}",
                col_dim
            )));
        }
        if row_dim == col_dim {
            return Err(AppError::validation("行维度与列维度不能相同"));
        }

        let valid_measures = ["total_amount", "order_count", "quantity", "profit_amount"];
        if !valid_measures.contains(&measure) {
            return Err(AppError::validation(format!(
                "不支持的度量: {}",
                measure
            )));
        }
        Ok(())
    }

    /// 构建 pivot SQL 查询并执行，返回原始行
    async fn execute_pivot_query(
        &self,
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> Result<Vec<PivotRow>, AppError> {
        let (row_key_expr, row_label_expr) = dim_to_expr(row_dim)?;
        let (col_key_expr, col_label_expr) = dim_to_expr(col_dim)?;

        // 判断是否需要关联 sales_order_items（当任一维度为 product/category 时）
        let needs_items =
            matches!(row_dim, "product" | "category") || matches!(col_dim, "product" | "category");

        // 批次 252 修复：measure_to_expr 替代原内联 match + unreachable!()
        let (joins, measure_expr) = if needs_items {
            // 项级聚合：关联 sales_order_items / products / product_categories
            let joins = r#"
                LEFT JOIN sales_order_items si ON si.order_id = s.id
                LEFT JOIN products p ON p.id = si.product_id
                LEFT JOIN product_categories pc ON pc.id = p.category_id
            "#;
            let measure_expr = measure_to_expr(measure, true)?;
            (joins, measure_expr)
        } else {
            // 订单级聚合：不关联 sales_order_items，避免 total_amount 重复计算
            let joins = "";
            let measure_expr = measure_to_expr(measure, false)?;
            (joins, measure_expr)
        };

        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s，无其他参数，scope 从 $1 开始）
        let (scope_sql, scope_values) = self.scope_sql("s", 1);

        let sql = format!(
            r#"
            SELECT
                {row_key} as row_key,
                {row_label} as row_label,
                {col_key} as col_key,
                {col_label} as col_label,
                {measure} as measure_value
            FROM sales_orders s
            LEFT JOIN customers c ON c.id = s.customer_id
            {joins}
            WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
              {scope_sql}
            GROUP BY row_key, row_label, col_key, col_label
            ORDER BY row_label ASC, col_label ASC
            "#,
            row_key = row_key_expr,
            row_label = row_label_expr,
            col_key = col_key_expr,
            col_label = col_label_expr,
            measure = measure_expr,
            joins = joins,
            scope_sql = scope_sql,
        );

        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            scope_values,
        );

        PivotRow::find_by_statement(stmt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("透视查询执行失败: {}", e)))
    }

    /// 从查询结果构建交叉聚合矩阵
    fn build_pivot_matrix(
        rows: Vec<PivotRow>,
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> serde_json::Value {
        // 收集唯一的行/列键（保持有序），并构建矩阵
        let mut row_set: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut col_set: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut matrix: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

        for r in rows {
            let row_key = r.row_key.unwrap_or_default();
            let row_label = r.row_label.unwrap_or_else(|| row_key.clone());
            let col_key = r.col_key.unwrap_or_default();
            let col_label = r.col_label.unwrap_or_else(|| col_key.clone());
            let value = dec_to_f64(r.measure_value);

            row_set.entry(row_key.clone()).or_insert(row_label);
            col_set.entry(col_key.clone()).or_insert(col_label);
            matrix.insert(format!("{}|{}", row_key, col_key), value);
        }

        let rows_json: Vec<serde_json::Value> = row_set
            .iter()
            .map(|(k, v)| serde_json::json!({ "key": k, "label": v }))
            .collect();
        let cols_json: Vec<serde_json::Value> = col_set
            .iter()
            .map(|(k, v)| serde_json::json!({ "key": k, "label": v }))
            .collect();
        let matrix_json: serde_json::Value = matrix
            .into_iter()
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect();

        serde_json::json!({
            "row_dim": row_dim,
            "col_dim": col_dim,
            "measure": measure,
            "rows": rows_json,
            "columns": cols_json,
            "matrix": matrix_json,
        })
    }

    /// 透视（行列转换），按 row_dim × col_dim 构建二维聚合矩阵
    ///
    /// 实现说明（v11 批次 144 P1-3 修复）：
    /// - 原实现返回占位 note 字段，col 维度分组未实现
    /// - 现使用动态 SQL 构建真实的 row × col 交叉聚合矩阵
    /// - 当任一维度为 product/category 时，需要关联 sales_order_items 表进行项级聚合
    /// - 否则在订单级别聚合，避免因 JOIN 倍增导致 total_amount 重复计算
    pub async fn pivot(
        &self,
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_pivot_params(row_dim, col_dim, measure)?;

        let rows = self.execute_pivot_query(row_dim, col_dim, measure).await?;

        Ok(Self::build_pivot_matrix(rows, row_dim, col_dim, measure))
    }
}

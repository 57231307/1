//! 报表模板 Service
//!
//! 提供报表模板的CRUD操作和持久化功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::models::report_template::{
    ActiveModel, Entity as ReportTemplateEntity, Model as ReportTemplateModel,
};
use crate::utils::error::AppError;

/// 创建报表模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportTemplateRequest {
    pub name: String,
    pub code: String,
    pub report_type: String,
    pub template_id: Option<String>,
    pub category: Option<String>,
    pub data_source: Option<String>,
    pub columns: serde_json::Value,
    pub filters: Option<serde_json::Value>,
    pub parameters: Option<serde_json::Value>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub data_source_sql: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub supported_formats: Option<Vec<String>>,
}

/// 更新报表模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReportTemplateRequest {
    pub name: Option<String>,
    pub report_type: Option<String>,
    pub columns: Option<serde_json::Value>,
    pub filters: Option<serde_json::Value>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub data_source_sql: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub status: Option<String>,
}

/// 报表模板查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ReportTemplateQuery {
    pub report_type: Option<String>,
    #[allow(dead_code)] // TODO(tech-debt): 报表模板模块接入业务后移除
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 报表模板 Service
pub struct ReportTemplateService {
    db: Arc<DatabaseConnection>,
}

/// 敏感表列表 - 禁止通过自定义 SQL 访问
const SENSITIVE_TABLES: &[&str] = &[
    "users",
    "roles",
    "permissions",
    "audit_logs",
    "jti_blacklist",
    "system_config",
];

/// 危险 SQL 关键词 - 禁止在 SELECT 查询中使用
const DANGEROUS_KEYWORDS: &[&str] = &[
    "DELETE", "UPDATE", "INSERT", "DROP", "TRUNCATE", "ALTER", "CREATE", "EXEC", "EXECUTE",
    "GRANT", "REVOKE",
];

impl ReportTemplateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建报表模板
    pub async fn create(
        &self,
        tenant_id: i32,
        user_id: i32,
        role_id: Option<i32>,
        req: CreateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        // 安全检查：仅允许管理员提交自定义 SQL
        if req.data_source_sql.is_some() && role_id != Some(1) {
            return Err(AppError::permission_denied(
                "出于安全原因，仅系统管理员允许提交自定义 SQL 报表",
            ));
        }

        // 检查编码是否已存在
        let existing = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::TenantId.eq(tenant_id))
            .filter(crate::models::report_template::Column::Code.eq(&req.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!(
                "报表模板编码 {} 已存在",
                req.code
            )));
        }

        let now = Utc::now();
        let active_model = ActiveModel {
            id: Default::default(),
            tenant_id: Set(tenant_id),
            template_id: Set(req.template_id),
            name: Set(req.name),
            code: Set(req.code),
            report_type: Set(req.report_type),
            category: Set(req.category),
            data_source: Set(req.data_source),
            columns: Set(req.columns),
            filters: Set(req.filters),
            parameters: Set(req.parameters),
            sort_by: Set(req.sort_by),
            sort_order: Set(req.sort_order.or(Some("asc".to_string()))),
            data_source_sql: Set(req.data_source_sql),
            description: Set(req.description),
            is_public: Set(req.is_public.unwrap_or(false)),
            supported_formats: Set(req.supported_formats.map(sea_orm::JsonValue::from)),
            status: Set("ACTIVE".to_string()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 获取报表模板详情
    pub async fn get_by_id(
        &self,
        id: i32,
        tenant_id: i32,
        user_id: i32,
    ) -> Result<Option<ReportTemplateModel>, AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .filter(crate::models::report_template::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?;

        // 检查读取权限：公开或者自己创建的
        if let Some(ref t) = model {
            if !t.is_public && t.created_by != user_id {
                return Err(AppError::permission_denied("无权访问该私有报表模板"));
            }
        }

        Ok(model)
    }

    /// 更新报表模板
    pub async fn update(
        &self,
        id: i32,
        tenant_id: i32,
        user_id: i32,
        role_id: Option<i32>,
        req: UpdateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .filter(crate::models::report_template::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // 检查更新权限：只能更新自己创建的模板
        if model.created_by != user_id {
            return Err(AppError::permission_denied("只有创建者可以更新该报表模板"));
        }

        // 安全检查：仅允许管理员提交自定义 SQL
        if req.data_source_sql.is_some() && role_id != Some(1) {
            return Err(AppError::permission_denied(
                "出于安全原因，仅系统管理员允许提交自定义 SQL 报表",
            ));
        }

        let mut active_model: ActiveModel = model.into();

        if let Some(name) = req.name {
            active_model.name = Set(name);
        }
        if let Some(report_type) = req.report_type {
            active_model.report_type = Set(report_type);
        }
        if let Some(columns) = req.columns {
            active_model.columns = Set(columns);
        }
        if let Some(filters) = req.filters {
            active_model.filters = Set(Some(filters));
        }
        if let Some(sort_by) = req.sort_by {
            active_model.sort_by = Set(Some(sort_by));
        }
        if let Some(sort_order) = req.sort_order {
            active_model.sort_order = Set(Some(sort_order));
        }
        if let Some(data_source_sql) = req.data_source_sql {
            active_model.data_source_sql = Set(Some(data_source_sql));
        }
        if let Some(description) = req.description {
            active_model.description = Set(Some(description));
        }
        if let Some(is_public) = req.is_public {
            active_model.is_public = Set(is_public);
        }
        if let Some(status) = req.status {
            active_model.status = Set(status);
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除报表模板（软删除）
    pub async fn delete(&self, id: i32, tenant_id: i32, user_id: i32) -> Result<(), AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .filter(crate::models::report_template::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        if model.created_by != user_id {
            return Err(AppError::permission_denied("只有创建者可以删除该报表模板"));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("INACTIVE".to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 查询报表模板列表
    pub async fn list(
        &self,
        tenant_id: i32,
        user_id: i32,
        query: ReportTemplateQuery,
    ) -> Result<(Vec<ReportTemplateModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);

        let mut select = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::TenantId.eq(tenant_id))
            .filter(crate::models::report_template::Column::Status.eq("ACTIVE"));

        // 只显示公开模板或用户自己创建的模板
        select = select.filter(
            crate::models::report_template::Column::IsPublic
                .eq(true)
                .or(crate::models::report_template::Column::CreatedBy.eq(user_id)),
        );

        if let Some(report_type) = query.report_type {
            select =
                select.filter(crate::models::report_template::Column::ReportType.eq(report_type));
        }

        if let Some(keyword) = query.keyword {
            select = select.filter(
                crate::models::report_template::Column::Name
                    .contains(&keyword)
                    .or(crate::models::report_template::Column::Code.contains(&keyword)),
            );
        }

        let total = select.clone().count(&*self.db).await?;

        let items = select
            .order_by_desc(crate::models::report_template::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

        Ok((items, total))
    }

    /// 执行自定义报表
    pub async fn execute_custom_report(
        &self,
        template_id: i32,
        tenant_id: i32,
        user_id: i32,
        role_id: Option<i32>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        let template = self
            .get_by_id(template_id, tenant_id, user_id)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // 如果有自定义SQL，使用SQL执行
        if let Some(sql) = &template.data_source_sql {
            return self
                .execute_sql_report(sql, tenant_id, user_id, role_id, page, page_size)
                .await;
        }

        // 否则使用预定义的报表类型
        Err(AppError::business(
            "自定义报表需要配置数据源SQL".to_string(),
        ))
    }

    /// 执行自定义 SQL 报表（公开方法，供路由直接调用）
    pub async fn execute_sql_report(
        &self,
        sql: &str,
        tenant_id: i32,
        user_id: i32,
        role_id: Option<i32>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        use sea_orm::{ConnectionTrait, Statement};

        // 1. 权限检查：仅系统管理员可执行
        if role_id != Some(1) {
            return Err(AppError::permission_denied(
                "出于安全原因，仅系统管理员允许执行原始 SQL 报表",
            ));
        }

        // 2. 基础验证
        let sql_trimmed = sql.trim();
        if sql_trimmed.is_empty() {
            return Err(AppError::validation("SQL 语句不能为空"));
        }

        // 3. 只允许 SELECT 语句
        let sql_upper = sql_trimmed.to_uppercase();
        if !sql_upper.starts_with("SELECT") {
            return Err(AppError::validation("只允许 SELECT 查询语句"));
        }

        // 4. 禁止危险关键词
        Self::check_dangerous_keywords(&sql_upper)?;

        // 5. 检查敏感表访问
        Self::check_sensitive_tables(sql_trimmed)?;

        // 6. 强制添加租户 ID 过滤条件
        let filtered_sql = Self::add_tenant_filter(sql_trimmed, tenant_id)?;

        // 7. 记录审计日志
        Self::log_sql_execution(user_id, sql_trimmed, tenant_id);

        // 8. 添加分页
        let paginated_sql = format!(
            "{} LIMIT {} OFFSET {}",
            filtered_sql,
            page_size,
            (page - 1) * page_size
        );

        let stmt = Statement::from_string(sea_orm::DatabaseBackend::Postgres, paginated_sql);

        let result = self
            .db
            .as_ref()
            .query_all_raw(stmt)
            .await
            .map_err(|e| AppError::database(format!("SQL 执行失败：{}", e)))?;

        if result.is_empty() {
            return Ok((vec![], vec![], 0));
        }

        // 解析 QueryResult 的列和行
        let first_row = &result[0];
        let column_count = first_row.column_names().len();

        let headers: Vec<String> = (0..column_count)
            .map(|i| {
                first_row
                    .column_names()
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i))
            })
            .collect();

        let mut data: Vec<Vec<String>> = Vec::new();
        for row in &result {
            let mut row_data: Vec<String> = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for i in 0..column_count {
                let value: String = row.try_get("", &headers[i]).unwrap_or_default();
                row_data.push(value);
            }
            data.push(row_data);
        }

        let total = data.len() as u64;
        Ok((headers, data, total))
    }

    /// 检查 SQL 中是否包含危险关键词
    fn check_dangerous_keywords(sql_upper: &str) -> Result<(), AppError> {
        for keyword in DANGEROUS_KEYWORDS {
            // 使用单词边界检查，避免误判普通列名中包含关键词子串
            let pattern = format!(" {} ", keyword);
            let pattern_start = format!("{} ", keyword);
            let pattern_end = format!(" {}", keyword);
            if sql_upper.contains(&pattern)
                || sql_upper.starts_with(&pattern_start)
                || sql_upper.ends_with(&pattern_end)
                || sql_upper == *keyword
            {
                return Err(AppError::validation(format!(
                    "禁止使用危险关键词：{}",
                    keyword
                )));
            }
        }
        Ok(())
    }

    /// 检查 SQL 是否访问了敏感表
    fn check_sensitive_tables(sql: &str) -> Result<(), AppError> {
        let sql_lower = sql.to_lowercase();

        for table in SENSITIVE_TABLES {
            // 检查 FROM / JOIN 后是否引用了敏感表
            // 支持 "from table"、"join table"、"from table "、", table" 等模式
            let patterns = [
                format!("from {}", table),
                format!("join {}", table),
                format!(", {}", table),
            ];

            for pattern in &patterns {
                if sql_lower.contains(pattern.as_str()) {
                    return Err(AppError::permission_denied(format!(
                        "禁止访问敏感表：{}",
                        table
                    )));
                }
            }
        }

        Ok(())
    }

    /// 强制添加租户 ID 过滤条件，防止跨租户数据泄露
    fn add_tenant_filter(sql: &str, tenant_id: i32) -> Result<String, AppError> {
        let sql_upper = sql.to_uppercase();

        // 如果 SQL 中已包含 WHERE 子句，追加 AND 条件
        // 注意：需要处理子查询中也有 WHERE 的情况，这里采用简单策略
        // 在最外层追加过滤条件
        if sql_upper.contains(" WHERE ") {
            // 在已有 WHERE 基础上追加 AND 条件
            // 需要找到最后一个 WHERE 后面的位置来追加
            Ok(format!("{} AND tenant_id = {}", sql, tenant_id))
        } else {
            // 没有 WHERE 子句，在 FROM 子句后添加 WHERE
            // 查找可能的插入点：ORDER BY / GROUP BY / LIMIT / OFFSET / 末尾
            let insert_keywords = [
                " ORDER BY ",
                " GROUP BY ",
                " HAVING ",
                " LIMIT ",
                " OFFSET ",
            ];

            let mut insert_pos = sql.len();
            for kw in &insert_keywords {
                if let Some(pos) = sql_upper.rfind(kw) {
                    if pos < insert_pos {
                        insert_pos = pos;
                    }
                }
            }

            let (before, after) = sql.split_at(insert_pos);
            Ok(format!(
                "{} WHERE tenant_id = {}{}",
                before, tenant_id, after
            ))
        }
    }

    /// 记录 SQL 执行审计日志
    fn log_sql_execution(user_id: i32, sql: &str, tenant_id: i32) {
        tracing::warn!(
            user_id = user_id,
            tenant_id = tenant_id,
            sql = sql,
            "执行自定义 SQL 报表"
        );
        // TODO: 写入审计日志表
    }
}

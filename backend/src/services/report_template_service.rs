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
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ReportTemplateQuery {
    pub report_type: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 报表模板 Service
pub struct ReportTemplateService {
    db: Arc<DatabaseConnection>,
}

impl ReportTemplateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建报表模板
    pub async fn create(
        &self,
        tenant_id: i32,
        user_id: i32,
        req: CreateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
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
            supported_formats: Set(
                req.supported_formats
                    .map(sea_orm::JsonValue::from),
            ),
            status: Set("ACTIVE".to_string()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 获取报表模板详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReportTemplateModel>, AppError> {
        let model = ReportTemplateEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 更新报表模板
    pub async fn update(
        &self,
        id: i32,
        req: UpdateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        let model = ReportTemplateEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

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
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReportTemplateEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

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
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        let template = self
            .get_by_id(template_id)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // 如果有自定义SQL，使用SQL执行
        if let Some(sql) = &template.data_source_sql {
            return self.execute_sql_report(sql, page, page_size).await;
        }

        // 否则使用预定义的报表类型
        Err(AppError::business(
            "自定义报表需要配置数据源SQL".to_string(),
        ))
    }

    /// 执行SQL报表
    async fn execute_sql_report(
        &self,
        sql: &str,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        use sea_orm::{ConnectionTrait, Statement};

        // 安全检查：只允许SELECT语句
        let sql_upper = sql.trim().to_uppercase();
        if !sql_upper.starts_with("SELECT") {
            return Err(AppError::validation("只允许SELECT查询语句".to_string()));
        }

        // 添加分页
        let paginated_sql = format!(
            "{} LIMIT {} OFFSET {}",
            sql,
            page_size,
            (page - 1) * page_size
        );

        let stmt = Statement::from_string(sea_orm::DatabaseBackend::Postgres, paginated_sql);

        let result = self
            .db
            .as_ref()
            .query_all_raw(stmt)
            .await
            .map_err(|e| AppError::database(format!("SQL执行失败: {}", e)))?;

        if result.is_empty() {
            return Ok((vec![], vec![], 0));
        }

        // 解析QueryResult的列和行
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
}

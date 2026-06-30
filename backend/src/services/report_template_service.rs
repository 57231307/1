//! 报表模板 Service
//!
//! 提供报表模板的CRUD操作和持久化功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
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

// P0-B 安全修复：DANGEROUS_KEYWORDS / SENSITIVE_TABLES 常量及配套检查方法
// （check_dangerous_keywords / check_sensitive_tables / log_sql_execution）
// 全部删除。execute_sql_report 走 SimpleQuery 协议，黑名单无法阻止分号切割攻击；
// 统一在 create / update / execute 入口拒绝 data_source_sql，彻底关闭 SQL 注入攻击面。

impl ReportTemplateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建报表模板
    pub async fn create(
        &self,
        user_id: i32,
        _role_id: Option<i32>,
        req: CreateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        // P0-B 安全修复：彻底关闭"自定义 SQL 报表"入口。
        // 历史实现 execute_sql_report 通过 Statement::from_string + query_all 走 SimpleQuery
        // 协议，允许多语句执行；关键词黑名单 + starts_with("SELECT") 都不能阻止分号切割，
        // 攻击者可利用 `SELECT 1; DROP TABLE ...` 实现 SQL 注入。
        // 修复策略：禁止所有角色在 create/update 中提交 data_source_sql；
        // execute_custom_report 也不再调用 execute_sql_report，统一返回功能禁用错误。
        // 后续如需 SQL 报表能力，必须改用预定义白名单模板（report_type + 模板 ID），
        // 由后端硬编码 SQL，前端仅传参数。
        if req.data_source_sql.is_some() {
            return Err(AppError::permission_denied(
                "出于安全考虑，自定义 SQL 报表功能已禁用，请使用预定义报表模板".to_string(),
            ));
        }

        // 检查编码是否已存在
        let existing = ReportTemplateEntity::find()
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
        user_id: i32,
    ) -> Result<Option<ReportTemplateModel>, AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
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
        user_id: i32,
        _role_id: Option<i32>,
        req: UpdateReportTemplateRequest,
    ) -> Result<ReportTemplateModel, AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // 检查更新权限：只能更新自己创建的模板
        if model.created_by != user_id {
            return Err(AppError::permission_denied("只有创建者可以更新该报表模板"));
        }

        // P0-B 安全修复：禁止通过 update 提交自定义 SQL（与 create 一致）
        if req.data_source_sql.is_some() {
            return Err(AppError::permission_denied(
                "出于安全考虑，自定义 SQL 报表功能已禁用，请使用预定义报表模板".to_string(),
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
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let model = ReportTemplateEntity::find()
            .filter(crate::models::report_template::Column::Id.eq(id))
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
        user_id: i32,
        query: ReportTemplateQuery,
    ) -> Result<(Vec<ReportTemplateModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);

        let mut select = ReportTemplateEntity::find()
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
            .fetch_page(page.saturating_sub(1))
            .await?;

        Ok((items, total))
    }

    /// 执行自定义报表
    // TODO(tech-debt): _page/_page_size 当前未实际参与分页，待自定义 SQL 报表安全重构后启用
    pub async fn execute_custom_report(
        &self,
        template_id: i32,
        user_id: i32,
        _role_id: Option<i32>,
        _page: u64,
        _page_size: u64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        let _template = self
            .get_by_id(template_id, user_id)
            .await?
            .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

        // P0-B 安全修复：彻底关闭"自定义 SQL 报表"执行入口。
        // 任何带 data_source_sql 的模板统一返回功能禁用错误，
        // 避免攻击者通过创建/更新已存在的模板字段来触发 SQL 执行。
        if _template.data_source_sql.is_some() {
            return Err(AppError::permission_denied(
                "出于安全考虑，自定义 SQL 报表功能已禁用，请使用预定义报表模板".to_string(),
            ));
        }

        // 否则使用预定义的报表类型
        Err(AppError::business(
            "自定义报表需要配置数据源SQL".to_string(),
        ))
    }
}

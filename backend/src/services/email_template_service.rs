//! 邮件模板 Service
//!
//! 提供邮件模板的CRUD操作和持久化功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

use sea_orm::DatabaseConnection;

use crate::models::email_template::{
    ActiveModel, Entity as EmailTemplateEntity, Model as EmailTemplateModel,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 创建邮件模板请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateEmailTemplateRequest {
    pub name: String,
    pub code: String,
    pub subject_template: String,
    pub body_template: String,
    pub template_type: String,
    pub variables: Option<serde_json::Value>,
    pub description: Option<String>,
}

/// 更新邮件模板请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateEmailTemplateRequest {
    pub name: Option<String>,
    pub subject_template: Option<String>,
    pub body_template: Option<String>,
    pub template_type: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// 邮件模板查询参数
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct EmailTemplateQuery {
    pub template_type: Option<String>,
    pub is_active: Option<bool>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 邮件模板 Service
pub struct EmailTemplateService {
    db: Arc<DatabaseConnection>,
}

impl EmailTemplateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建邮件模板
    pub async fn create(
        &self,
        user_id: i32,
        req: CreateEmailTemplateRequest,
    ) -> Result<EmailTemplateModel, AppError> {
        // 检查编码是否已存在
        let existing = EmailTemplateEntity::find()
            .filter(crate::models::email_template::Column::Code.eq(&req.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!(
                "邮件模板编码 {} 已存在",
                req.code
            )));
        }

        let now = Utc::now();
        let active_model = ActiveModel {
            id: Default::default(),
            name: Set(req.name),
            code: Set(req.code),
            subject_template: Set(req.subject_template),
            body_template: Set(req.body_template),
            template_type: Set(req.template_type),
            variables: Set(req.variables),
            description: Set(req.description),
            is_active: Set(true),
            status: Set("ACTIVE".to_string()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 获取邮件模板详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<EmailTemplateModel>, AppError> {
        let model = EmailTemplateEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 更新邮件模板
    pub async fn update(
        &self,
        id: i32,
        req: UpdateEmailTemplateRequest,
    ) -> Result<EmailTemplateModel, AppError> {
        let model = EmailTemplateEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("邮件模板不存在"))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(name) = req.name {
            active_model.name = Set(name);
        }
        if let Some(subject_template) = req.subject_template {
            active_model.subject_template = Set(subject_template);
        }
        if let Some(body_template) = req.body_template {
            active_model.body_template = Set(body_template);
        }
        if let Some(template_type) = req.template_type {
            active_model.template_type = Set(template_type);
        }
        if let Some(variables) = req.variables {
            active_model.variables = Set(Some(variables));
        }
        if let Some(description) = req.description {
            active_model.description = Set(Some(description));
        }
        if let Some(is_active) = req.is_active {
            active_model.is_active = Set(is_active);
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除邮件模板（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = EmailTemplateEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("邮件模板不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.is_active = Set(false);
        active_model.status = Set("INACTIVE".to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 查询邮件模板列表
    pub async fn list(
        &self,
        query: EmailTemplateQuery,
    ) -> Result<(Vec<EmailTemplateModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        let mut select = EmailTemplateEntity::find()
            .filter(crate::models::email_template::Column::Status.eq("ACTIVE"));

        if let Some(template_type) = query.template_type {
            select = select
                .filter(crate::models::email_template::Column::TemplateType.eq(template_type));
        }

        if let Some(is_active) = query.is_active {
            select = select.filter(crate::models::email_template::Column::IsActive.eq(is_active));
        }

        if let Some(keyword) = query.keyword {
            select = select.filter(
                crate::models::email_template::Column::Name
                    .contains(&keyword)
                    .or(crate::models::email_template::Column::Code.contains(&keyword)),
            );
        }

        // 批次 256 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = select
            .order_by_desc(crate::models::email_template::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }
}

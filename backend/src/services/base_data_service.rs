//! 基础数据配置服务

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseDataConfig {
    pub id: i32,
    pub category: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub extra_data: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBaseDataRequest {
    pub category: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub extra_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBaseDataRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
    pub extra_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success_count: u32,
    pub failed_count: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BaseDataError {
    #[error("数据库错误：{0}")]
    DatabaseError(String),
    #[error("数据已存在：{0}")]
    DuplicateError(String),
    #[error("数据不存在")]
    NotFound,
    #[error("导入错误：{0}")]
    ImportError(String),
}

pub struct BaseDataService {
    db: Arc<DatabaseConnection>,
}

impl BaseDataService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_by_category(&self, category: &str) -> Result<Vec<BaseDataConfig>, BaseDataError> {
        let items = crate::models::base_data_config::Entity::find()
            .filter(crate::models::base_data_config::Column::Category.eq(category))
            .order_by_asc(crate::models::base_data_config::Column::SortOrder)
            .all(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?;

        Ok(items.into_iter().map(|m| BaseDataConfig {
            id: m.id,
            category: m.category,
            code: m.code,
            name: m.name,
            description: m.description,
            sort_order: m.sort_order,
            is_active: m.is_active,
            extra_data: m.extra_data,
            created_at: m.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: m.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }).collect())
    }

    pub async fn get_by_id(&self, id: i32) -> Result<BaseDataConfig, BaseDataError> {
        let item = crate::models::base_data_config::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?
            .ok_or(BaseDataError::NotFound)?;

        Ok(BaseDataConfig {
            id: item.id,
            category: item.category,
            code: item.code,
            name: item.name,
            description: item.description,
            sort_order: item.sort_order,
            is_active: item.is_active,
            extra_data: item.extra_data,
            created_at: item.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: item.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub async fn create(&self, req: CreateBaseDataRequest) -> Result<BaseDataConfig, BaseDataError> {
        let existing = crate::models::base_data_config::Entity::find()
            .filter(crate::models::base_data_config::Column::Category.eq(&req.category))
            .filter(crate::models::base_data_config::Column::Code.eq(&req.code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?;

        if existing.is_some() {
            return Err(BaseDataError::DuplicateError(format!(
                "分类 {} 下已存在编码 {}",
                req.category, req.code
            )));
        }

        let now = Utc::now();
        let model = crate::models::base_data_config::ActiveModel {
            id: Set(0),
            category: Set(req.category),
            code: Set(req.code),
            name: Set(req.name),
            description: Set(req.description),
            sort_order: Set(req.sort_order.unwrap_or(0)),
            is_active: Set(true),
            extra_data: Set(req.extra_data),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let item = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?;

        Ok(BaseDataConfig {
            id: item.id,
            category: item.category,
            code: item.code,
            name: item.name,
            description: item.description,
            sort_order: item.sort_order,
            is_active: item.is_active,
            extra_data: item.extra_data,
            created_at: item.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: item.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub async fn update(&self, id: i32, req: UpdateBaseDataRequest) -> Result<BaseDataConfig, BaseDataError> {
        let item = crate::models::base_data_config::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?
            .ok_or(BaseDataError::NotFound)?;

        let mut model: crate::models::base_data_config::ActiveModel = item.into();

        if let Some(name) = req.name {
            model.name = Set(name);
        }
        if let Some(description) = req.description {
            model.description = Set(Some(description));
        }
        if let Some(sort_order) = req.sort_order {
            model.sort_order = Set(sort_order);
        }
        if let Some(is_active) = req.is_active {
            model.is_active = Set(is_active);
        }
        if let Some(extra_data) = req.extra_data {
            model.extra_data = Set(Some(extra_data));
        }
        model.updated_at = Set(Utc::now());

        let updated = model
            .update(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?;

        Ok(BaseDataConfig {
            id: updated.id,
            category: updated.category,
            code: updated.code,
            name: updated.name,
            description: updated.description,
            sort_order: updated.sort_order,
            is_active: updated.is_active,
            extra_data: updated.extra_data,
            created_at: updated.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: updated.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub async fn delete(&self, id: i32) -> Result<(), BaseDataError> {
        let item = crate::models::base_data_config::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?
            .ok_or(BaseDataError::NotFound)?;

        let mut model: crate::models::base_data_config::ActiveModel = item.into();
        model.is_active = Set(false);
        model.updated_at = Set(Utc::now());

        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| BaseDataError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn batch_import(&self, category: &str, items: Vec<CreateBaseDataRequest>) -> Result<ImportResult, BaseDataError> {
        let mut success_count = 0u32;
        let mut failed_count = 0u32;
        let mut errors = Vec::new();

        for (index, item) in items.into_iter().enumerate() {
            let mut req = item;
            req.category = category.to_string();

            match self.create(req).await {
                Ok(_) => success_count += 1,
                Err(e) => {
                    failed_count += 1;
                    errors.push(format!("第 {} 行: {}", index + 1, e));
                }
            }
        }

        Ok(ImportResult {
            success_count,
            failed_count,
            errors,
        })
    }

    pub async fn export(&self, category: &str) -> Result<Vec<BaseDataConfig>, BaseDataError> {
        self.list_by_category(category).await
    }
}

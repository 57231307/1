//! BPM 流程定义管理服务（真实实现）
//!
//! 拆分自 bpm_service.rs：原 10 个流程定义/版本/模板管理方法。
//!
//! 批次 67（P1 1-2 修复）：将原占位实现（全部返回 `Err("Not implemented")`）
//! 替换为真实 CRUD 逻辑，参考 `bpm_service.rs` 中已有 instance/task 模式。
//!
//! 设计说明：
//! - 流程定义 CRUD：直接操作 `bpm_process_definition` 表
//! - 版本管理：基于同一 `code` 的多记录实现，`activate_process_version`
//!   会将同 code 的其他记录置为 INACTIVE，当前记录置为 ACTIVE
//! - 模板功能：通过 `category` 字段特殊值 `__TEMPLATE__` 标识模板记录，
//!   `list_process_definitions` 过滤模板，`list_templates` 只查模板

use crate::models::bpm_process_definition;
use crate::models::dto::bpm_dto::{
    CreateProcessDefinitionRequest, ProcessDefinitionQuery, TemplateQuery,
    UpdateProcessDefinitionRequest,
};
use crate::models::dto::PageResponse;
use crate::utils::error::AppError;

use super::bpm_service::BpmService;
use sea_orm::*;

/// 模板标识：`category` 字段使用此值标记模板记录
const TEMPLATE_CATEGORY: &str = "__TEMPLATE__";

impl BpmService {
    /// 创建流程定义
    ///
    /// 插入一条新的流程定义记录，状态默认为 DRAFT（除非请求显式指定）
    pub async fn create_process_definition(
        &self,
        req: CreateProcessDefinitionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        // 校验 code 唯一性（非模板记录中 code 不可重复）
        let existing = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(&req.code))
            .filter(
                Condition::any()
                    .add(bpm_process_definition::Column::Category.is_null())
                    .add(bpm_process_definition::Column::Category.ne(TEMPLATE_CATEGORY)),
            )
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::validation(format!(
                "流程编码已存在: {}",
                req.code
            )));
        }

        let now = chrono::Utc::now();
        let status = req.status.unwrap_or_else(|| "DRAFT".to_string());
        let active_model = bpm_process_definition::ActiveModel {
            name: Set(req.name),
            code: Set(req.code),
            description: Set(req.description),
            category: Set(req.category),
            version: Set(req.version.or_else(|| Some("v1".to_string()))),
            config: Set(req.config),
            status: Set(status),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let model = active_model.insert(&*self.db).await?;
        Ok(model)
    }

    /// 获取单个流程定义
    ///
    /// 按 id 查询流程定义，返回 Option
    pub async fn get_process_definition(
        &self,
        id: i32,
    ) -> Result<Option<bpm_process_definition::Model>, AppError> {
        let model = bpm_process_definition::Entity::find_by_id(id)
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 更新流程定义
    ///
    /// 部分更新：仅更新请求中提供的字段
    pub async fn update_process_definition(
        &self,
        id: i32,
        req: UpdateProcessDefinitionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        let existing = bpm_process_definition::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流程定义不存在: {}", id)))?;

        let mut active: bpm_process_definition::ActiveModel = existing.into();
        if let Some(name) = req.name {
            active.name = Set(name);
        }
        if let Some(description) = req.description {
            active.description = Set(Some(description));
        }
        if let Some(category) = req.category {
            active.category = Set(Some(category));
        }
        if let Some(config) = req.config {
            active.config = Set(Some(config));
        }
        if let Some(status) = req.status {
            active.status = Set(status);
        }
        active.updated_at = Set(chrono::Utc::now());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除流程定义
    ///
    /// 按 id 删除流程定义记录
    pub async fn delete_process_definition(&self, id: i32) -> Result<(), AppError> {
        let existing = bpm_process_definition::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流程定义不存在: {}", id)))?;

        let active: bpm_process_definition::ActiveModel = existing.into();
        active.delete(&*self.db).await?;
        Ok(())
    }

    /// 获取流程定义列表（分页）
    ///
    /// 过滤模板记录（category != __TEMPLATE__ 或 category IS NULL），支持 category 和 status 筛选
    pub async fn list_process_definitions(
        &self,
        query: ProcessDefinitionQuery,
    ) -> Result<PageResponse<bpm_process_definition::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        // 过滤模板记录：category 为 NULL 或 category != __TEMPLATE__
        // SQL 中 NULL != 'x' 结果为 NULL（非 true），需显式包含 IS NULL
        let mut stmt = bpm_process_definition::Entity::find().filter(
            Condition::any()
                .add(bpm_process_definition::Column::Category.is_null())
                .add(bpm_process_definition::Column::Category.ne(TEMPLATE_CATEGORY)),
        );

        if let Some(category) = query.category {
            stmt = stmt.filter(bpm_process_definition::Column::Category.eq(category));
        }
        if let Some(status) = query.status {
            stmt = stmt.filter(bpm_process_definition::Column::Status.eq(status));
        }

        stmt = stmt.order_by_desc(bpm_process_definition::Column::CreatedAt);

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Ok(PageResponse {
            data: items,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// 创建新版本
    ///
    /// 批次 67 说明：原 `create_process_version` 方法签名只接收 `CreateVersionRequest`
    ///（无 definition_id），无法实现真实的版本复制逻辑。现 handler 层 `create_version`
    /// 直接调用 `create_process_definition` 完成版本创建，此 service 方法已废弃并删除。
    /// 保留此注释说明设计决策，避免后续误添加。

    /// 获取流程定义的所有版本
    ///
    /// 按 definition_id 查询其 code，再查询同 code 的所有记录
    pub async fn list_process_versions(
        &self,
        definition_id: i32,
    ) -> Result<Vec<bpm_process_definition::Model>, AppError> {
        let definition = bpm_process_definition::Entity::find_by_id(definition_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流程定义不存在: {}", definition_id)))?;

        let versions = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(definition.code))
            .filter(
                Condition::any()
                    .add(bpm_process_definition::Column::Category.is_null())
                    .add(bpm_process_definition::Column::Category.ne(TEMPLATE_CATEGORY)),
            )
            .order_by_desc(bpm_process_definition::Column::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(versions)
    }

    /// 激活指定版本
    ///
    /// 将同 code 的其他记录置为 INACTIVE，当前记录置为 ACTIVE
    pub async fn activate_process_version(
        &self,
        id: i32,
    ) -> Result<bpm_process_definition::Model, AppError> {
        let txn = self.db.begin().await?;

        let target = bpm_process_definition::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流程定义不存在: {}", id)))?;

        let code = target.code.clone();

        // 将同 code 的所有非模板记录置为 INACTIVE
        let siblings = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(&code))
            .filter(
                Condition::any()
                    .add(bpm_process_definition::Column::Category.is_null())
                    .add(bpm_process_definition::Column::Category.ne(TEMPLATE_CATEGORY)),
            )
            .filter(bpm_process_definition::Column::Status.eq("ACTIVE"))
            .all(&txn)
            .await?;

        for sibling in siblings {
            if sibling.id != id {
                let mut active: bpm_process_definition::ActiveModel = sibling.into();
                active.status = Set("INACTIVE".to_string());
                active.updated_at = Set(chrono::Utc::now());
                active.update(&txn).await?;
            }
        }

        // 激活目标记录
        let mut target_active: bpm_process_definition::ActiveModel = target.into();
        target_active.status = Set("ACTIVE".to_string());
        target_active.updated_at = Set(chrono::Utc::now());
        let updated = target_active.update(&txn).await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 保存为模板
    ///
    /// 复制 definition 为新记录，category 设为 __TEMPLATE__，name 加 " [模板]" 后缀
    pub async fn save_as_template(&self, id: i32, name: String) -> Result<(), AppError> {
        let source = bpm_process_definition::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流程定义不存在: {}", id)))?;

        let now = chrono::Utc::now();
        // 模板 code 加 __TEMPLATE__ 前缀避免与正常定义冲突
        let template_code = format!("{}-{}-{}", TEMPLATE_CATEGORY, source.code, now.timestamp());

        let active_model = bpm_process_definition::ActiveModel {
            name: Set(name),
            code: Set(template_code),
            description: Set(source.description),
            category: Set(Some(TEMPLATE_CATEGORY.to_string())),
            version: Set(source.version),
            config: Set(source.config),
            status: Set("ACTIVE".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        active_model.insert(&*self.db).await?;
        Ok(())
    }

    /// 获取模板列表（分页）
    ///
    /// 查询 category = __TEMPLATE__ 的记录，支持 category 二次筛选（暂未使用）
    pub async fn list_templates(
        &self,
        _query: TemplateQuery,
    ) -> Result<PageResponse<bpm_process_definition::Model>, AppError> {
        let page = _query.page.unwrap_or(1);
        let page_size = _query.page_size.unwrap_or(10).clamp(1, 100);

        let stmt = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Category.eq(TEMPLATE_CATEGORY))
            .order_by_desc(bpm_process_definition::Column::CreatedAt);

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Ok(PageResponse {
            data: items,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// 从模板创建流程定义
    ///
    /// 复制模板为新 definition（category 设为默认值 "general"），状态为 DRAFT
    pub async fn create_from_template(
        &self,
        template_id: i32,
    ) -> Result<bpm_process_definition::Model, AppError> {
        let template = bpm_process_definition::Entity::find_by_id(template_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("模板不存在: {}", template_id)))?;

        // 校验：源记录必须是模板
        if template.category.as_deref() != Some(TEMPLATE_CATEGORY) {
            return Err(AppError::validation("指定的记录不是模板"));
        }

        let now = chrono::Utc::now();
        // 新 code 加时间戳后缀避免重复
        let new_code = format!("{}-{}", template.code, now.timestamp());

        let active_model = bpm_process_definition::ActiveModel {
            name: Set(format!("{}-副本", template.name)),
            code: Set(new_code),
            description: Set(template.description),
            category: Set(Some("general".to_string())),
            version: Set(Some("v1".to_string())),
            config: Set(template.config),
            status: Set("DRAFT".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let model = active_model.insert(&*self.db).await?;
        Ok(model)
    }
}

use crate::models::quality_standard;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;
use tracing::info;

/// 质量标准查询参数
#[derive(Debug, Clone, Default)]
pub struct QualityStandardQueryParams {
    pub standard_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建质量标准请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateQualityStandardRequest {
    pub standard_code: String,
    pub standard_name: String,
    pub standard_type: String,
    pub version: String,
    pub content: String,
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub remark: Option<String>,
}

/// 更新质量标准请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UpdateQualityStandardRequest {
    pub standard_name: Option<String>,
    pub standard_type: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建版本历史请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateVersionHistoryRequest {
    pub standard_id: i32,
    pub version: String,
    pub change_reason: String,
    pub change_content: String,
}

pub struct QualityStandardService {
    db: Arc<DatabaseConnection>,
}

impl QualityStandardService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取质量标准列表
    pub async fn get_standards_list(
        &self,
        params: QualityStandardQueryParams,
    ) -> Result<(Vec<quality_standard::Model>, u64), AppError> {
        let mut query = quality_standard::Entity::find();

        if let Some(standard_type) = &params.standard_type {
            query = query.filter(quality_standard::Column::StandardType.eq(standard_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(quality_standard::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let standards = query
            .order_by(quality_standard::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((standards, total))
    }

    /// 获取质量标准详情
    pub async fn get_standard_by_id(&self, id: i32) -> Result<quality_standard::Model, AppError> {
        let standard = quality_standard::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("质量标准不存在：{}", id)))?;
        Ok(standard)
    }

    /// 创建质量标准
    pub async fn create_standard(
        &self,
        req: CreateQualityStandardRequest,
        user_id: i32,
    ) -> Result<quality_standard::Model, AppError> {
        info!("用户 {} 正在创建质量标准：{}", user_id, req.standard_code);

        let active_standard = quality_standard::ActiveModel {
            standard_code: Set(req.standard_code),
            standard_name: Set(req.standard_name),
            standard_type: Set(req.standard_type),
            version: Set(req.version),
            content: Set(req.content),
            status: Set("draft".to_string()),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            ..Default::default()
        };

        let standard = active_standard.insert(&*self.db).await?;
        info!("质量标准创建成功：{}", standard.standard_code);
        Ok(standard)
    }

    /// 更新质量标准
    pub async fn update_standard(
        &self,
        id: i32,
        req: UpdateQualityStandardRequest,
        user_id: i32,
    ) -> Result<quality_standard::Model, AppError> {
        info!("用户 {} 正在更新质量标准：{}", user_id, id);

        let mut standard: quality_standard::ActiveModel = self.get_standard_by_id(id).await?.into();

        if let Some(standard_name) = req.standard_name {
            standard.standard_name = Set(standard_name);
        }
        if let Some(standard_type) = req.standard_type {
            standard.standard_type = Set(standard_type);
        }
        if let Some(content) = req.content {
            standard.content = Set(content);
        }
        if let Some(status) = req.status {
            standard.status = Set(status);
        }

        standard.save(&*self.db).await?;
        let updated = self.get_standard_by_id(id).await?;
        info!("质量标准更新成功：{}", id);
        Ok(updated)
    }

    /// 删除质量标准
    pub async fn delete_standard(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除质量标准：{}", user_id, id);

        let _standard = self.get_standard_by_id(id).await?;

        // 检查是否有引用
        // TODO: ParentId 字段不存在，需要检查实际引用情况
        let referenced_count = 0;
        // quality_standard::Entity::find()
        //     .filter(quality_standard::Column::ParentId.eq(Some(id)))
        //     .count(&*self.db)
        //     .await?;

        if referenced_count > 0 {
            return Err(AppError::ValidationError(
                "质量标准被引用，无法删除".to_string(),
            ));
        }

        quality_standard::Entity::delete_many()
            .filter(quality_standard::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;

        info!("质量标准删除成功：{}", id);
        Ok(())
    }

    /// 获取版本历史列表
    pub async fn get_version_history(
        &self,
        standard_id: i32,
    ) -> Result<Vec<quality_standard::Model>, AppError> {
        info!("查询质量标准版本历史：{}", standard_id);

        let versions = quality_standard::Entity::find()
            .filter(quality_standard::Column::Id.eq(standard_id))
            .order_by(quality_standard::Column::Version, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(versions)
    }

    /// 创建版本历史（版本升级）
    #[allow(dead_code)]
    pub async fn create_version_history(
        &self,
        req: CreateVersionHistoryRequest,
        user_id: i32,
    ) -> Result<quality_standard::Model, AppError> {
        info!(
            "用户 {} 正在创建质量标准版本历史：{}",
            user_id, req.standard_id
        );

        let old_standard = self.get_standard_by_id(req.standard_id).await?;

        // 创建新版本
        let active_standard = quality_standard::ActiveModel {
            standard_code: Set(old_standard.standard_code),
            standard_name: Set(old_standard.standard_name),
            standard_type: Set(old_standard.standard_type),
            product_id: Set(old_standard.product_id),
            product_category_id: Set(old_standard.product_category_id),
            version: Set(req.version),
            previous_version_id: Set(Some(req.standard_id)),
            content: Set(req.change_content),
            technical_requirements: Set(old_standard.technical_requirements),
            testing_methods: Set(old_standard.testing_methods),
            acceptance_criteria: Set(old_standard.acceptance_criteria),
            status: Set("draft".to_string()),
            effective_date: Set(chrono::Local::now().date_naive()),
            expiry_date: Set(None),
            ..Default::default()
        };

        let new_standard = active_standard.insert(&*self.db).await?;
        info!("质量标准版本历史创建成功：{}", new_standard.standard_code);
        Ok(new_standard)
    }

    /// 质量标准审批
    pub async fn approve_standard(
        &self,
        standard_id: i32,
        user_id: i32,
        _approval_comment: Option<String>,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在审批质量标准：{}", user_id, standard_id);

        let standard = self.get_standard_by_id(standard_id).await?;

        if standard.status != "draft" && standard.status != "rejected" {
            return Err(AppError::ValidationError(
                "质量标准状态不允许审批".to_string(),
            ));
        }

        let mut standard_active: quality_standard::ActiveModel = standard.into();
        standard_active.status = Set("approved".to_string());
        standard_active.save(&*self.db).await?;

        info!("质量标准审批通过：{}", standard_id);
        Ok(())
    }

    /// 质量标准发布
    #[allow(dead_code)]
    pub async fn publish_standard(&self, standard_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在发布质量标准：{}", user_id, standard_id);

        let standard = self.get_standard_by_id(standard_id).await?;

        if standard.status != "approved" {
            return Err(AppError::ValidationError(
                "质量标准未审批，无法发布".to_string(),
            ));
        }

        let mut standard_active: quality_standard::ActiveModel = standard.into();
        standard_active.status = Set("active".to_string());
        standard_active.save(&*self.db).await?;

        info!("质量标准发布成功：{}", standard_id);
        Ok(())
    }
}

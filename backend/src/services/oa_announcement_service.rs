//! OA 公告 Service（P0-D17 / Batch 488）
//!
//! 提供公告 CRUD + 状态转换（发布/归档）能力。
//! 与权限码 `oa-announcements` 绑定（init_service.rs 已注册）。
//!
//! 参考模板：`report_subscription_service.rs`（同样走 sea_orm + paginate_with_total）。

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

use crate::models::oa_announcement::{
    ActiveModel, Entity as OaAnnouncementEntity, Model as OaAnnouncementModel,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 创建公告请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateOaAnnouncementRequest {
    pub title: String,
    pub content: String,
    /// NOTICE=通知，ANNOUNCEMENT=公告，NEWS=新闻
    pub announcement_type: String,
    /// 发布日期（YYYY-MM-DD）
    pub publish_date: chrono::NaiveDate,
    /// 生效日期（YYYY-MM-DD）
    pub effective_date: chrono::NaiveDate,
    /// 失效日期（可选）
    pub expiry_date: Option<chrono::NaiveDate>,
    /// 是否置顶
    #[serde(default)]
    pub is_top: bool,
    /// 附件（JSON 数组）
    pub attachments: Option<serde_json::Value>,
    /// 备注
    pub remarks: Option<String>,
}

/// 更新公告请求（草稿可全字段更新，已发布状态仅允许更新备注与失效日期）
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateOaAnnouncementRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub announcement_type: Option<String>,
    pub publish_date: Option<chrono::NaiveDate>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub is_top: Option<bool>,
    pub attachments: Option<serde_json::Value>,
    pub remarks: Option<String>,
}

/// 公告查询参数
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OaAnnouncementQuery {
    /// 状态过滤：DRAFT/PUBLISHED/ARCHIVED（留空表示全部）
    pub status: Option<String>,
    /// 类型过滤：NOTICE/ANNOUNCEMENT/NEWS
    pub announcement_type: Option<String>,
    /// 是否仅看置顶
    pub is_top: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// OA 公告 Service
pub struct OaAnnouncementService {
    db: Arc<DatabaseConnection>,
}

impl OaAnnouncementService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 校验公告类型枚举
    fn validate_announcement_type(t: &str) -> Result<(), AppError> {
        match t {
            "NOTICE" | "ANNOUNCEMENT" | "NEWS" => Ok(()),
            _ => Err(AppError::validation(format!(
                "无效的公告类型: {}（应为 NOTICE/ANNOUNCEMENT/NEWS）",
                t
            ))),
        }
    }

    /// 校验状态枚举
    fn validate_status(s: &str) -> Result<(), AppError> {
        match s {
            "DRAFT" | "PUBLISHED" | "ARCHIVED" => Ok(()),
            _ => Err(AppError::validation(format!(
                "无效的公告状态: {}（应为 DRAFT/PUBLISHED/ARCHIVED）",
                s
            ))),
        }
    }

    /// 创建公告（默认为 DRAFT 状态）
    pub async fn create(
        &self,
        user_id: i32,
        req: CreateOaAnnouncementRequest,
    ) -> Result<OaAnnouncementModel, AppError> {
        Self::validate_announcement_type(&req.announcement_type)?;

        if req.effective_date < req.publish_date {
            return Err(AppError::validation("生效日期不能早于发布日期"));
        }
        if let Some(expiry) = req.expiry_date {
            if expiry < req.effective_date {
                return Err(AppError::validation("失效日期不能早于生效日期"));
            }
        }

        let now = Utc::now();
        let active_model = ActiveModel {
            id: Default::default(),
            title: Set(req.title),
            content: Set(req.content),
            announcement_type: Set(req.announcement_type),
            publish_date: Set(req.publish_date),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            publisher_id: Set(user_id),
            status: Set("DRAFT".to_string()),
            is_top: Set(req.is_top),
            attachments: Set(req.attachments),
            remarks: Set(req.remarks),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;
        Ok(model)
    }

    /// 获取公告详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<OaAnnouncementModel>, AppError> {
        let model = OaAnnouncementEntity::find_by_id(id).one(&*self.db).await?;
        Ok(model)
    }

    /// 更新公告
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOaAnnouncementRequest,
    ) -> Result<OaAnnouncementModel, AppError> {
        let model = OaAnnouncementEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("公告不存在"))?;

        // 已发布或已归档状态：限制可更新字段
        let is_restricted = model.status == "PUBLISHED" || model.status == "ARCHIVED";
        if is_restricted {
            // 仅允许更新 expiry_date / remarks / is_top
            let mut active_model: ActiveModel = model.into();
            if let Some(expiry_date) = req.expiry_date {
                active_model.expiry_date = Set(Some(expiry_date));
            }
            if let Some(remarks) = req.remarks {
                active_model.remarks = Set(Some(remarks));
            }
            if let Some(is_top) = req.is_top {
                active_model.is_top = Set(is_top);
            }
            active_model.updated_at = Set(Utc::now());
            return active_model.update(&*self.db).await.map_err(Into::into);
        }

        // DRAFT 状态：全字段更新
        let mut active_model: ActiveModel = model.into();
        if let Some(title) = req.title {
            active_model.title = Set(title);
        }
        if let Some(content) = req.content {
            active_model.content = Set(content);
        }
        if let Some(announcement_type) = req.announcement_type {
            Self::validate_announcement_type(&announcement_type)?;
            active_model.announcement_type = Set(announcement_type);
        }
        if let Some(publish_date) = req.publish_date {
            active_model.publish_date = Set(publish_date);
        }
        if let Some(effective_date) = req.effective_date {
            active_model.effective_date = Set(effective_date);
        }
        if let Some(expiry_date) = req.expiry_date {
            active_model.expiry_date = Set(Some(expiry_date));
        }
        if let Some(is_top) = req.is_top {
            active_model.is_top = Set(is_top);
        }
        if let Some(attachments) = req.attachments {
            active_model.attachments = Set(Some(attachments));
        }
        if let Some(remarks) = req.remarks {
            active_model.remarks = Set(Some(remarks));
        }
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除公告（仅 DRAFT 可硬删除，其他状态禁止删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = OaAnnouncementEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("公告不存在"))?;

        if model.status != "DRAFT" {
            return Err(AppError::bad_request(
                "仅草稿状态的公告可删除，已发布或已归档请改用归档/撤回操作",
            ));
        }

        OaAnnouncementEntity::delete_by_id(id)
            .exec(&*self.db)
            .await?;
        Ok(())
    }

    /// 发布公告（DRAFT → PUBLISHED）
    pub async fn publish(&self, id: i32) -> Result<OaAnnouncementModel, AppError> {
        let model = OaAnnouncementEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("公告不存在"))?;

        if model.status != "DRAFT" {
            return Err(AppError::bad_request(format!(
                "仅草稿状态可发布，当前状态: {}",
                model.status
            )));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("PUBLISHED".to_string());
        active_model.updated_at = Set(Utc::now());
        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }

    /// 归档公告（PUBLISHED → ARCHIVED）
    pub async fn archive(&self, id: i32) -> Result<OaAnnouncementModel, AppError> {
        let model = OaAnnouncementEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("公告不存在"))?;

        if model.status != "PUBLISHED" {
            return Err(AppError::bad_request(format!(
                "仅已发布状态可归档，当前状态: {}",
                model.status
            )));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("ARCHIVED".to_string());
        active_model.updated_at = Set(Utc::now());
        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }

    /// 查询公告列表（按发布日期倒序 + 创建时间倒序）
    pub async fn list(
        &self,
        query: OaAnnouncementQuery,
    ) -> Result<(Vec<OaAnnouncementModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = OaAnnouncementEntity::find();

        if let Some(status) = query.status {
            Self::validate_status(&status)?;
            select =
                select.filter(crate::models::oa_announcement::Column::Status.eq(status));
        }

        if let Some(announcement_type) = query.announcement_type {
            Self::validate_announcement_type(&announcement_type)?;
            select = select.filter(
                crate::models::oa_announcement::Column::AnnouncementType.eq(announcement_type),
            );
        }

        if let Some(is_top) = query.is_top {
            select = select
                .filter(crate::models::oa_announcement::Column::IsTop.eq(is_top));
        }

        // 置顶优先，其次发布日期倒序，最后创建时间倒序
        let paginator = select
            .order_by_desc(crate::models::oa_announcement::Column::IsTop)
            .order_by_desc(crate::models::oa_announcement::Column::PublishDate)
            .order_by_desc(crate::models::oa_announcement::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }
}

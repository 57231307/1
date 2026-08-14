//! OA 公告 Service（P0-D17 / Batch 488）
//!
//! 提供公告 CRUD + 状态转换（发布/归档）能力。
//! 与权限码 `oa-announcements` 绑定（init_service.rs 已注册）。
//!
//! 参考模板：`report_subscription_service.rs`（同样走 sea_orm + paginate_with_total）。

use chrono::Utc;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
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
    /// 缺陷 7.2 修复：可见性范围 ALL/DEPT/ROLE/CUSTOM（默认 ALL）
    #[serde(default = "default_visibility_scope")]
    pub visibility_scope: String,
    /// 缺陷 7.2 修复：可见性配置 JSON
    /// DEPT: {"department_ids": [1,2,3]}；ROLE: {"role_ids": [1,2,3]}；CUSTOM: {"user_ids": [1,2,3]}
    pub visible_scope_config: Option<serde_json::Value>,
}

/// 缺陷 7.2 修复：visibility_scope 默认值
fn default_visibility_scope() -> String {
    "ALL".to_string()
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
    /// 缺陷 7.2 修复：可见性范围（仅 DRAFT 状态可更新）
    pub visibility_scope: Option<String>,
    /// 缺陷 7.2 修复：可见性配置（仅 DRAFT 状态可更新）
    pub visible_scope_config: Option<serde_json::Value>,
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

    /// 缺陷 7.2 修复：校验可见性范围枚举
    fn validate_visibility_scope(s: &str) -> Result<(), AppError> {
        match s {
            "ALL" | "DEPT" | "ROLE" | "CUSTOM" => Ok(()),
            _ => Err(AppError::validation(format!(
                "无效的可见性范围: {}（应为 ALL/DEPT/ROLE/CUSTOM）",
                s
            ))),
        }
    }

    /// 缺陷 7.2 修复：校验 visible_scope_config 与 visibility_scope 一致性
    fn validate_visibility_config(
        scope: &str,
        config: &Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        if scope == "ALL" {
            return Ok(());
        }
        let cfg = config.as_ref().ok_or_else(|| {
            AppError::validation(format!(
                "visibility_scope={} 必须提供 visible_scope_config JSON",
                scope
            ))
        })?;
        let obj = cfg
            .as_object()
            .ok_or_else(|| AppError::validation("visible_scope_config 必须为 JSON 对象"))?;
        let required_key = match scope {
            "DEPT" => "department_ids",
            "ROLE" => "role_ids",
            "CUSTOM" => "user_ids",
            _ => return Ok(()),
        };
        let arr = obj.get(required_key).ok_or_else(|| {
            AppError::validation(format!(
                "visibility_scope={} 时 visible_scope_config 必须包含 {} 字段",
                scope, required_key
            ))
        })?;
        if !arr.is_array() {
            return Err(AppError::validation(format!(
                "visible_scope_config.{} 必须为 JSON 数组",
                required_key
            )));
        }
        Ok(())
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
        Self::validate_visibility_scope(&req.visibility_scope)?;
        Self::validate_visibility_config(&req.visibility_scope, &req.visible_scope_config)?;

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
            visibility_scope: Set(req.visibility_scope),
            visible_scope_config: Set(req.visible_scope_config),
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
        // 缺陷 7.2 修复：可见性范围更新（仅 DRAFT 状态可更新）
        if let Some(visibility_scope) = req.visibility_scope {
            Self::validate_visibility_scope(&visibility_scope)?;
            let config = req.visible_scope_config.clone();
            Self::validate_visibility_config(&visibility_scope, &config)?;
            active_model.visibility_scope = Set(visibility_scope);
            if let Some(config) = config {
                active_model.visible_scope_config = Set(Some(config));
            }
        } else if req.visible_scope_config.is_some() {
            return Err(AppError::validation(
                "更新 visible_scope_config 必须同时提供 visibility_scope",
            ));
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
            select = select.filter(crate::models::oa_announcement::Column::Status.eq(status));
        }

        if let Some(announcement_type) = query.announcement_type {
            Self::validate_announcement_type(&announcement_type)?;
            select = select.filter(
                crate::models::oa_announcement::Column::AnnouncementType.eq(announcement_type),
            );
        }

        if let Some(is_top) = query.is_top {
            select = select.filter(crate::models::oa_announcement::Column::IsTop.eq(is_top));
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

    /// 缺陷 7.2 修复：按用户上下文过滤可见公告列表
    /// 过滤规则：ALL：所有用户可见；DEPT：仅当用户 department_id 在 visible_scope_config.department_ids 中时可见；ROLE：仅当用户 role_id 在 visible_scope_config.role_ids 中时可见；CUSTOM：仅当用户 user_id 在 visible_scope_config.user_ids 中时可见
    pub async fn list_for_user(
        &self,
        query: OaAnnouncementQuery,
        user_id: i32,
        department_id: Option<i32>,
        role_id: Option<i32>,
    ) -> Result<(Vec<OaAnnouncementModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        // 缺陷 7.2 修复：在内存中按可见性过滤（避免复杂 JSON 查询跨数据库兼容问题）
        // 取出原始查询结果后逐条检查 visibility_scope 是否对当前用户可见
        let mut select = OaAnnouncementEntity::find();

        if let Some(status) = query.status {
            Self::validate_status(&status)?;
            select = select.filter(crate::models::oa_announcement::Column::Status.eq(status));
        }

        if let Some(announcement_type) = query.announcement_type {
            Self::validate_announcement_type(&announcement_type)?;
            select = select.filter(
                crate::models::oa_announcement::Column::AnnouncementType.eq(announcement_type),
            );
        }

        if let Some(is_top) = query.is_top {
            select = select.filter(crate::models::oa_announcement::Column::IsTop.eq(is_top));
        }

        // 一次性拉取所有匹配条件的公告到内存（公告总量通常 < 1万，可接受）
        let all_items = select
            .order_by_desc(crate::models::oa_announcement::Column::IsTop)
            .order_by_desc(crate::models::oa_announcement::Column::PublishDate)
            .order_by_desc(crate::models::oa_announcement::Column::CreatedAt)
            .all(&*self.db)
            .await?;

        // 缺陷 7.2 修复：按 visibility_scope 过滤
        let visible_items: Vec<OaAnnouncementModel> = all_items
            .into_iter()
            .filter(|m| Self::is_visible_to_user(m, user_id, department_id, role_id))
            .collect();

        let total = visible_items.len() as u64;
        let start = ((page.saturating_sub(1)) as usize) * (page_size as usize);
        let end = (start + page_size as usize).min(visible_items.len());

        let items = if start >= visible_items.len() {
            Vec::new()
        } else {
            visible_items[start..end].to_vec()
        };

        Ok((items, total))
    }

    /// 缺陷 7.2 修复：判断公告对当前用户是否可见
    fn is_visible_to_user(
        m: &OaAnnouncementModel,
        user_id: i32,
        department_id: Option<i32>,
        role_id: Option<i32>,
    ) -> bool {
        match m.visibility_scope.as_str() {
            "ALL" => true,
            "DEPT" => {
                let config = match &m.visible_scope_config {
                    Some(c) => c,
                    None => return false,
                };
                let dept_ids = config
                    .get("department_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_i64().map(|i| i as i32))
                            .collect::<Vec<_>>()
                    });
                match (department_id, dept_ids) {
                    (Some(d), Some(ids)) => ids.contains(&d),
                    _ => false,
                }
            }
            "ROLE" => {
                let config = match &m.visible_scope_config {
                    Some(c) => c,
                    None => return false,
                };
                let role_ids = config
                    .get("role_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_i64().map(|i| i as i32))
                            .collect::<Vec<_>>()
                    });
                match (role_id, role_ids) {
                    (Some(r), Some(ids)) => ids.contains(&r),
                    _ => false,
                }
            }
            "CUSTOM" => {
                let config = match &m.visible_scope_config {
                    Some(c) => c,
                    None => return false,
                };
                let user_ids = config
                    .get("user_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_i64().map(|i| i as i32))
                            .collect::<Vec<_>>()
                    });
                match user_ids {
                    Some(ids) => ids.contains(&user_id),
                    None => false,
                }
            }
            _ => true,
        }
    }
}

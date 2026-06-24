//! 色卡 CRUD 服务
//!
//! 提供色卡基础 CRUD 业务：create / list / get_by_id / update / archive
//! 色号相关业务在 color_card_item_service 中实现
//! 借出相关业务在 color_card_borrow_service 中实现
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_card::{self, ActiveModel as ColorCardActive, Entity as ColorCardEntity};
use crate::models::color_card_create_dto::{
    ArchiveColorCardDto, CreateColorCardDto, UpdateColorCardDto,
};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum CrudError {
    #[error("色卡不存在")]
    NotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 色卡 CRUD 服务
pub struct ColorCardCrudService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardCrudService {
    /// 从数据库连接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建色卡
    pub async fn create(
        &self,
        dto: CreateColorCardDto,
        tenant_id: i64,
    ) -> Result<color_card::Model, CrudError> {
        // 1. 业务校验
        Self::validate_card_type(&dto.card_type)?;

        // 2. 开始事务
        let txn = self.db.begin().await?;

        // 3. 插入主表
        let now = Utc::now();
        let active = ColorCardActive {
            id: Default::default(),
            card_no: Set(dto.card_no),
            card_name: Set(dto.card_name),
            card_type: Set(dto.card_type),
            season: Set(dto.season),
            brand: Set(dto.brand),
            total_colors: Set(0),
            status: Set("active".to_string()),
            description: Set(dto.description),
            cover_image_url: Set(dto.cover_image_url),
            tenant_id: Set(tenant_id),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&txn).await?;

        // 4. 提交事务
        txn.commit().await?;
        Ok(result)
    }

    /// 列表查询（分页 + 过滤 + 多租户隔离）
    pub async fn list(
        &self,
        tenant_id: i64,
        page: u64,
        page_size: u64,
        card_type: Option<String>,
        season: Option<String>,
        status: Option<String>,
        keyword: Option<String>,
    ) -> Result<(Vec<color_card::Model>, u64), CrudError> {
        let mut query = ColorCardEntity::find();

        // 强制多租户隔离
        query = query.filter(color_card::Column::TenantId.eq(tenant_id));

        if let Some(t) = card_type {
            query = query.filter(color_card::Column::CardType.eq(t));
        }
        if let Some(s) = season {
            query = query.filter(color_card::Column::Season.eq(s));
        }
        if let Some(s) = status {
            query = query.filter(color_card::Column::Status.eq(s));
        }
        if let Some(k) = keyword {
            let pattern = format!("%{}%", k);
            query = query.filter(color_card::Column::CardName.like(pattern));
        }

        let paginator = query
            .order_by_desc(color_card::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    /// 按 ID 查询（多租户隔离）
    pub async fn get_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<color_card::Model, CrudError> {
        ColorCardEntity::find_by_id(id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(CrudError::NotFound)
    }

    /// 更新色卡（仅 active 状态可更新）
    pub async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        dto: UpdateColorCardDto,
    ) -> Result<color_card::Model, CrudError> {
        let existing = self.get_by_id(id, tenant_id).await?;
        if existing.status != "active" {
            return Err(CrudError::InvalidState);
        }

        let mut active: ColorCardActive = existing.into();
        if let Some(v) = dto.card_name {
            active.card_name = Set(v);
        }
        if let Some(v) = dto.card_type {
            Self::validate_card_type(&v)?;
            active.card_type = Set(v);
        }
        if let Some(v) = dto.season {
            active.season = Set(Some(v));
        }
        if let Some(v) = dto.brand {
            active.brand = Set(Some(v));
        }
        if let Some(v) = dto.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = dto.cover_image_url {
            active.cover_image_url = Set(Some(v));
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 归档色卡（soft delete，状态变为 archived）
    pub async fn archive(
        &self,
        id: i64,
        tenant_id: i64,
        _dto: ArchiveColorCardDto,
    ) -> Result<color_card::Model, CrudError> {
        let existing = self.get_by_id(id, tenant_id).await?;
        if existing.status == "archived" {
            return Err(CrudError::InvalidState);
        }

        let mut active: ColorCardActive = existing.into();
        active.status = Set("archived".to_string());
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 标记色卡为遗失
    #[allow(dead_code)] // TODO(tech-debt): 当前未接入路由，后续如需直接标记色卡遗失可接入 CRUD 路由
    pub async fn mark_lost(&self, id: i64, tenant_id: i64) -> Result<color_card::Model, CrudError> {
        let existing = self.get_by_id(id, tenant_id).await?;
        let mut active: ColorCardActive = existing.into();
        active.status = Set("lost".to_string());
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 校验色卡类型
    fn validate_card_type(card_type: &str) -> Result<(), CrudError> {
        match card_type {
            "PANTONE" | "CNCS" | "CUSTOM" => Ok(()),
            _ => Err(CrudError::Validation(format!(
                "无效的色卡类型: {}（允许: PANTONE / CNCS / CUSTOM）",
                card_type
            ))),
        }
    }
}

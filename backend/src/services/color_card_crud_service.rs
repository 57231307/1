//! 色卡 CRUD 服务
//!
//! 提供色卡基础 CRUD 业务：create / list / get_by_id / update / archive
//! 色号相关业务在 color_card_item_service 中实现
//! 借出相关业务在 color_card_issue_service 中实现（V15 P0-F03 重构：borrow→issue）
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::models::color_card::{self, ActiveModel as ColorCardActive, Entity as ColorCardEntity};
use crate::models::color_card_create_dto::{
    ArchiveColorCardDto, CreateColorCardDto, UpdateColorCardDto,
};
// 批次 211 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
// V15 P2 B05-P2-4：色卡状态机闭环常量（draft/issued/received/used/expired/lost/archived）
// status/mod.rs 通过 pub use wage_energy_chemical_business::* 重导出，直接用 color_card 子模块
use crate::models::status::color_card as card_status;
use crate::utils::sql_escape::safe_like_pattern;

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

#[allow(dead_code)]
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
    pub async fn create(&self, dto: CreateColorCardDto) -> Result<color_card::Model, CrudError> {
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
            status: Set(master_data::ACTIVE.to_string()),
            description: Set(dto.description),
            cover_image_url: Set(dto.cover_image_url),
            // V15 P0-F10：新建色卡默认库存为 0，需后续库存初始化或入库调整
            stock_quantity: Set(0),
            issued_quantity: Set(0),
            // batch-13 P3：色卡能力字段
            dyeing_capability: Set(None),
            printing_capability: Set(None),
            color_fastness_grade: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&txn).await?;

        // 4. 提交事务
        txn.commit().await?;
        Ok(result)
    }

    /// 列表查询（分页 + 过滤）
    pub async fn list(
        &self,
        page: u64,
        page_size: u64,
        card_type: Option<String>,
        season: Option<String>,
        status: Option<String>,
        keyword: Option<String>,
    ) -> Result<(Vec<color_card::Model>, u64), CrudError> {
        let mut query = ColorCardEntity::find();

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
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let pattern = safe_like_pattern(&k);
            query = query.filter(color_card::Column::CardName.like(pattern));
        }

        let paginator = query
            .order_by_desc(color_card::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i64) -> Result<color_card::Model, CrudError> {
        ColorCardEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CrudError::NotFound)
    }

    /// 更新色卡（仅 active 状态可更新）
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更；原实现完全无 txn 无 lock，并发 update 同时通过 active 状态检查后基于过期快照写入，；导致字段覆盖；色卡档案被并发修改无审计追溯。
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateColorCardDto,
        user_id: i32,
    ) -> Result<color_card::Model, CrudError> {
        let txn = self.db.begin().await?;

        // 1. 查询色卡（加 lock_exclusive 串行化并发 update）
        let existing = ColorCardEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(CrudError::NotFound)?;
        if existing.status != master_data::ACTIVE {
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

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await
        .map_err(|e| CrudError::Validation(e.to_string()))?;

        txn.commit().await?;
        Ok(result)
    }

    /// 归档色卡（soft delete，状态变为 archived）
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更；原实现完全无 txn 无 lock，并发 archive 重复归档，状态机失效。
    /// V15 P2 B05-P2-4：接入 validate_color_card_status_transition 校验闭环流转
    pub async fn archive(
        &self,
        id: i64,
        _dto: ArchiveColorCardDto,
        user_id: i32,
    ) -> Result<color_card::Model, CrudError> {
        let txn = self.db.begin().await?;

        // 1. 查询色卡（加 lock_exclusive 串行化并发 archive）
        let existing = ColorCardEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(CrudError::NotFound)?;
        Self::validate_color_card_status_transition(&existing.status, card_status::ARCHIVED)?;

        let mut active: ColorCardActive = existing.into();
        active.status = Set(card_status::ARCHIVED.to_string());
        active.updated_at = Set(Utc::now());

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await
        .map_err(|e| CrudError::Validation(e.to_string()))?;

        txn.commit().await?;
        Ok(result)
    }

    /// 标记色卡为遗失（v11 批次 154b：已接入 POST /:id/mark-lost 路由）（批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更）
    /// V15 P2 B05-P2-4：接入 validate_color_card_status_transition 校验闭环流转
    pub async fn mark_lost(&self, id: i64, user_id: i32) -> Result<color_card::Model, CrudError> {
        let txn = self.db.begin().await?;

        // 1. 查询色卡（加 lock_exclusive 串行化并发 mark_lost）
        let existing = ColorCardEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(CrudError::NotFound)?;
        Self::validate_color_card_status_transition(&existing.status, card_status::LOST)?;

        let mut active: ColorCardActive = existing.into();
        active.status = Set(card_status::LOST.to_string());
        active.updated_at = Set(Utc::now());

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入（mark_lost 暂未接入路由，
        // 但签名已补全 user_id，待路由接入时直接传 auth.user_id）
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await
        .map_err(|e| CrudError::Validation(e.to_string()))?;

        txn.commit().await?;
        Ok(result)
    }

    /// V15 P2 B05-P2-4：发放色卡（draft/active → issued），记录色卡已发放给客户/销售。
    pub async fn issue_card(&self, id: i64, user_id: i32) -> Result<color_card::Model, CrudError> {
        Self::transition_status(self, id, card_status::ISSUED, user_id).await
    }

    /// V15 P2 B05-P2-4：收回色卡（issued → received），客户归还色卡。
    pub async fn receive_card(
        &self,
        id: i64,
        user_id: i32,
    ) -> Result<color_card::Model, CrudError> {
        Self::transition_status(self, id, card_status::RECEIVED, user_id).await
    }

    /// V15 P2 B05-P2-4：标记色卡已使用（received → used），色卡已用于生产/打样。
    pub async fn use_card(&self, id: i64, user_id: i32) -> Result<color_card::Model, CrudError> {
        Self::transition_status(self, id, card_status::USED, user_id).await
    }

    /// V15 P2 B05-P2-4：标记色卡过期（used → expired），色卡超过有效期，终态。
    pub async fn expire_card(&self, id: i64, user_id: i32) -> Result<color_card::Model, CrudError> {
        Self::transition_status(self, id, card_status::EXPIRED, user_id).await
    }

    /// V15 P2 B05-P2-4：通用状态流转方法（加锁查询 → 校验流转 → 更新 + 审计）。
    async fn transition_status(
        &self,
        id: i64,
        target: &str,
        user_id: i32,
    ) -> Result<color_card::Model, CrudError> {
        let txn = self.db.begin().await?;
        let existing = ColorCardEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(CrudError::NotFound)?;
        Self::validate_color_card_status_transition(&existing.status, target)?;

        let mut active: ColorCardActive = existing.into();
        active.status = Set(target.to_string());
        active.updated_at = Set(Utc::now());
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await
        .map_err(|e| CrudError::Validation(e.to_string()))?;
        txn.commit().await?;
        Ok(result)
    }

    /// B05-P2-4：校验色卡状态流转（draft/active→issued→received→used→expired/lost/archived 终态）。
    fn validate_color_card_status_transition(from: &str, to: &str) -> Result<(), CrudError> {
        // 终态不可再流转
        if matches!(from, "expired" | "lost" | "archived") {
            return Err(CrudError::InvalidState);
        }
        let allowed = match to {
            card_status::ISSUED => matches!(from, "draft" | "active"),
            card_status::RECEIVED => from == "issued",
            card_status::USED => from == "received",
            card_status::EXPIRED => from == "used",
            card_status::LOST | card_status::ARCHIVED => true,
            _ => false,
        };
        if !allowed {
            return Err(CrudError::InvalidState);
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// V15 P2 B05-P2-4：合法流转 draft → issued → received → used → expired
    #[test]
    fn test_valid_forward_transition_chain() {
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            "draft",
            card_status::ISSUED
        )
        .is_ok());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            "active",
            card_status::ISSUED
        )
        .is_ok());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::ISSUED,
            card_status::RECEIVED
        )
        .is_ok());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::RECEIVED,
            card_status::USED
        )
        .is_ok());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::USED,
            card_status::EXPIRED
        )
        .is_ok());
    }

    /// V15 P2 B05-P2-4：任意非终态 → lost/archived 合法
    #[test]
    fn test_terminal_transitions_from_any_non_terminal() {
        for from in ["draft", "active", "issued", "received", "used"] {
            assert!(ColorCardCrudService::validate_color_card_status_transition(
                from,
                card_status::LOST
            )
            .is_ok());
            assert!(ColorCardCrudService::validate_color_card_status_transition(
                from,
                card_status::ARCHIVED
            )
            .is_ok());
        }
    }

    /// V15 P2 B05-P2-4：终态不可再流转
    #[test]
    fn test_terminal_state_no_outgoing() {
        for terminal in [
            card_status::EXPIRED,
            card_status::LOST,
            card_status::ARCHIVED,
        ] {
            assert!(ColorCardCrudService::validate_color_card_status_transition(
                terminal,
                card_status::ISSUED
            )
            .is_err());
            assert!(ColorCardCrudService::validate_color_card_status_transition(
                terminal,
                card_status::RECEIVED
            )
            .is_err());
        }
    }

    /// V15 P2 B05-P2-4：非法跳转（如 draft → received / issued → used）
    #[test]
    fn test_invalid_skip_transitions() {
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            "draft",
            card_status::RECEIVED
        )
        .is_err());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::ISSUED,
            card_status::USED
        )
        .is_err());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::RECEIVED,
            card_status::EXPIRED
        )
        .is_err());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            "draft",
            card_status::USED
        )
        .is_err());
    }

    /// V15 P2 B05-P2-4：非法回退（如 issued → draft / used → received）
    #[test]
    fn test_invalid_backward_transitions() {
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::ISSUED,
            "draft"
        )
        .is_err());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::USED,
            card_status::RECEIVED
        )
        .is_err());
        assert!(ColorCardCrudService::validate_color_card_status_transition(
            card_status::RECEIVED,
            card_status::ISSUED
        )
        .is_err());
    }
}

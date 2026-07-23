//! 单据-业务模式关联 Service impl 子模块（business_mode_ops/order_link）
//!
//! 批次 489 D10-2b 拆分：从原 `business_mode_service.rs` L1262-1434 迁移。
//! 包含 BusinessModeOrderLinkService 的 7 个方法：
//! - link_order / create / update / delete（CRUD）
//! - get_by_id / get_by_document / list（查询）
//!
//! 业务规则：
//! - 单据与业务模式为物理删除（解除关联）
//! - 单据同一类型+ID 仅可关联一个业务模式
//! - create 为 link_order 的标准 CRUD 入口（兼容 handler 调用）

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::business_mode_config::{self, Entity as ConfigEntity};
use crate::models::business_mode_order_link::{
    self, ActiveModel as OrderLinkActiveModel, Entity as OrderLinkEntity, Model as OrderLinkModel,
};
use crate::utils::error::AppError;

use crate::services::business_mode_service::{validate_document_type, BusinessModeOrderLinkService};
use crate::services::business_mode_ops::types::{
    BusinessModeOrderLinkQuery, CreateBusinessModeOrderLinkRequest,
    UpdateBusinessModeOrderLinkRequest,
};

impl BusinessModeOrderLinkService {
    /// 关联单据到业务模式
    pub async fn link_order(
        &self,
        mode_id: i32,
        document_type: &str,
        document_id: i32,
        document_no: &str,
        mode_snapshot: Option<serde_json::Value>,
    ) -> Result<OrderLinkModel, AppError> {
        validate_document_type(document_type)?;

        // 校验业务模式存在
        if ConfigEntity::find_by_id(mode_id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "业务模式 {} 不存在",
                mode_id
            )));
        }

        // 校验单据是否已关联其他业务模式
        if let Some(_existing) = OrderLinkEntity::find()
            .filter(business_mode_order_link::Column::DocumentType.eq(document_type))
            .filter(business_mode_order_link::Column::DocumentId.eq(document_id))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "单据 {}:{} 已关联业务模式，请先解除原关联",
                document_type, document_id
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = OrderLinkActiveModel {
            id: Default::default(),
            mode_id: Set(mode_id),
            document_type: Set(document_type.to_string()),
            document_id: Set(document_id),
            document_no: Set(document_no.to_string()),
            mode_snapshot: Set(mode_snapshot),
            remarks: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("单据业务模式关联创建失败: {}", e)))?;
        Ok(result)
    }

    /// 创建单据-业务模式关联（兼容标准 CRUD 入口）
    pub async fn create(
        &self,
        req: CreateBusinessModeOrderLinkRequest,
    ) -> Result<OrderLinkModel, AppError> {
        self.link_order(
            req.mode_id,
            &req.document_type,
            req.document_id,
            &req.document_no,
            req.mode_snapshot,
        )
        .await
    }

    /// 更新单据-业务模式关联
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeOrderLinkRequest,
    ) -> Result<OrderLinkModel, AppError> {
        let model = self.get_by_id(id).await?;

        let mut active: OrderLinkActiveModel = model.into();

        if let Some(v) = req.mode_id {
            // 校验业务模式存在
            if ConfigEntity::find_by_id(v)
                .filter(business_mode_config::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("业务模式 {} 不存在", v)));
            }
            active.mode_id = Set(v);
        }
        if let Some(v) = req.mode_snapshot {
            active.mode_snapshot = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除单据-业务模式关联（物理删除，解除关联）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        OrderLinkEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("单据业务模式关联删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<OrderLinkModel, AppError> {
        OrderLinkEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("单据业务模式关联 {} 不存在", id)))
    }

    /// 按单据查询关联
    pub async fn get_by_document(
        &self,
        document_type: &str,
        document_id: i32,
    ) -> Result<Option<OrderLinkModel>, AppError> {
        validate_document_type(document_type)?;
        let model = OrderLinkEntity::find()
            .filter(business_mode_order_link::Column::DocumentType.eq(document_type))
            .filter(business_mode_order_link::Column::DocumentId.eq(document_id))
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: BusinessModeOrderLinkQuery,
    ) -> Result<(Vec<OrderLinkModel>, u64), AppError> {
        let mut q = OrderLinkEntity::find();
        if let Some(v) = query.mode_id {
            q = q.filter(business_mode_order_link::Column::ModeId.eq(v));
        }
        if let Some(v) = query.document_type {
            q = q.filter(business_mode_order_link::Column::DocumentType.eq(v));
        }
        if let Some(v) = query.document_no {
            q = q.filter(business_mode_order_link::Column::DocumentNo.contains(&v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(business_mode_order_link::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

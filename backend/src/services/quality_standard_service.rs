use crate::models::custom_order;
use crate::models::quality_standard;
use crate::models::status::master_data;
use crate::models::status::quality_dyeing::quality_standard as qs_status;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use chrono::NaiveDate;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::info;

/// 质量标准代码自动编码前缀（存量数据以 "QS-" 开头，见原 create_standard
/// `format!("QS-{ts}-{4}")`；列 UNIQUE 证据：
/// migration/src/domain/system/m0005_add_basic_data_and_system_tables.rs:110
/// `"standard_code" VARCHAR(50) NOT NULL UNIQUE`）
const STANDARD_CODE_PREFIX: &str = "QS-";

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
pub struct CreateQualityStandardRequest {
    pub standard_code: Option<String>,
    pub standard_name: String,
    pub standard_type: Option<String>,
    pub version: Option<String>,
    pub content: Option<String>,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub remark: Option<String>,
}

/// 更新质量标准请求
#[derive(Debug, Clone)]
pub struct UpdateQualityStandardRequest {
    pub standard_name: Option<String>,
    pub standard_type: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建版本历史请求
#[derive(Debug, Clone)]
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
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
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
            .ok_or_else(|| AppError::not_found(format!("质量标准不存在：{}", id)))?;
        Ok(standard)
    }

    /// 创建质量标准
    pub async fn create_standard(
        &self,
        req: CreateQualityStandardRequest,
        user_id: i32,
    ) -> Result<quality_standard::Model, AppError> {
        // 自动生成标准代码收口到生成器（原「秒级时间戳+4位随机」同秒并发撞
        // standard_code UNIQUE 直接 500，见 STANDARD_CODE_PREFIX 注释 DDL 证据）
        let auto_code = req.standard_code.is_none();
        let manual_code = req.standard_code.clone().unwrap_or_default();

        let build_active = |standard_code: String| quality_standard::ActiveModel {
            standard_code: Set(standard_code),
            standard_name: Set(req.standard_name.clone()),
            standard_type: Set(req
                .standard_type
                .clone()
                .unwrap_or_else(|| "general".to_string())),
            version: Set(req.version.clone().unwrap_or_else(|| "1.0".to_string())),
            content: Set(req.content.clone().unwrap_or_default()),
            status: Set(qs_status::DRAFT.to_string()),
            effective_date: Set(req
                .effective_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive())),
            expiry_date: Set(req.expiry_date),
            ..Default::default()
        };

        let txn = (*self.db).begin().await?;
        let standard = if auto_code {
            DocumentNumberGenerator::insert_with_no_retry(
                &txn,
                STANDARD_CODE_PREFIX,
                quality_standard::Entity,
                quality_standard::Column::StandardCode,
                build_active,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "质量标准代码生成失败");
                AppError::business_displayable("质量标准代码生成失败，请稍后重试")
            })?
        } else {
            build_active(manual_code).insert(&txn).await?
        };
        txn.commit().await?;
        info!(
            "用户 {} 创建质量标准成功：{}",
            user_id, standard.standard_code
        );
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
    ///
    /// 引用预检（真实计数，不是占位）：
    /// 1. `custom_orders.quality_standard_id`——客户专属质量标准（写入方
    ///    `services/custom_order_crud_service.rs:368`；列由
    ///    `migration/src/domain/sales_crm/mod.rs:498` ADD COLUMN 添加且**无 FK**，
    ///    所以删除后不会被数据库拦住，只会让存量定制品单的质检标准悬空）；
    /// 2. `quality_standards.previous_version_id`——版本链自引用（写入方本服务
    ///    `create_version_history`，指向被升级的旧版本行；删掉旧行会让新版本失去前驱）。
    /// 任一命中 → `business_displayable` 公开规则文案拒绝（不含表名/列名/约束名）；
    /// 均无引用才执行删除。
    pub async fn delete_standard(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除质量标准：{}", user_id, id);

        let _standard = self.get_standard_by_id(id).await?;

        let referenced_by_custom_orders = custom_order::Entity::find()
            .filter(custom_order::Column::QualityStandardId.eq(id))
            .count(&*self.db)
            .await?;
        let referenced_by_version_chain = quality_standard::Entity::find()
            .filter(quality_standard::Column::PreviousVersionId.eq(id))
            .count(&*self.db)
            .await?;
        info!(
            "质量标准删除预检：id={}, 定制品单引用 {} 条, 版本链后继引用 {} 条",
            id, referenced_by_custom_orders, referenced_by_version_chain
        );

        if referenced_by_custom_orders > 0 || referenced_by_version_chain > 0 {
            return Err(AppError::business_displayable(
                "该质量标准已被定制品单或后续版本引用，无法删除",
            ));
        }

        let result = quality_standard::Entity::delete_many()
            .filter(quality_standard::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!("质量标准不存在：{}", id)));
        }

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
            status: Set(qs_status::DRAFT.to_string()),
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

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let standard = quality_standard::Entity::find_by_id(standard_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量标准不存在：{}", standard_id)))?;

        if standard.status != qs_status::DRAFT && standard.status != qs_status::REJECTED {
            return Err(AppError::validation("质量标准状态不允许审批".to_string()));
        }

        let mut standard_active: quality_standard::ActiveModel = standard.into();
        standard_active.status = Set(qs_status::APPROVED.to_string());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            standard_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("质量标准审批通过：{}", standard_id);
        Ok(())
    }

    /// 质量标准驳回（批次 157d-2 新增）
    pub async fn reject_standard(
        &self,
        standard_id: i32,
        user_id: i32,
        _reject_reason: Option<String>,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在驳回质量标准：{}", user_id, standard_id);

        let txn = (*self.db).begin().await?;

        let standard = quality_standard::Entity::find_by_id(standard_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量标准不存在：{}", standard_id)))?;

        if standard.status != qs_status::DRAFT && standard.status != qs_status::APPROVED {
            return Err(AppError::validation("质量标准状态不允许驳回".to_string()));
        }

        let mut standard_active: quality_standard::ActiveModel = standard.into();
        standard_active.status = Set(qs_status::REJECTED.to_string());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            standard_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("质量标准已驳回：{}", standard_id);
        Ok(())
    }

    /// 质量标准归档（published/active 或 approved 状态可归档，状态置 archived）
    pub async fn archive_standard(&self, standard_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在归档质量标准：{}", user_id, standard_id);

        // 与 approve/publish 一致：lock_exclusive 串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let standard = quality_standard::Entity::find_by_id(standard_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量标准不存在：{}", standard_id)))?;

        if standard.status != qs_status::APPROVED && standard.status != master_data::ACTIVE {
            return Err(AppError::validation("质量标准状态不允许归档".to_string()));
        }

        let mut standard_active: quality_standard::ActiveModel = standard.into();
        standard_active.status = Set(master_data::ARCHIVED.to_string());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            standard_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("质量标准归档成功：{}", standard_id);
        Ok(())
    }

    /// 质量标准发布
    pub async fn publish_standard(&self, standard_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在发布质量标准：{}", user_id, standard_id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let standard = quality_standard::Entity::find_by_id(standard_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量标准不存在：{}", standard_id)))?;

        if standard.status != qs_status::APPROVED {
            return Err(AppError::validation("质量标准未审批，无法发布".to_string()));
        }

        let mut standard_active: quality_standard::ActiveModel = standard.into();
        standard_active.status = Set(master_data::ACTIVE.to_string());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            standard_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("质量标准发布成功：{}", standard_id);
        Ok(())
    }
}

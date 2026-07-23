//! 产品 Service 色号管理子模块（product_ops/color）
//!
//! 批次 D10 拆分：从原 `product_service.rs` 迁移。
//! 包含 `ProductService` 的 5 个产品色号管理方法：
//! - `list_product_colors`：获取产品的色号列表
//! - `create_product_color`：创建产品色号
//! - `batch_create_product_colors`：批量创建（内部循环调用 create_product_color）
//! - `update_product_color`：更新（审计日志）
//! - `delete_product_color`：删除（审计日志）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, Set};

use crate::models::product_color::{self, Entity as ProductColorEntity};
use crate::services::product_service::{
    CreateProductColorInput, ProductService, UpdateProductColorParams,
};
use crate::utils::error::AppError;

impl ProductService {
    /// 获取产品的色号列表
    pub async fn list_product_colors(
        &self,
        product_id: i32,
    ) -> Result<Vec<product_color::Model>, AppError> {
        ProductColorEntity::find()
            .filter(product_color::Column::ProductId.eq(product_id))
            .filter(product_color::Column::IsActive.eq(true))
            .order_by(product_color::Column::ColorNo, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 创建产品色号
    ///
    /// 批次 412 技术债务清理：签名从 7 参数改为 2 参数（product_id + CreateProductColorInput），
    /// 复用已有的 `CreateProductColorInput` 参数对象，消除 `clippy::too_many_arguments` 警告。
    pub async fn create_product_color(
        &self,
        product_id: i32,
        input: CreateProductColorInput,
    ) -> Result<product_color::Model, AppError> {
        let active_model = product_color::ActiveModel {
            id: Default::default(),
            product_id: Set(product_id),
            color_no: Set(input.color_no),
            color_name: Set(input.color_name),
            pantone_code: Set(input.pantone_code),
            color_type: Set(input.color_type),
            dye_formula: Set(input.dye_formula),
            extra_cost: Set(Decimal::from_f64_retain(input.extra_cost).unwrap_or(Decimal::ZERO)),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 批量创建产品色号
    pub async fn batch_create_product_colors(
        &self,
        product_id: i32,
        colors: Vec<CreateProductColorInput>,
    ) -> Result<Vec<product_color::Model>, AppError> {
        let mut results = Vec::new();

        for input in colors {
            let result = self.create_product_color(product_id, input).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 更新产品色号
    ///
    /// 批次 330 v10 复审 P3 修复：使用 UpdateProductColorParams 参数对象替代 8 个独立参数
    pub async fn update_product_color(
        &self,
        params: UpdateProductColorParams,
    ) -> Result<product_color::Model, AppError> {
        let id = params.id;
        let color_name = params.color_name;
        let pantone_code = params.pantone_code;
        let color_type = params.color_type;
        let dye_formula = params.dye_formula;
        let extra_cost = params.extra_cost;
        let is_active = params.is_active;
        let user_id = params.user_id;

        let mut color: product_color::ActiveModel = ProductColorEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品色号 ID {} 不存在", id)))?
            .into();

        if let Some(cn) = color_name {
            color.color_name = Set(cn);
        }
        if let Some(pc) = pantone_code {
            color.pantone_code = Set(Some(pc));
        }
        if let Some(ct) = color_type {
            color.color_type = Set(ct);
        }
        if let Some(df) = dye_formula {
            color.dye_formula = Set(Some(df));
        }
        if let Some(ec) = extra_cost {
            color.extra_cost = Set(Decimal::from_f64_retain(ec).unwrap_or(Decimal::ZERO));
        }
        if let Some(active) = is_active {
            color.is_active = Set(active);
        }

        color.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            color,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;
        Ok(result)
    }

    /// 删除产品色号
    pub async fn delete_product_color(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ProductColorEntity,
            _,
        >(&*self.db, "product_color", id, Some(user_id))
        .await
    }
}

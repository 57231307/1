//! 供应商商品目录服务（supplier_product_service）
//!
//! 提供 supplier_products 表的 list（按 supplier 过滤 + 关键字）、get、create、update。
//! 写法对齐 sku_mapping_service：单次查询、无 N+1、分页用 PaginatedResponse 信封、
//! 引用校验失败走 AppError::validation（本仓映射为 400 / VALIDATION_ERROR），
//! 用户可感知的重复提示走 AppError::business_displayable（真实文案外显）。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, NotSet, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;
use validator::Validate;

use crate::models::{supplier, supplier_product};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

// =====================================================
// DTO
// =====================================================

/// 列表查询参数（supplier_id 为必填父级过滤；keyword 对 product_code/product_name LIKE）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SupplierProductQueryParams {
    pub supplier_id: Option<i32>,
    pub keyword: Option<String>,
    pub is_enabled: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建入参
#[allow(dead_code, reason = "请求 DTO 由 serde/validate 消费部分字段")]
#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct CreateSupplierProductRequest {
    pub supplier_id: i32,
    #[validate(length(min = 1, max = 100, message = "商品编码长度必须在 1 到 100 个字符之间"))]
    pub product_code: String,
    #[validate(length(min = 1, max = 200, message = "商品名称长度必须在 1 到 200 个字符之间"))]
    pub product_name: String,
    pub product_description: Option<String>,
    #[validate(length(min = 1, max = 20, message = "单位长度必须在 1 到 20 个字符之间"))]
    pub unit: String,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

/// 更新入参
#[allow(dead_code, reason = "请求 DTO 由 serde/validate 消费部分字段")]
#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct UpdateSupplierProductRequest {
    pub supplier_id: i32,
    #[validate(length(min = 1, max = 100, message = "商品编码长度必须在 1 到 100 个字符之间"))]
    pub product_code: String,
    #[validate(length(min = 1, max = 200, message = "商品名称长度必须在 1 到 200 个字符之间"))]
    pub product_name: String,
    pub product_description: Option<String>,
    #[validate(length(min = 1, max = 20, message = "单位长度必须在 1 到 20 个字符之间"))]
    pub unit: String,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

pub struct SupplierProductService {
    db: Arc<DatabaseConnection>,
}

impl SupplierProductService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列表（分页）。supplier_id 为必填父级过滤，缺失即 validation。
    pub async fn list(
        &self,
        params: SupplierProductQueryParams,
    ) -> Result<(Vec<supplier_product::Model>, u64, u64, u64), AppError> {
        let page = params.page.unwrap_or(1).clamp(1, 1000);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let supplier_id = params.supplier_id.ok_or_else(|| {
            AppError::validation("supplier_id 不能为空（供应商商品必须按供应商过滤查询）")
        })?;

        let mut query = supplier_product::Entity::find()
            .filter(supplier_product::Column::SupplierId.eq(supplier_id));

        if let Some(enabled) = params.is_enabled {
            query = query.filter(supplier_product::Column::IsEnabled.eq(enabled));
        }
        // 关键字对高基数字段 product_code / product_name 做 LIKE 检索（任一命中）；空串剔除。
        if let Some(kw) = params
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            query = query.filter(
                Condition::any()
                    .add(supplier_product::Column::ProductCode.contains(kw))
                    .add(supplier_product::Column::ProductName.contains(kw)),
            );
        }

        query = query.order_by(supplier_product::Column::ProductCode, Order::Asc);

        let paginator = query.paginate(&*self.db, page_size);
        let (items, total) = paginate_with_total(paginator, page).await?;

        Ok((items, total, page, page_size))
    }

    /// 单条详情
    pub async fn get(&self, id: i32) -> Result<supplier_product::Model, AppError> {
        supplier_product::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商商品 {}", id)))
    }

    /// 创建供应商商品（校验 supplier 存在 + 同供应商下编码唯一）
    pub async fn create(
        &self,
        req: CreateSupplierProductRequest,
        user_id: i32,
    ) -> Result<supplier_product::Model, AppError> {
        self.ensure_supplier_exists(req.supplier_id).await?;
        self.ensure_code_unique(req.supplier_id, &req.product_code, None)
            .await?;

        let now = chrono::Utc::now();
        let active = supplier_product::ActiveModel {
            id: NotSet,
            supplier_id: Set(req.supplier_id),
            product_code: Set(req.product_code),
            product_name: Set(req.product_name),
            product_description: Set(req.product_description),
            unit: Set(req.unit),
            is_enabled: Set(req.is_enabled.unwrap_or(true)),
            created_at: Set(now),
            updated_at: Set(now),
            created_by: Set(Some(user_id)),
            updated_by: Set(Some(user_id)),
            remarks: Set(req.remarks),
        };
        let inserted = active.insert(&*self.db).await?;
        Ok(inserted)
    }

    /// 更新供应商商品（记录存在 + supplier 存在 + 同供应商下编码唯一）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateSupplierProductRequest,
        user_id: i32,
    ) -> Result<supplier_product::Model, AppError> {
        let existing = supplier_product::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商商品 {}", id)))?;

        self.ensure_supplier_exists(req.supplier_id).await?;
        self.ensure_code_unique(req.supplier_id, &req.product_code, Some(id))
            .await?;

        let mut active: supplier_product::ActiveModel = existing.into();
        let cur_enabled = active.is_enabled.unwrap();
        active.supplier_id = Set(req.supplier_id);
        active.product_code = Set(req.product_code);
        active.product_name = Set(req.product_name);
        active.product_description = Set(req.product_description);
        active.unit = Set(req.unit);
        // 未显式传 is_enabled 时保留原值，避免把已停用记录静默重新启用。
        active.is_enabled = Set(req.is_enabled.unwrap_or(cur_enabled));
        active.remarks = Set(req.remarks);
        active.updated_at = Set(chrono::Utc::now());
        active.updated_by = Set(Some(user_id));

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    async fn ensure_supplier_exists(&self, supplier_id: i32) -> Result<(), AppError> {
        // 仅取主键判存在：避免对 supplier::Model 的 NULLABLE 扩展列整行解码在
        // 历史/手工供应商行上抛 500（同 sku_mapping_service::validate_refs）。
        let exists = supplier::Entity::find()
            .filter(supplier::Column::Id.eq(supplier_id))
            .select_only()
            .column(supplier::Column::Id)
            .into_tuple::<i32>()
            .one(&*self.db)
            .await?
            .is_some();
        if !exists {
            return Err(AppError::validation(format!(
                "供应商 ID {} 不存在",
                supplier_id
            )));
        }
        Ok(())
    }

    /// 同一供应商下 product_code 逻辑唯一（表无 DB 唯一约束，故前置查重）。
    /// exclude_id：更新时排除自身。
    async fn ensure_code_unique(
        &self,
        supplier_id: i32,
        product_code: &str,
        exclude_id: Option<i32>,
    ) -> Result<(), AppError> {
        let mut q = supplier_product::Entity::find()
            .filter(supplier_product::Column::SupplierId.eq(supplier_id))
            .filter(supplier_product::Column::ProductCode.eq(product_code));
        if let Some(eid) = exclude_id {
            q = q.filter(supplier_product::Column::Id.ne(eid));
        }
        if q.one(&*self.db).await?.is_some() {
            return Err(AppError::business_displayable(
                "该供应商商品编码已存在，请勿重复创建",
            ));
        }
        Ok(())
    }
}

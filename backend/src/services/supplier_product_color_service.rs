//! 供应商商品色号目录服务（supplier_product_color_service）
//!
//! 提供 supplier_product_colors 表的 list（按 supplier_product 过滤 + 关键字）、get、
//! create、update。一个供应商商品可挂上千色号，故关键字对 color_no/color_name 做 LIKE。
//! extra_cost（Decimal）在请求体中以字符串承载（避免浮点精度丢失），缺省落 0。
//! 写法对齐 sku_mapping_service / supplier_product_service。

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, NotSet, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::str::FromStr;
use std::sync::Arc;
use validator::Validate;

use crate::models::{supplier_product, supplier_product_color};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

// =====================================================
// DTO
// =====================================================

/// 列表查询参数（supplier_product_id 必填父级过滤；keyword 对 color_no/color_name LIKE）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SupplierProductColorQueryParams {
    pub supplier_product_id: Option<i32>,
    pub keyword: Option<String>,
    pub is_enabled: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建入参（extra_cost 为可选字符串金额，缺省 0）
#[allow(dead_code, reason = "请求 DTO 由 serde/validate 消费部分字段")]
#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct CreateSupplierProductColorRequest {
    pub supplier_product_id: i32,
    #[validate(length(min = 1, max = 50, message = "色号编码长度必须在 1 到 50 个字符之间"))]
    pub color_no: String,
    #[validate(length(min = 1, max = 100, message = "色号名称长度必须在 1 到 100 个字符之间"))]
    pub color_name: String,
    #[validate(length(max = 50, message = "PANTONE 编码长度不能超过 50 个字符"))]
    pub pantone_code: Option<String>,
    pub extra_cost: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 更新入参
#[allow(dead_code, reason = "请求 DTO 由 serde/validate 消费部分字段")]
#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct UpdateSupplierProductColorRequest {
    pub supplier_product_id: i32,
    #[validate(length(min = 1, max = 50, message = "色号编码长度必须在 1 到 50 个字符之间"))]
    pub color_no: String,
    #[validate(length(min = 1, max = 100, message = "色号名称长度必须在 1 到 100 个字符之间"))]
    pub color_name: String,
    #[validate(length(max = 50, message = "PANTONE 编码长度不能超过 50 个字符"))]
    pub pantone_code: Option<String>,
    pub extra_cost: Option<String>,
    pub is_enabled: Option<bool>,
}

pub struct SupplierProductColorService {
    db: Arc<DatabaseConnection>,
}

impl SupplierProductColorService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列表（分页）。supplier_product_id 为必填父级过滤，缺失即 validation。
    pub async fn list(
        &self,
        params: SupplierProductColorQueryParams,
    ) -> Result<(Vec<supplier_product_color::Model>, u64, u64, u64), AppError> {
        let page = params.page.unwrap_or(1).clamp(1, 1000);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let supplier_product_id = params.supplier_product_id.ok_or_else(|| {
            AppError::validation("supplier_product_id 不能为空（色号必须按供应商商品过滤查询）")
        })?;

        let mut query = supplier_product_color::Entity::find()
            .filter(supplier_product_color::Column::SupplierProductId.eq(supplier_product_id));

        if let Some(enabled) = params.is_enabled {
            query = query.filter(supplier_product_color::Column::IsEnabled.eq(enabled));
        }
        if let Some(kw) = params
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            query = query.filter(
                Condition::any()
                    .add(supplier_product_color::Column::ColorNo.contains(kw))
                    .add(supplier_product_color::Column::ColorName.contains(kw)),
            );
        }

        query = query.order_by(supplier_product_color::Column::ColorNo, Order::Asc);

        let paginator = query.paginate(&*self.db, page_size);
        let (items, total) = paginate_with_total(paginator, page).await?;

        Ok((items, total, page, page_size))
    }

    /// 单条详情
    pub async fn get(&self, id: i32) -> Result<supplier_product_color::Model, AppError> {
        supplier_product_color::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商色号 {}", id)))
    }

    /// 创建供应商色号（校验 supplier_product 存在 + 同商品下色号唯一）。
    /// 注：supplier_product_colors 表无 created_by/updated_by 列，故不记录操作人。
    pub async fn create(
        &self,
        req: CreateSupplierProductColorRequest,
    ) -> Result<supplier_product_color::Model, AppError> {
        self.ensure_product_exists(req.supplier_product_id).await?;
        self.ensure_color_unique(req.supplier_product_id, &req.color_no, None)
            .await?;
        let extra_cost = parse_extra_cost(req.extra_cost.as_deref())?;

        let now = chrono::Utc::now();
        let active = supplier_product_color::ActiveModel {
            id: NotSet,
            supplier_product_id: Set(req.supplier_product_id),
            color_no: Set(req.color_no),
            color_name: Set(req.color_name),
            pantone_code: Set(req.pantone_code),
            extra_cost: Set(extra_cost),
            is_enabled: Set(req.is_enabled.unwrap_or(true)),
            created_at: Set(now),
            updated_at: Set(now),
            remarks: Set(None),
        };
        let inserted = active.insert(&*self.db).await?;
        Ok(inserted)
    }

    /// 更新供应商色号（记录存在 + product 存在 + 同商品下色号唯一）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateSupplierProductColorRequest,
    ) -> Result<supplier_product_color::Model, AppError> {
        let existing = supplier_product_color::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商色号 {}", id)))?;

        self.ensure_product_exists(req.supplier_product_id).await?;
        self.ensure_color_unique(req.supplier_product_id, &req.color_no, Some(id))
            .await?;
        let extra_cost = parse_extra_cost(req.extra_cost.as_deref())?;

        let mut active: supplier_product_color::ActiveModel = existing.into();
        let cur_enabled = active.is_enabled.unwrap();
        active.supplier_product_id = Set(req.supplier_product_id);
        active.color_no = Set(req.color_no);
        active.color_name = Set(req.color_name);
        active.pantone_code = Set(req.pantone_code);
        active.extra_cost = Set(extra_cost);
        // 未显式传 is_enabled 时保留原值，避免把已停用色号静默重新启用。
        active.is_enabled = Set(req.is_enabled.unwrap_or(cur_enabled));
        active.updated_at = Set(chrono::Utc::now());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    async fn ensure_product_exists(&self, supplier_product_id: i32) -> Result<(), AppError> {
        supplier_product::Entity::find_by_id(supplier_product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("供应商商品 ID {} 不存在", supplier_product_id))
            })?;
        Ok(())
    }

    /// 同一供应商商品下 color_no 逻辑唯一（表无 DB 唯一约束，故前置查重）。
    async fn ensure_color_unique(
        &self,
        supplier_product_id: i32,
        color_no: &str,
        exclude_id: Option<i32>,
    ) -> Result<(), AppError> {
        let mut q = supplier_product_color::Entity::find()
            .filter(supplier_product_color::Column::SupplierProductId.eq(supplier_product_id))
            .filter(supplier_product_color::Column::ColorNo.eq(color_no));
        if let Some(eid) = exclude_id {
            q = q.filter(supplier_product_color::Column::Id.ne(eid));
        }
        if q.one(&*self.db).await?.is_some() {
            return Err(AppError::business_displayable(
                "该供应商色号在此商品下已存在，请勿重复创建",
            ));
        }
        Ok(())
    }
}

/// 解析可选 extra_cost 字符串为 Decimal，空/缺省落 0，非法格式 → validation。
fn parse_extra_cost(raw: Option<&str>) -> Result<Decimal, AppError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Decimal::from_str(s)
            .map_err(|_| AppError::validation(format!("extra_cost 格式错误: {}", s))),
        None => Ok(Decimal::ZERO),
    }
}

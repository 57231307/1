//! 存货跌价准备服务
//! V15 P2 B08-16：季节性降价/呆滞面料/过期化学品跌价准备计提
use crate::models::inventory_write_down::{ActiveModel, Entity as Iwd, Model};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

#[allow(dead_code, reason = "预留")]
pub struct InventoryWriteDownService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code, reason = "预留")]
impl InventoryWriteDownService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 查询跌价准备列表（分页）。
    ///
    /// 功能：按产品/跌价类型过滤，返回当前页行集与命中总数（按计提期间倒序）。
    /// 调用方：handlers/inventory_write_down_handler.rs::list_write_downs
    ///       （GET /inventory/write-downs）。
    /// 入参：params.page 为 1-based 页码（缺省第 1 页）；params.page_size 为每页行数
    ///       （缺省 20，合法范围 1-100）；page=0 或 page_size 越界按 400
    ///       VALIDATION_ERROR fail-visible 拒绝，不静默夹紧。
    /// 传给谁：SeaORM 分页器交 utils::pagination::paginate_with_total（本仓分页偏移
    ///       唯一权威，内部已做 1-based→0-based 转换，调用方不得再自行减 1）。
    /// 存什么·存哪里：只读查询，不落任何数据。
    pub async fn list(&self, params: ListParams) -> Result<(Vec<Model>, u64), AppError> {
        // 页码语义=1-based（与全站 PaginatedResponse.page 回显口径一致），缺省即第 1 页。
        let page = params.page.unwrap_or(1);
        // 每页上限 100 是全仓统一分页边界（既有站点 clamp(1,100) 的同一取值），
        // 越界处置按本仓口径回 400 点名允许值，不静默夹紧。
        let page_size = params.page_size.unwrap_or(20);
        if page == 0 {
            return Err(AppError::validation_displayable(
                "page 必须是从 1 开始的页码，第一页请传 page=1",
            ));
        }
        if page_size == 0 || page_size > 100 {
            return Err(AppError::validation_displayable(
                "page_size 必须在 1-100 之间（每页返回的行数）",
            ));
        }
        let mut query = Iwd::find();
        if let Some(product_id) = params.product_id {
            query =
                query.filter(crate::models::inventory_write_down::Column::ProductId.eq(product_id));
        }
        if let Some(write_down_type) = params.write_down_type {
            query = query.filter(
                crate::models::inventory_write_down::Column::WriteDownType.eq(write_down_type),
            );
        }
        let paginator = query
            .order_by_desc(crate::models::inventory_write_down::Column::Period)
            .paginate(&*self.db, page_size);
        let (items, total) = paginate_with_total(paginator, page).await?;
        Ok((items, total))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Model, AppError> {
        Iwd::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("跌价准备记录 {} 不存在", id)))
    }

    pub async fn create(&self, data: CreateWriteDownReq) -> Result<Model, AppError> {
        // inventory_write_down.created_at / updated_at 在 DDL 里是 NOT NULL 且无默认值
        // （migration/src/domain/v15/mod.rs:2966）：`..Default::default()` 把这两列留成
        // Unset ⇒ INSERT 传 NULL ⇒ not-null 违例直接 500（CI #4669 用例
        // inventory/04-write-down 的 POST /inventory/write-downs）。
        let now = chrono::Utc::now();
        let active = ActiveModel {
            product_id: Set(data.product_id),
            write_down_type: Set(data.write_down_type),
            original_cost: Set(data.original_cost),
            net_realizable_value: Set(data.net_realizable_value),
            write_down_amount: Set(data.original_cost - data.net_realizable_value),
            reason: Set(data.reason),
            period: Set(data.period),
            status: Set("draft".to_string()),
            created_by: Set(data.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    pub async fn confirm(&self, id: i32, confirmed_by: i32) -> Result<Model, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: ActiveModel = model.into();
        active.status = Set("confirmed".to_string());
        active.confirmed_by = Set(Some(confirmed_by));
        active.confirmed_at = Set(Some(chrono::Utc::now()));
        let model = active.update(&*self.db).await?;
        Ok(model)
    }
}

#[allow(dead_code, reason = "预留")]
pub struct ListParams {
    pub product_id: Option<i32>,
    pub write_down_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code, reason = "预留")]
pub struct CreateWriteDownReq {
    pub product_id: i32,
    pub write_down_type: String,
    pub original_cost: Decimal,
    pub net_realizable_value: Decimal,
    pub reason: Option<String>,
    pub period: chrono::NaiveDate,
    pub created_by: i32,
}

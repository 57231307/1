//! 数据导出子模块（import_export_ops::export）
//!
//! 从原 `import_export_service.rs` 迁移 4 个方法：
//! - `export_data`：导出入口（分发到产品/客户/库存导出）
//! - `export_products`：产品数据导出
//! - `export_customers`：客户数据导出
//! - `export_inventory`：库存数据导出
//!
//! 调用 facade 中的纯函数（`pub(crate)` 可见性）：
//! - `ImportExportService::parse_date_filter`（解析日期过滤字符串）
//! - `ImportExportService::get_export_limit`（获取导出行数上限）
//!
//! 架构优化（2026-06-25）：
//! 1. 不再全表查询，根据 ExportQuery 条件过滤（status / date_from / date_to / keyword）
//! 2. 强制行数上限 MAX_EXPORT_ROWS，避免大表导出 OOM
//! 3. 过滤已删除数据（is_deleted=false）

use crate::services::import_export_service::{ExportQuery, ImportExportService};
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

impl ImportExportService {
    /// 导出数据
    pub async fn export_data(
        &self,
        export_type: &str,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        match export_type {
            "products" => self.export_products(query).await,
            "customers" => self.export_customers(query).await,
            "inventory" => self.export_inventory(query).await,
            _ => Err(AppError::validation(format!(
                "不支持的导出类型: {}",
                export_type
            ))),
        }
    }

    /// 导出产品数据
    async fn export_products(
        &self,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::product::{Column as ProductCol, Entity as ProductEntity};

        let mut db_query = ProductEntity::find().filter(ProductCol::IsDeleted.eq(false));

        if let Some(status) = &query.status {
            db_query = db_query.filter(ProductCol::Status.eq(status.clone()));
        }
        if let Some(keyword) = &query.keyword {
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let like = safe_like_pattern(keyword);
            db_query = db_query.filter(
                ProductCol::Name
                    .like(like.clone())
                    .or(ProductCol::Code.like(like)),
            );
        }
        if let Some(date_from) = &query.date_from {
            if let Some(dt) = ImportExportService::parse_date_filter(date_from) {
                db_query = db_query.filter(ProductCol::CreatedAt.gte(dt));
            }
        }
        if let Some(date_to) = &query.date_to {
            if let Some(dt) = ImportExportService::parse_date_filter(date_to) {
                db_query = db_query.filter(ProductCol::CreatedAt.lte(dt));
            }
        }

        let limit = ImportExportService::get_export_limit(query);
        let products = db_query.limit(limit).all(&*self.db).await?;

        let headers = vec![
            "ID".to_string(),
            "产品编码".to_string(),
            "产品名称".to_string(),
            "单位".to_string(),
            "标准单价".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = products
            .into_iter()
            .map(|p| {
                vec![
                    p.id.to_string(),
                    p.code,
                    p.name,
                    p.unit,
                    p.standard_price.map(|p| p.to_string()).unwrap_or_default(),
                    p.status,
                    p.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect();

        Ok((headers, data))
    }

    /// 导出客户数据
    async fn export_customers(
        &self,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::customer::{Column as CustomerCol, Entity as CustomerEntity};

        let mut db_query = CustomerEntity::find();

        if let Some(status) = &query.status {
            db_query = db_query.filter(CustomerCol::Status.eq(status.clone()));
        }
        if let Some(keyword) = &query.keyword {
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let like = safe_like_pattern(keyword);
            db_query = db_query.filter(
                CustomerCol::CustomerName
                    .like(like.clone())
                    .or(CustomerCol::CustomerCode.like(like)),
            );
        }
        if let Some(date_from) = &query.date_from {
            if let Some(dt) = ImportExportService::parse_date_filter(date_from) {
                db_query = db_query.filter(CustomerCol::CreatedAt.gte(dt));
            }
        }
        if let Some(date_to) = &query.date_to {
            if let Some(dt) = ImportExportService::parse_date_filter(date_to) {
                db_query = db_query.filter(CustomerCol::CreatedAt.lte(dt));
            }
        }

        let limit = ImportExportService::get_export_limit(query);
        let customers = db_query.limit(limit).all(&*self.db).await?;

        let headers = vec![
            "ID".to_string(),
            "客户编码".to_string(),
            "客户名称".to_string(),
            "联系人".to_string(),
            "电话".to_string(),
            "邮箱".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = customers
            .into_iter()
            .map(|c| {
                vec![
                    c.id.to_string(),
                    c.customer_code,
                    c.customer_name,
                    c.contact_person.unwrap_or_default(),
                    c.contact_phone.unwrap_or_default(),
                    c.contact_email.unwrap_or_default(),
                    c.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect();

        Ok((headers, data))
    }

    /// 导出库存数据
    async fn export_inventory(
        &self,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::inventory_stock::{Column as StockCol, Entity as StockEntity};

        let mut db_query = StockEntity::find();

        if let Some(keyword) = &query.keyword {
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let like = safe_like_pattern(keyword);
            db_query = db_query.filter(
                StockCol::BatchNo
                    .like(like.clone())
                    .or(StockCol::ColorNo.like(like)),
            );
        }
        if let Some(date_from) = &query.date_from {
            if let Some(dt) = ImportExportService::parse_date_filter(date_from) {
                db_query = db_query.filter(StockCol::CreatedAt.gte(dt));
            }
        }
        if let Some(date_to) = &query.date_to {
            if let Some(dt) = ImportExportService::parse_date_filter(date_to) {
                db_query = db_query.filter(StockCol::CreatedAt.lte(dt));
            }
        }

        let limit = ImportExportService::get_export_limit(query);
        let stocks = db_query.limit(limit).all(&*self.db).await?;

        let headers = vec![
            "ID".to_string(),
            "产品ID".to_string(),
            "仓库ID".to_string(),
            "可用库存".to_string(),
            "预留库存".to_string(),
            "在途库存".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = stocks
            .into_iter()
            .map(|s| {
                vec![
                    s.id.to_string(),
                    s.product_id.to_string(),
                    s.warehouse_id.to_string(),
                    s.quantity_available.to_string(),
                    s.quantity_reserved.to_string(),
                    s.quantity_incoming.to_string(),
                    s.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect();

        Ok((headers, data))
    }
}

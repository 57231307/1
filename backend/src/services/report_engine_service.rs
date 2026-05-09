//! 报表引擎 Service
//!
//! 提供报表模板管理和数据导出功能

use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::purchase_order::Entity as PurchaseOrderEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::utils::error::AppError;

/// 报表类型
#[derive(Debug, Clone)]
pub enum ReportType {
    Sales,
    Purchase,
    Inventory,
    Financial,
    Custom,
}

/// 报表模板
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub columns: Vec<ReportColumn>,
    pub filters: Vec<ReportFilter>,
    pub sort_by: Option<String>,
    pub sort_order: String,
}

/// 报表列定义
#[derive(Debug, Clone)]
pub struct ReportColumn {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub width: Option<i32>,
    pub format: Option<String>,
}

/// 报表筛选条件
#[derive(Debug, Clone)]
pub struct ReportFilter {
    pub field: String,
    pub operator: String,
    pub value: String,
}

/// 报表数据
#[derive(Debug, Clone)]
pub struct ReportData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
}

/// 导出格式
#[derive(Debug, Clone)]
pub enum ExportFormat {
    CSV,
    Excel,
    PDF,
    JSON,
}

/// 报表引擎 Service
pub struct ReportEngineService {
    db: Arc<DatabaseConnection>,
}

impl ReportEngineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取预定义报表模板
    pub fn get_predefined_templates() -> Vec<ReportTemplate> {
        vec![
            ReportTemplate {
                id: "sales_daily".to_string(),
                name: "销售日报表".to_string(),
                report_type: ReportType::Sales,
                columns: vec![
                    ReportColumn { field: "order_no".to_string(), title: "订单编号".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                    ReportColumn { field: "customer_name".to_string(), title: "客户名称".to_string(), data_type: "string".to_string(), width: Some(200), format: None },
                    ReportColumn { field: "order_date".to_string(), title: "订单日期".to_string(), data_type: "date".to_string(), width: Some(120), format: Some("YYYY-MM-DD".to_string()) },
                    ReportColumn { field: "total_amount".to_string(), title: "订单金额".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "status".to_string(), title: "状态".to_string(), data_type: "string".to_string(), width: Some(100), format: None },
                ],
                filters: vec![
                    ReportFilter { field: "order_date".to_string(), operator: ">=".to_string(), value: "today-30".to_string() },
                ],
                sort_by: Some("order_date".to_string()),
                sort_order: "desc".to_string(),
            },
            ReportTemplate {
                id: "inventory_status".to_string(),
                name: "库存状态报表".to_string(),
                report_type: ReportType::Inventory,
                columns: vec![
                    ReportColumn { field: "product_code".to_string(), title: "产品编码".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                    ReportColumn { field: "product_name".to_string(), title: "产品名称".to_string(), data_type: "string".to_string(), width: Some(200), format: None },
                    ReportColumn { field: "quantity_available".to_string(), title: "可用库存".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "quantity_reserved".to_string(), title: "预留库存".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "warehouse".to_string(), title: "仓库".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                ],
                filters: vec![],
                sort_by: Some("quantity_available".to_string()),
                sort_order: "desc".to_string(),
            },
            ReportTemplate {
                id: "purchase_summary".to_string(),
                name: "采购汇总报表".to_string(),
                report_type: ReportType::Purchase,
                columns: vec![
                    ReportColumn { field: "order_no".to_string(), title: "采购单号".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                    ReportColumn { field: "supplier_name".to_string(), title: "供应商".to_string(), data_type: "string".to_string(), width: Some(200), format: None },
                    ReportColumn { field: "order_date".to_string(), title: "下单日期".to_string(), data_type: "date".to_string(), width: Some(120), format: Some("YYYY-MM-DD".to_string()) },
                    ReportColumn { field: "total_amount".to_string(), title: "采购金额".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "delivery_date".to_string(), title: "交期".to_string(), data_type: "date".to_string(), width: Some(120), format: Some("YYYY-MM-DD".to_string()) },
                ],
                filters: vec![
                    ReportFilter { field: "order_date".to_string(), operator: ">=".to_string(), value: "today-30".to_string() },
                ],
                sort_by: Some("order_date".to_string()),
                sort_order: "desc".to_string(),
            },
        ]
    }

    /// 执行报表查询
    pub async fn execute_report(
        &self,
        template_id: &str,
        custom_filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        match template_id {
            "sales_daily" => self.query_sales_report(custom_filters, page, page_size).await,
            "inventory_status" => self.query_inventory_report(custom_filters, page, page_size).await,
            "purchase_summary" => self.query_purchase_report(custom_filters, page, page_size).await,
            _ => Err(AppError::NotFound(format!("报表模板 {} 不存在", template_id))),
        }
    }

    /// 销售报表查询
    async fn query_sales_report(
        &self,
        _filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let paginator = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::IsDeleted.eq(false))
            .order_by_desc(crate::models::sales_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let orders = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let columns = vec![
            "订单编号".to_string(),
            "客户ID".to_string(),
            "订单金额".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = orders
            .into_iter()
            .map(|o| {
                vec![
                    o.order_no,
                    o.customer_id.to_string(),
                    o.total_amount.to_string(),
                    o.status,
                    o.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 库存报表查询
    async fn query_inventory_report(
        &self,
        _filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let paginator = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::IsDeleted.eq(false))
            .order_by_desc(crate::models::inventory_stock::Column::QuantityAvailable)
            .paginate(&*self.db, page_size);

        let stocks = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let columns = vec![
            "产品ID".to_string(),
            "可用库存".to_string(),
            "预留库存".to_string(),
            "在途库存".to_string(),
            "仓库ID".to_string(),
        ];

        let rows: Vec<Vec<String>> = stocks
            .into_iter()
            .map(|s| {
                vec![
                    s.product_id.to_string(),
                    s.quantity_available.to_string(),
                    s.quantity_reserved.to_string(),
                    s.quantity_incoming.to_string(),
                    s.warehouse_id.to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 采购报表查询
    async fn query_purchase_report(
        &self,
        _filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let paginator = PurchaseOrderEntity::find()
            .filter(crate::models::purchase_order::Column::IsDeleted.eq(false))
            .order_by_desc(crate::models::purchase_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let orders = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total = PurchaseOrderEntity::find()
            .filter(crate::models::purchase_order::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let columns = vec![
            "采购单号".to_string(),
            "供应商ID".to_string(),
            "采购金额".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = orders
            .into_iter()
            .map(|o| {
                vec![
                    o.order_no,
                    o.supplier_id.to_string(),
                    o.total_amount.to_string(),
                    o.order_status,
                    o.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 导出报表数据
    pub fn export_report(
        &self,
        data: &ReportData,
        format: ExportFormat,
    ) -> Result<Vec<u8>, AppError> {
        match format {
            ExportFormat::CSV => self.export_csv(data),
            ExportFormat::JSON => self.export_json(data),
            _ => Err(AppError::ValidationError("该导出格式暂未支持".to_string())),
        }
    }

    /// 导出CSV格式
    fn export_csv(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let mut csv = String::new();
        
        // 表头
        csv.push_str(&data.columns.join(","));
        csv.push('\n');
        
        // 数据行
        for row in &data.rows {
            let escaped: Vec<String> = row
                .iter()
                .map(|cell| {
                    if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                        format!("\"{}\"", cell.replace('"', "\"\""))
                    } else {
                        cell.clone()
                    }
                })
                .collect();
            csv.push_str(&escaped.join(","));
            csv.push('\n');
        }
        
        Ok(csv.into_bytes())
    }

    /// 导出JSON格式
    fn export_json(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let json_data: Vec<serde_json::Map<String, serde_json::Value>> = data
            .rows
            .iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, col) in data.columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        map.insert(col.clone(), serde_json::Value::String(val.clone()));
                    }
                }
                map
            })
            .collect();

        let json = serde_json::to_string(&json_data)
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        
        Ok(json.into_bytes())
    }
}

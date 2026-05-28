//! 导入导出 Service
//!
//! 提供 CSV/Excel 数据导入导出功能

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::utils::error::AppError;

/// 导入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: u64,
    pub failed: u64,
    pub errors: Vec<ImportError>,
}

/// 导入错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub row: u64,
    pub field: Option<String>,
    pub message: String,
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormatType {
    CSV,
    Excel,
}

/// 导入模板定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplate {
    pub import_type: String,
    pub name: String,
    pub description: String,
    pub columns: Vec<ImportColumnDef>,
}

/// 导入列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportColumnDef {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub required: bool,
    pub example: Option<String>,
}

/// 导入导出 Service
pub struct ImportExportService {
    db: Arc<DatabaseConnection>,
}

impl ImportExportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取导入模板
    pub fn get_import_template(import_type: &str) -> Result<ImportTemplate, AppError> {
        match import_type {
            "products" => Ok(ImportTemplate {
                import_type: "products".to_string(),
                name: "产品导入模板".to_string(),
                description: "用于批量导入产品信息".to_string(),
                columns: vec![
                    ImportColumnDef {
                        field: "code".to_string(),
                        title: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("P001".to_string()),
                    },
                    ImportColumnDef {
                        field: "name".to_string(),
                        title: "产品名称".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("棉布A".to_string()),
                    },
                    ImportColumnDef {
                        field: "category".to_string(),
                        title: "产品类别".to_string(),
                        data_type: "string".to_string(),
                        required: false,
                        example: Some("面料".to_string()),
                    },
                    ImportColumnDef {
                        field: "unit".to_string(),
                        title: "单位".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("米".to_string()),
                    },
                    ImportColumnDef {
                        field: "price".to_string(),
                        title: "单价".to_string(),
                        data_type: "decimal".to_string(),
                        required: false,
                        example: Some("15.50".to_string()),
                    },
                ],
            }),
            "customers" => Ok(ImportTemplate {
                import_type: "customers".to_string(),
                name: "客户导入模板".to_string(),
                description: "用于批量导入客户信息".to_string(),
                columns: vec![
                    ImportColumnDef {
                        field: "code".to_string(),
                        title: "客户编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("C001".to_string()),
                    },
                    ImportColumnDef {
                        field: "name".to_string(),
                        title: "客户名称".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("ABC公司".to_string()),
                    },
                    ImportColumnDef {
                        field: "contact".to_string(),
                        title: "联系人".to_string(),
                        data_type: "string".to_string(),
                        required: false,
                        example: Some("张三".to_string()),
                    },
                    ImportColumnDef {
                        field: "phone".to_string(),
                        title: "电话".to_string(),
                        data_type: "string".to_string(),
                        required: false,
                        example: Some("13800138000".to_string()),
                    },
                ],
            }),
            "inventory" => Ok(ImportTemplate {
                import_type: "inventory".to_string(),
                name: "库存导入模板".to_string(),
                description: "用于批量导入库存信息".to_string(),
                columns: vec![
                    ImportColumnDef {
                        field: "product_code".to_string(),
                        title: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("P001".to_string()),
                    },
                    ImportColumnDef {
                        field: "warehouse_code".to_string(),
                        title: "仓库编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("WH01".to_string()),
                    },
                    ImportColumnDef {
                        field: "quantity".to_string(),
                        title: "数量".to_string(),
                        data_type: "decimal".to_string(),
                        required: true,
                        example: Some("1000".to_string()),
                    },
                ],
            }),
            _ => Err(AppError::ValidationError(format!(
                "不支持的导入类型: {}",
                import_type
            ))),
        }
    }

    /// 解析CSV内容
    pub fn parse_csv(content: &str) -> Result<Vec<Vec<String>>, AppError> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| AppError::ValidationError(format!("CSV解析错误: {}", e)))?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            rows.push(row);
        }

        Ok(rows)
    }

    /// 生成CSV内容
    pub fn generate_csv(headers: &[String], data: &[Vec<String>]) -> Result<String, AppError> {
        let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);

        // 写入表头
        wtr.write_record(headers)
            .map_err(|e| AppError::ValidationError(format!("CSV写入错误: {}", e)))?;

        // 写入数据行
        for row in data {
            wtr.write_record(row)
                .map_err(|e| AppError::ValidationError(format!("CSV写入错误: {}", e)))?;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| AppError::ValidationError(format!("CSV序列化错误: {}", e)))?;

        String::from_utf8(data).map_err(|e| AppError::ValidationError(format!("CSV编码错误: {}", e)))
    }

    /// 验证导入数据
    pub fn validate_import_data(
        data: &[Vec<String>],
        template: &ImportTemplate,
    ) -> Vec<ImportError> {
        let mut errors = Vec::new();

        for (row_idx, row) in data.iter().enumerate() {
            for (col_idx, col_def) in template.columns.iter().enumerate() {
                if col_def.required {
                    let value = row.get(col_idx).map(|s| s.trim()).unwrap_or("");
                    if value.is_empty() {
                        errors.push(ImportError {
                            row: (row_idx + 1) as u64,
                            field: Some(col_def.field.clone()),
                            message: format!("必填字段 '{}' 为空", col_def.title),
                        });
                    }
                }

                // 数据类型验证
                if let Some(value) = row.get(col_idx) {
                    let value = value.trim();
                    if !value.is_empty() {
                        match col_def.data_type.as_str() {
                            "decimal" if value.parse::<f64>().is_err() => {
                                errors.push(ImportError {
                                    row: (row_idx + 1) as u64,
                                    field: Some(col_def.field.clone()),
                                    message: format!(
                                        "'{}' 不是有效的数字格式",
                                        col_def.title
                                    ),
                                });
                            }
                            "integer" if value.parse::<i64>().is_err() => {
                                errors.push(ImportError {
                                    row: (row_idx + 1) as u64,
                                    field: Some(col_def.field.clone()),
                                    message: format!(
                                        "'{}' 不是有效的整数格式",
                                        col_def.title
                                    ),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        errors
    }

    /// 执行数据导入
    pub async fn import_data(
        &self,
        import_type: &str,
        data: &[Vec<String>],
        user_id: i32,
    ) -> Result<ImportResult, AppError> {
        let mut imported = 0u64;
        let mut failed = 0u64;
        let mut errors = Vec::new();

        match import_type {
            "products" => {
                for (idx, row) in data.iter().enumerate() {
                    match self.import_product_row(row, user_id).await {
                        Ok(_) => imported += 1,
                        Err(e) => {
                            failed += 1;
                            errors.push(ImportError {
                                row: (idx + 1) as u64,
                                field: None,
                                message: e.to_string(),
                            });
                        }
                    }
                }
            }
            "customers" => {
                for (idx, row) in data.iter().enumerate() {
                    match self.import_customer_row(row, user_id).await {
                        Ok(_) => imported += 1,
                        Err(e) => {
                            failed += 1;
                            errors.push(ImportError {
                                row: (idx + 1) as u64,
                                field: None,
                                message: e.to_string(),
                            });
                        }
                    }
                }
            }
            _ => {
                return Err(AppError::ValidationError(format!(
                    "不支持的导入类型: {}",
                    import_type
                )));
            }
        }

        Ok(ImportResult {
            imported,
            failed,
            errors,
        })
    }

    /// 导入产品行
    async fn import_product_row(
        &self,
        row: &[String],
        _user_id: i32,
    ) -> Result<(), AppError> {
        use sea_orm::Set;
        use crate::models::product::{ActiveModel as ProductActiveModel, Entity as ProductEntity};

        let code = row.first().map(|s| s.trim().to_string()).unwrap_or_default();
        let name = row.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        let _category = row.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
        let unit = row.get(3).map(|s| s.trim().to_string()).unwrap_or("个".to_string());
        let price = row.get(4)
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);

        if code.is_empty() || name.is_empty() {
            return Err(AppError::ValidationError("产品编码和名称不能为空".to_string()));
        }

        // 检查编码是否已存在
        let existing = ProductEntity::find()
            .filter(crate::models::product::Column::Code.eq(&code))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::BusinessError(format!("产品编码 {} 已存在", code)));
        }

        let now = chrono::Utc::now();
        let active_model = ProductActiveModel {
            id: Default::default(),
            code: Set(code),
            name: Set(name),
            category_id: Set(None),
            specification: Set(None),
            unit: Set(unit),
            standard_price: Set(Some(rust_decimal::Decimal::from_f64_retain(price).unwrap_or_default())),
            cost_price: Set(None),
            description: Set(None),
            status: Set("ACTIVE".to_string()),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            product_type: Set("GENERAL".to_string()),
            fabric_composition: Set(None),
            yarn_count: Set(None),
            density: Set(None),
            width: Set(None),
            gram_weight: Set(None),
            structure: Set(None),
            finish: Set(None),
            min_order_quantity: Set(None),
            lead_time: Set(None),
            supplier_product_code: Set(None),
            supplier_id: Set(None),
            is_batch_managed: Set(None),
            batch_level: Set(None),
        };

        active_model.insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 导入客户行
    async fn import_customer_row(
        &self,
        row: &[String],
        user_id: i32,
    ) -> Result<(), AppError> {
        use sea_orm::Set;
        use crate::models::customer::{ActiveModel as CustomerActiveModel, Entity as CustomerEntity};

        let code = row.first().map(|s| s.trim().to_string()).unwrap_or_default();
        let name = row.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        let contact = row.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
        let phone = row.get(3).map(|s| s.trim().to_string()).unwrap_or_default();

        if code.is_empty() || name.is_empty() {
            return Err(AppError::ValidationError("客户编码和名称不能为空".to_string()));
        }

        // 检查编码是否已存在
        let existing = CustomerEntity::find()
            .filter(crate::models::customer::Column::CustomerCode.eq(&code))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::BusinessError(format!("客户编码 {} 已存在", code)));
        }

        let now = chrono::Utc::now();
        let active_model = CustomerActiveModel {
            id: Default::default(),
            customer_code: Set(code),
            customer_name: Set(name),
            contact_person: Set(Some(contact)),
            contact_phone: Set(Some(phone)),
            contact_email: Set(None),
            address: Set(None),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            credit_limit: Set(rust_decimal::Decimal::ZERO),
            payment_terms: Set(30),
            tax_id: Set(None),
            bank_name: Set(None),
            bank_account: Set(None),
            status: Set("ACTIVE".to_string()),
            customer_type: Set("RETAIL".to_string()),
            notes: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            customer_industry: Set(None),
            main_products: Set(None),
            annual_purchase: Set(None),
            quality_requirement: Set(None),
            inspection_standard: Set(None),
        };

        active_model.insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

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
            _ => Err(AppError::ValidationError(format!(
                "不支持的导出类型: {}",
                export_type
            ))),
        }
    }

    /// 导出产品数据
    async fn export_products(
        &self,
        _query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::product::Entity as ProductEntity;

        let products = ProductEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let headers = vec![
            "ID".to_string(),
            "产品编码".to_string(),
            "产品名称".to_string(),
            "单位".to_string(),
            "标准单价".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = products.into_iter().map(|p| {
            vec![
                p.id.to_string(),
                p.code,
                p.name,
                p.unit,
                p.standard_price.map(|p| p.to_string()).unwrap_or_default(),
                p.status,
                p.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ]
        }).collect();

        Ok((headers, data))
    }

    /// 导出客户数据
    async fn export_customers(
        &self,
        _query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::customer::Entity as CustomerEntity;

        let customers = CustomerEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let headers = vec![
            "ID".to_string(),
            "客户编码".to_string(),
            "客户名称".to_string(),
            "联系人".to_string(),
            "电话".to_string(),
            "邮箱".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = customers.into_iter().map(|c| {
            vec![
                c.id.to_string(),
                c.customer_code,
                c.customer_name,
                c.contact_person.unwrap_or_default(),
                c.contact_phone.unwrap_or_default(),
                c.contact_email.unwrap_or_default(),
                c.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ]
        }).collect();

        Ok((headers, data))
    }

    /// 导出库存数据
    async fn export_inventory(
        &self,
        _query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::inventory_stock::Entity as InventoryStockEntity;

        let stocks = InventoryStockEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let headers = vec![
            "ID".to_string(),
            "产品ID".to_string(),
            "仓库ID".to_string(),
            "可用库存".to_string(),
            "预留库存".to_string(),
            "在途库存".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = stocks.into_iter().map(|s| {
            vec![
                s.id.to_string(),
                s.product_id.to_string(),
                s.warehouse_id.to_string(),
                s.quantity_available.to_string(),
                s.quantity_reserved.to_string(),
                s.quantity_incoming.to_string(),
                s.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ]
        }).collect();

        Ok((headers, data))
    }
}

/// 导出查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub status: Option<String>,
}

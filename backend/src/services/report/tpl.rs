//! 报表模板服务（report/tpl）
//!
//! 包含报表模板的查询与管理：
//! - `get_predefined_templates` 返回 9 个内置预定义模板
//! - `create_custom_template` 创建用户自定义模板
//! - `get_all_templates` 合并预定义 + 自定义
//! - `get_template` 按 ID 获取单个模板
//!
//! 拆分自原 `report_engine_service.rs` 的"报表模板管理"段。

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::info;

use crate::models::report_template::{self, Entity as ReportTemplateEntity};
use crate::utils::error::AppError;

use super::{CreateTemplateRequest, ReportEngineService, ReportTemplate};

impl ReportEngineService {
    /// 获取预定义报表模板
    pub fn get_predefined_templates(&self) -> Vec<ReportTemplate> {
        use super::{
            ReportColumn as Rc, ReportFilter as Rf, ReportParameter as Rp,
        };

        vec![
            ReportTemplate {
                id: "sales_summary".to_string(),
                name: "销售汇总报表".to_string(),
                description: "按时间段统计销售总额、订单数、客户数等汇总数据".to_string(),
                category: "sales".to_string(),
                data_source: "sales".to_string(),
                columns: vec![
                    Rc {
                        key: "period".to_string(),
                        label: "期间".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: Some("group".to_string()),
                        sortable: true,
                        filterable: true,
                        width: Some(120),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "total_amount".to_string(),
                        label: "销售总额".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(150),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "order_count".to_string(),
                        label: "订单数".to_string(),
                        data_type: "integer".to_string(),
                        format: Some("#,##0".to_string()),
                        aggregation: Some("count".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "customer_count".to_string(),
                        label: "客户数".to_string(),
                        data_type: "integer".to_string(),
                        format: Some("#,##0".to_string()),
                        aggregation: Some("count".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "date_range".to_string(),
                    label: "统计期间".to_string(),
                    filter_type: "date_range".to_string(),
                    default_value: None,
                    options: None,
                    required: true,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string(), "csv".to_string()],
                parameters: vec![Rp {
                    name: "group_by".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    default_value: Some(serde_json::Value::String("day".to_string())),
                    description: Some("分组方式: day, week, month, year".to_string()),
                }],
            },
            ReportTemplate {
                id: "sales_detail".to_string(),
                name: "销售明细报表".to_string(),
                description: "列出每笔销售订单的详细信息".to_string(),
                category: "sales".to_string(),
                data_source: "sales".to_string(),
                columns: vec![
                    Rc {
                        key: "order_no".to_string(),
                        label: "订单号".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "customer_name".to_string(),
                        label: "客户".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "product_name".to_string(),
                        label: "产品".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "quantity".to_string(),
                        label: "数量".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "amount".to_string(),
                        label: "金额".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![
                    Rf {
                        key: "date_range".to_string(),
                        label: "订单日期".to_string(),
                        filter_type: "date_range".to_string(),
                        default_value: None,
                        options: None,
                        required: true,
                    },
                    Rf {
                        key: "customer_id".to_string(),
                        label: "客户".to_string(),
                        filter_type: "select".to_string(),
                        default_value: None,
                        options: None,
                        required: false,
                    },
                ],
                supported_formats: vec![
                    "excel".to_string(),
                    "pdf".to_string(),
                    "csv".to_string(),
                    "json".to_string(),
                ],
                parameters: vec![],
            },
            ReportTemplate {
                id: "inventory_status".to_string(),
                name: "库存状态报表".to_string(),
                description: "查询各仓库各产品的库存状态，包括在库、可用、预留等数量".to_string(),
                category: "inventory".to_string(),
                data_source: "inventory".to_string(),
                columns: vec![
                    Rc {
                        key: "warehouse_name".to_string(),
                        label: "仓库".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "product_code".to_string(),
                        label: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: true,
                        width: Some(120),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "product_name".to_string(),
                        label: "产品名称".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "quantity_on_hand".to_string(),
                        label: "在库数量".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "quantity_available".to_string(),
                        label: "可用数量".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "warehouse_id".to_string(),
                    label: "仓库".to_string(),
                    filter_type: "select".to_string(),
                    default_value: None,
                    options: None,
                    required: false,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![],
            },
            ReportTemplate {
                id: "purchase_summary".to_string(),
                name: "采购汇总报表".to_string(),
                description: "按供应商和时间段统计采购总额、订单数".to_string(),
                category: "purchase".to_string(),
                data_source: "purchase".to_string(),
                columns: vec![
                    Rc {
                        key: "supplier_name".to_string(),
                        label: "供应商".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: Some("group".to_string()),
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "total_amount".to_string(),
                        label: "采购总额".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(150),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "order_count".to_string(),
                        label: "订单数".to_string(),
                        data_type: "integer".to_string(),
                        format: Some("#,##0".to_string()),
                        aggregation: Some("count".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "date_range".to_string(),
                    label: "采购期间".to_string(),
                    filter_type: "date_range".to_string(),
                    default_value: None,
                    options: None,
                    required: true,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![],
            },
            ReportTemplate {
                id: "ar_aging".to_string(),
                name: "应收账款账龄分析".to_string(),
                description: "按客户和账龄段分析应收账款分布".to_string(),
                category: "finance".to_string(),
                data_source: "ar_aging".to_string(),
                columns: vec![
                    Rc {
                        key: "customer_name".to_string(),
                        label: "客户".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: Some("group".to_string()),
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "current".to_string(),
                        label: "当期".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "1_30_days".to_string(),
                        label: "1-30天".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "31_60_days".to_string(),
                        label: "31-60天".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "over_60_days".to_string(),
                        label: "60天以上".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![],
            },
            ReportTemplate {
                id: "top_products".to_string(),
                name: "畅销产品报表".to_string(),
                description: "按销量或销售额统计TOP N产品".to_string(),
                category: "sales".to_string(),
                data_source: "sales".to_string(),
                columns: vec![
                    Rc {
                        key: "product_code".to_string(),
                        label: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "product_name".to_string(),
                        label: "产品名称".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "total_quantity".to_string(),
                        label: "销售数量".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "total_amount".to_string(),
                        label: "销售金额".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(150),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "date_range".to_string(),
                    label: "统计期间".to_string(),
                    filter_type: "date_range".to_string(),
                    default_value: None,
                    options: None,
                    required: true,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![Rp {
                    name: "top_n".to_string(),
                    param_type: "integer".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(20)),
                    description: Some("TOP N 数量".to_string()),
                }],
            },
            ReportTemplate {
                id: "customer_analysis".to_string(),
                name: "客户分析报表".to_string(),
                description: "按客户分析销售额、订单数、客单价等".to_string(),
                category: "sales".to_string(),
                data_source: "sales".to_string(),
                columns: vec![
                    Rc {
                        key: "customer_name".to_string(),
                        label: "客户".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: Some("group".to_string()),
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "order_count".to_string(),
                        label: "订单数".to_string(),
                        data_type: "integer".to_string(),
                        format: Some("#,##0".to_string()),
                        aggregation: Some("count".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "total_amount".to_string(),
                        label: "销售总额".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(150),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "avg_order_amount".to_string(),
                        label: "客单价".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("avg".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "date_range".to_string(),
                    label: "统计期间".to_string(),
                    filter_type: "date_range".to_string(),
                    default_value: None,
                    options: None,
                    required: true,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![],
            },
            ReportTemplate {
                id: "profit_analysis".to_string(),
                name: "利润分析报表".to_string(),
                description: "按产品/客户/期间分析销售收入、成本和毛利".to_string(),
                category: "finance".to_string(),
                data_source: "sales".to_string(),
                columns: vec![
                    Rc {
                        key: "product_name".to_string(),
                        label: "产品".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: Some("group".to_string()),
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "revenue".to_string(),
                        label: "收入".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "cost".to_string(),
                        label: "成本".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "profit".to_string(),
                        label: "毛利".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "profit_margin".to_string(),
                        label: "毛利率".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("0.00%".to_string()),
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "date_range".to_string(),
                    label: "统计期间".to_string(),
                    filter_type: "date_range".to_string(),
                    default_value: None,
                    options: None,
                    required: true,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![],
            },
            ReportTemplate {
                id: "inventory_turnover".to_string(),
                name: "库存周转率报表".to_string(),
                description: "按产品/仓库分析库存周转率".to_string(),
                category: "inventory".to_string(),
                data_source: "inventory".to_string(),
                columns: vec![
                    Rc {
                        key: "product_name".to_string(),
                        label: "产品".to_string(),
                        data_type: "string".to_string(),
                        format: None,
                        aggregation: Some("group".to_string()),
                        sortable: true,
                        filterable: true,
                        width: Some(150),
                        alignment: Some("left".to_string()),
                    },
                    Rc {
                        key: "avg_stock".to_string(),
                        label: "平均库存".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("avg".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "outbound".to_string(),
                        label: "出库量".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("#,##0.00".to_string()),
                        aggregation: Some("sum".to_string()),
                        sortable: true,
                        filterable: false,
                        width: Some(120),
                        alignment: Some("right".to_string()),
                    },
                    Rc {
                        key: "turnover_rate".to_string(),
                        label: "周转率".to_string(),
                        data_type: "decimal".to_string(),
                        format: Some("0.00".to_string()),
                        aggregation: None,
                        sortable: true,
                        filterable: false,
                        width: Some(100),
                        alignment: Some("right".to_string()),
                    },
                ],
                filters: vec![Rf {
                    key: "date_range".to_string(),
                    label: "统计期间".to_string(),
                    filter_type: "date_range".to_string(),
                    default_value: None,
                    options: None,
                    required: true,
                }],
                supported_formats: vec!["excel".to_string(), "pdf".to_string()],
                parameters: vec![],
            },
        ]
    }

    /// 创建自定义报表模板
    pub async fn create_custom_template(
        &self,
        user_id: i32,
        req: CreateTemplateRequest,
    ) -> Result<ReportTemplate, AppError> {
        // 验证输入
        if req.name.is_empty() {
            return Err(AppError::bad_request("模板名称不能为空".to_string()));
        }
        if req.data_source.is_empty() {
            return Err(AppError::bad_request("数据源不能为空".to_string()));
        }

        // 序列化列、过滤、参数为 JSON
        let columns_json = serde_json::to_string(&req.columns)
            .map_err(|e| AppError::internal(format!("序列化列定义失败: {}", e)))?;
        let filters_json = serde_json::to_string(&req.filters)
            .map_err(|e| AppError::internal(format!("序列化筛选条件失败: {}", e)))?;
        let parameters_json = serde_json::to_string(&req.parameters)
            .map_err(|e| AppError::internal(format!("序列化参数失败: {}", e)))?;
        let formats_json = serde_json::to_string(&req.supported_formats)
            .map_err(|e| AppError::internal(format!("序列化支持格式失败: {}", e)))?;

        let now = Utc::now();
        let active_model = report_template::ActiveModel {
            id: Default::default(),
            template_id: Set(format!("custom_{}", chrono::Utc::now().timestamp_millis())),
            template_name: Set(req.name.clone()),
            description: Set(Some(req.description.clone())),
            category: Set(req.category.clone()),
            data_source: Set(req.data_source.clone()),
            columns: Set(columns_json),
            filters: Set(filters_json),
            parameters: Set(parameters_json),
            supported_formats: Set(formats_json),
            created_by: Set(user_id),
            is_public: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        info!(
            "创建自定义报表模板成功：id={}, name={}",
            model.id, model.template_name
        );

        Ok(ReportTemplate {
            id: model.template_id,
            name: model.template_name,
            description: model.description.unwrap_or_default(),
            category: model.category,
            data_source: model.data_source,
            columns: req.columns,
            filters: req.filters,
            supported_formats: req.supported_formats,
            parameters: req.parameters,
        })
    }

    /// 获取所有模板（预定义 + 自定义）
    pub async fn get_all_templates(
        &self,
        user_id: Option<i32>,
        category: Option<String>,
    ) -> Result<Vec<ReportTemplate>, AppError> {
        let mut templates = self.get_predefined_templates();

        // 加载自定义模板
        let mut query = ReportTemplateEntity::find();
        if let Some(uid) = user_id {
            query = query.filter(report_template::Column::CreatedBy.eq(uid));
        }
        if let Some(ref cat) = category {
            query = query.filter(report_template::Column::Category.eq(cat));
        }

        let custom_templates = query.all(&*self.db).await?;

        for ct in custom_templates {
            // 反序列化 JSON
            let columns: Vec<crate::services::report::ReportColumn> = serde_json::from_str(&ct.columns).unwrap_or_default();
            let filters: Vec<crate::services::report::ReportFilter> = serde_json::from_str(&ct.filters).unwrap_or_default();
            let parameters: Vec<crate::services::report::ReportParameter> =
                serde_json::from_str(&ct.parameters).unwrap_or_default();
            let formats: Vec<String> = serde_json::from_str(&ct.supported_formats).unwrap_or_default();

            templates.push(ReportTemplate {
                id: ct.template_id,
                name: ct.template_name,
                description: ct.description.unwrap_or_default(),
                category: ct.category,
                data_source: ct.data_source,
                columns,
                filters,
                supported_formats: formats,
                parameters,
            });
        }

        Ok(templates)
    }

    /// 根据 template_id 获取模板（按 id 优先匹配自定义，fallback 到预定义）
    pub async fn get_template(&self, template_id: &str) -> Result<ReportTemplate, AppError> {
        let predefined = self.get_predefined_templates();
        if let Some(t) = predefined.iter().find(|t| t.id == template_id) {
            return Ok(t.clone());
        }

        let ct = ReportTemplateEntity::find()
            .filter(report_template::Column::TemplateId.eq(template_id))
            .one(&*self.db)
            .await?;

        if let Some(ct) = ct {
            let columns: Vec<crate::services::report::ReportColumn> = serde_json::from_str(&ct.columns).unwrap_or_default();
            let filters: Vec<crate::services::report::ReportFilter> = serde_json::from_str(&ct.filters).unwrap_or_default();
            let parameters: Vec<crate::services::report::ReportParameter> =
                serde_json::from_str(&ct.parameters).unwrap_or_default();
            let formats: Vec<String> = serde_json::from_str(&ct.supported_formats).unwrap_or_default();

            Ok(ReportTemplate {
                id: ct.template_id,
                name: ct.template_name,
                description: ct.description.unwrap_or_default(),
                category: ct.category,
                data_source: ct.data_source,
                columns,
                filters,
                supported_formats: formats,
                parameters,
            })
        } else {
            Err(AppError::not_found(format!(
                "报表模板 {} 不存在",
                template_id
            )))
        }
    }
}

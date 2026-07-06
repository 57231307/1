//! 报表模板服务（report/tpl）
//!
//! 包含报表模板的查询与管理：
//! - `get_predefined_templates` 返回 9 个内置预定义模板
//! - `create_custom_template` 创建用户自定义模板
//! - `get_all_templates` 合并预定义 + 自定义
//! - `get_template` 按 ID 获取单个模板
//!
//! 拆分自原 `report_engine_service.rs` 的"报表模板管理"段。

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::report_template::{self, Entity as ReportTemplateEntity};
use crate::utils::error::AppError;

use super::{ReportEngineService, ReportTemplate};

impl ReportEngineService {
    /// 获取预定义报表模板
    pub fn get_predefined_templates(&self) -> Vec<ReportTemplate> {
        use super::{ReportColumn as Rc, ReportFilter as Rf, ReportParameter as Rp};

        vec![
            ReportTemplate {
                id: "sales_summary".to_string(),
                name: "销售汇总报表".to_string(),
                description: "按时间段统计销售总额、订单数、客户数等汇总数据".to_string(),
                category: "sales".to_string(),
                data_source: "sales".to_string(),
                report_type: "sales".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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
                report_type: "sales".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
                        operator: None,
                        value: None,
                        key: "date_range".to_string(),
                        label: "订单日期".to_string(),
                        filter_type: "date_range".to_string(),
                        default_value: None,
                        options: None,
                        required: true,
                    },
                    Rf {
                        field_alias: None,
                        operator: None,
                        value: None,
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
                report_type: "inventory".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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
                report_type: "purchase".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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
                report_type: "ar_aging".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                report_type: "sales".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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
                report_type: "sales".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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
                report_type: "sales".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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
                report_type: "inventory".to_string(),
                columns: vec![
                    Rc {
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                        field_alias: None,
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
                    field_alias: None,
                    operator: None,
                    value: None,
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

    /// 根据 template_id 获取模板（按 id 优先匹配自定义，fallback 到预定义）
    pub async fn get_template(&self, template_id: &str) -> Result<ReportTemplate, AppError> {
        let predefined = self.get_predefined_templates();
        if let Some(t) = predefined.iter().find(|t| t.id == template_id) {
            return Ok(t.clone());
        }

        // 先按 template_id 字段查，再按 code 字段查
        let ct_opt = ReportTemplateEntity::find()
            .filter(report_template::Column::TemplateId.eq(template_id))
            .one(&*self.db)
            .await?;
        let ct = if let Some(c) = ct_opt {
            Some(c)
        } else {
            ReportTemplateEntity::find()
                .filter(report_template::Column::Code.eq(template_id))
                .one(&*self.db)
                .await?
        };

        if let Some(ct) = ct {
            let columns: Vec<crate::services::report::ReportColumn> =
                serde_json::from_value(ct.columns.clone()).unwrap_or_default();
            let filters: Vec<crate::services::report::ReportFilter> = ct
                .filters
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let parameters: Vec<crate::services::report::ReportParameter> = ct
                .parameters
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let formats: Vec<String> = ct
                .supported_formats
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            Ok(ReportTemplate {
                id: ct.template_id.clone().unwrap_or(ct.code.clone()),
                name: ct.name.clone(),
                description: ct.description.clone().unwrap_or_default(),
                category: ct.category.clone().unwrap_or_default(),
                data_source: ct.data_source.clone().unwrap_or_default(),
                report_type: ct.report_type.clone(),
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

    /// 创建用户自定义报表模板
    ///
    /// v11 批次 154 P2-A：接入 CreateTemplateRequest，将自定义模板写入 report_templates 表
    pub async fn create_custom_template(
        &self,
        user_id: i32,
        req: super::CreateTemplateRequest,
    ) -> Result<ReportTemplate, AppError> {
        use chrono::Utc;
        use sea_orm::Set;

        // 生成唯一 code：custom_{user_id}_{timestamp}
        let code = format!("custom_{}_{}", user_id, Utc::now().timestamp());
        let template_id = format!("custom_{}", &code);

        // 序列化 JSON 字段
        let columns_json = serde_json::to_value(&req.columns)
            .map_err(|e| AppError::internal(format!("序列化列定义失败: {}", e)))?;
        let filters_json = serde_json::to_value(&req.filters)
            .map_err(|e| AppError::internal(format!("序列化筛选条件失败: {}", e)))?;
        let parameters_json = serde_json::to_value(&req.parameters)
            .map_err(|e| AppError::internal(format!("序列化参数失败: {}", e)))?;
        let formats_json = serde_json::to_value(&req.supported_formats)
            .map_err(|e| AppError::internal(format!("序列化导出格式失败: {}", e)))?;

        let now = Utc::now();
        let active_model = report_template::ActiveModel {
            id: Default::default(),
            template_id: Set(Some(template_id.clone())),
            name: Set(req.name.clone()),
            code: Set(code.clone()),
            report_type: Set(req.report_type.unwrap_or_else(|| "custom".to_string())),
            category: Set(Some(req.category.clone())),
            data_source: Set(Some(req.data_source.clone())),
            columns: Set(columns_json),
            filters: Set(Some(filters_json)),
            parameters: Set(Some(parameters_json)),
            supported_formats: Set(Some(formats_json)),
            sort_by: Set(None),
            sort_order: Set(Some("asc".to_string())),
            data_source_sql: Set(None),
            description: Set(Some(req.description.clone())),
            is_public: Set(false),
            status: Set("ACTIVE".to_string()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;
        tracing::info!(
            template_id = model.id,
            code = %code,
            user_id = user_id,
            "自定义报表模板创建成功"
        );

        // 返回 ReportTemplate（与 get_template 返回格式一致）
        Ok(ReportTemplate {
            id: model.template_id.clone().unwrap_or(model.code.clone()),
            name: model.name,
            description: model.description.unwrap_or_default(),
            category: model.category.unwrap_or_default(),
            data_source: model.data_source.unwrap_or_default(),
            report_type: model.report_type,
            columns: req.columns,
            filters: req.filters,
            supported_formats: req.supported_formats,
            parameters: req.parameters,
        })
    }
}

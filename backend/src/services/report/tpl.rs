//! 报表模板服务（report/tpl）
//!
//! 包含报表模板的查询与管理：
//! - `get_predefined_templates` 返回 9 个内置预定义模板
//! - `create_custom_template` 创建用户自定义模板
//! - `get_all_templates` 合并预定义 + 自定义
//! - `get_template` 按 ID 获取单个模板
//!
//! 拆分自原 `report_engine_service.rs` 的"报表模板管理"段。

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use crate::models::report_template::{self, Entity as ReportTemplateEntity};
use crate::utils::error::AppError;

use super::{ReportEngineService, ReportTemplate};

impl ReportEngineService {
    /// 获取预定义报表模板
    pub fn get_predefined_templates(&self) -> Vec<ReportTemplate> {
        vec![
            Self::build_sales_summary_template(),
            Self::build_sales_detail_template(),
            Self::build_inventory_status_template(),
            Self::build_purchase_summary_template(),
            Self::build_ar_aging_template(),
            Self::build_top_products_template(),
            Self::build_customer_analysis_template(),
            Self::build_profit_analysis_template(),
            Self::build_inventory_turnover_template(),
        ]
    }

    /// 构建 ReportColumn（field_alias 恒为 None）
    fn make_col(
        key: &str,
        label: &str,
        data_type: &str,
        format: Option<&str>,
        aggregation: Option<&str>,
        sortable: bool,
        filterable: bool,
        width: Option<i32>,
        alignment: Option<&str>,
    ) -> super::ReportColumn {
        super::ReportColumn {
            field_alias: None,
            key: key.to_string(),
            label: label.to_string(),
            data_type: data_type.to_string(),
            format: format.map(|s| s.to_string()),
            aggregation: aggregation.map(|s| s.to_string()),
            sortable,
            filterable,
            width,
            alignment: alignment.map(|s| s.to_string()),
        }
    }

    /// 构建 ReportFilter（field_alias/operator/value/default_value/options 恒为 None）
    fn make_filter(key: &str, label: &str, filter_type: &str, required: bool) -> super::ReportFilter {
        super::ReportFilter {
            field_alias: None,
            operator: None,
            value: None,
            key: key.to_string(),
            label: label.to_string(),
            filter_type: filter_type.to_string(),
            default_value: None,
            options: None,
            required,
        }
    }

    /// 构建销售汇总报表模板
    fn build_sales_summary_template() -> ReportTemplate {
        use super::ReportParameter as Rp;
        ReportTemplate {
            id: "sales_summary".to_string(),
            name: "销售汇总报表".to_string(),
            description: "按时间段统计销售总额、订单数、客户数等汇总数据".to_string(),
            category: "sales".to_string(),
            data_source: "sales".to_string(),
            report_type: "sales".to_string(),
            columns: vec![
                Self::make_col("period", "期间", "string", None, Some("group"), true, true, Some(120), Some("left")),
                Self::make_col("total_amount", "销售总额", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(150), Some("right")),
                Self::make_col("order_count", "订单数", "integer", Some("#,##0"), Some("count"), true, false, Some(100), Some("right")),
                Self::make_col("customer_count", "客户数", "integer", Some("#,##0"), Some("count"), true, false, Some(100), Some("right")),
            ],
            filters: vec![Self::make_filter("date_range", "统计期间", "date_range", true)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string(), "csv".to_string()],
            parameters: vec![Rp {
                name: "group_by".to_string(),
                param_type: "string".to_string(),
                required: false,
                default_value: Some(serde_json::Value::String("day".to_string())),
                description: Some("分组方式: day, week, month, year".to_string()),
            }],
        }
    }

    /// 构建销售明细报表模板
    fn build_sales_detail_template() -> ReportTemplate {
        ReportTemplate {
            id: "sales_detail".to_string(),
            name: "销售明细报表".to_string(),
            description: "列出每笔销售订单的详细信息".to_string(),
            category: "sales".to_string(),
            data_source: "sales".to_string(),
            report_type: "sales".to_string(),
            columns: vec![
                Self::make_col("order_no", "订单号", "string", None, None, true, true, Some(150), Some("left")),
                Self::make_col("customer_name", "客户", "string", None, None, true, true, Some(150), Some("left")),
                Self::make_col("product_name", "产品", "string", None, None, true, true, Some(150), Some("left")),
                Self::make_col("quantity", "数量", "decimal", Some("#,##0.00"), None, true, false, Some(100), Some("right")),
                Self::make_col("amount", "金额", "decimal", Some("#,##0.00"), None, true, false, Some(120), Some("right")),
            ],
            filters: vec![
                Self::make_filter("date_range", "订单日期", "date_range", true),
                Self::make_filter("customer_id", "客户", "select", false),
            ],
            supported_formats: vec!["excel".to_string(), "pdf".to_string(), "csv".to_string(), "json".to_string()],
            parameters: vec![],
        }
    }

    /// 构建库存状态报表模板
    fn build_inventory_status_template() -> ReportTemplate {
        ReportTemplate {
            id: "inventory_status".to_string(),
            name: "库存状态报表".to_string(),
            description: "查询各仓库各产品的库存状态，包括在库、可用、预留等数量".to_string(),
            category: "inventory".to_string(),
            data_source: "inventory".to_string(),
            report_type: "inventory".to_string(),
            columns: vec![
                Self::make_col("warehouse_name", "仓库", "string", None, None, true, true, Some(150), Some("left")),
                Self::make_col("product_code", "产品编码", "string", None, None, true, true, Some(120), Some("left")),
                Self::make_col("product_name", "产品名称", "string", None, None, true, true, Some(150), Some("left")),
                Self::make_col("quantity_on_hand", "在库数量", "decimal", Some("#,##0.00"), None, true, false, Some(120), Some("right")),
                Self::make_col("quantity_available", "可用数量", "decimal", Some("#,##0.00"), None, true, false, Some(120), Some("right")),
            ],
            filters: vec![Self::make_filter("warehouse_id", "仓库", "select", false)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![],
        }
    }

    /// 构建采购汇总报表模板
    fn build_purchase_summary_template() -> ReportTemplate {
        ReportTemplate {
            id: "purchase_summary".to_string(),
            name: "采购汇总报表".to_string(),
            description: "按供应商和时间段统计采购总额、订单数".to_string(),
            category: "purchase".to_string(),
            data_source: "purchase".to_string(),
            report_type: "purchase".to_string(),
            columns: vec![
                Self::make_col("supplier_name", "供应商", "string", None, Some("group"), true, true, Some(150), Some("left")),
                Self::make_col("total_amount", "采购总额", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(150), Some("right")),
                Self::make_col("order_count", "订单数", "integer", Some("#,##0"), Some("count"), true, false, Some(100), Some("right")),
            ],
            filters: vec![Self::make_filter("date_range", "采购期间", "date_range", true)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![],
        }
    }

    /// 构建应收账款账龄分析报表模板
    fn build_ar_aging_template() -> ReportTemplate {
        ReportTemplate {
            id: "ar_aging".to_string(),
            name: "应收账款账龄分析".to_string(),
            description: "按客户和账龄段分析应收账款分布".to_string(),
            category: "finance".to_string(),
            data_source: "ar_aging".to_string(),
            report_type: "ar_aging".to_string(),
            columns: vec![
                Self::make_col("customer_name", "客户", "string", None, Some("group"), true, true, Some(150), Some("left")),
                Self::make_col("current", "当期", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("1_30_days", "1-30天", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("31_60_days", "31-60天", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("over_60_days", "60天以上", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
            ],
            filters: vec![],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![],
        }
    }

    /// 构建畅销产品报表模板
    fn build_top_products_template() -> ReportTemplate {
        use super::ReportParameter as Rp;
        ReportTemplate {
            id: "top_products".to_string(),
            name: "畅销产品报表".to_string(),
            description: "按销量或销售额统计TOP N产品".to_string(),
            category: "sales".to_string(),
            data_source: "sales".to_string(),
            report_type: "sales".to_string(),
            columns: vec![
                Self::make_col("product_code", "产品编码", "string", None, None, true, false, Some(120), Some("left")),
                Self::make_col("product_name", "产品名称", "string", None, None, true, false, Some(150), Some("left")),
                Self::make_col("total_quantity", "销售数量", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("total_amount", "销售金额", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(150), Some("right")),
            ],
            filters: vec![Self::make_filter("date_range", "统计期间", "date_range", true)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![Rp {
                name: "top_n".to_string(),
                param_type: "integer".to_string(),
                required: false,
                default_value: Some(serde_json::json!(20)),
                description: Some("TOP N 数量".to_string()),
            }],
        }
    }

    /// 构建客户分析报表模板
    fn build_customer_analysis_template() -> ReportTemplate {
        ReportTemplate {
            id: "customer_analysis".to_string(),
            name: "客户分析报表".to_string(),
            description: "按客户分析销售额、订单数、客单价等".to_string(),
            category: "sales".to_string(),
            data_source: "sales".to_string(),
            report_type: "sales".to_string(),
            columns: vec![
                Self::make_col("customer_name", "客户", "string", None, Some("group"), true, true, Some(150), Some("left")),
                Self::make_col("order_count", "订单数", "integer", Some("#,##0"), Some("count"), true, false, Some(100), Some("right")),
                Self::make_col("total_amount", "销售总额", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(150), Some("right")),
                Self::make_col("avg_order_amount", "客单价", "decimal", Some("#,##0.00"), Some("avg"), true, false, Some(120), Some("right")),
            ],
            filters: vec![Self::make_filter("date_range", "统计期间", "date_range", true)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![],
        }
    }

    /// 构建利润分析报表模板
    fn build_profit_analysis_template() -> ReportTemplate {
        ReportTemplate {
            id: "profit_analysis".to_string(),
            name: "利润分析报表".to_string(),
            description: "按产品/客户/期间分析销售收入、成本和毛利".to_string(),
            category: "finance".to_string(),
            data_source: "sales".to_string(),
            report_type: "sales".to_string(),
            columns: vec![
                Self::make_col("product_name", "产品", "string", None, Some("group"), true, true, Some(150), Some("left")),
                Self::make_col("revenue", "收入", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("cost", "成本", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("profit", "毛利", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("profit_margin", "毛利率", "decimal", Some("0.00%"), None, true, false, Some(100), Some("right")),
            ],
            filters: vec![Self::make_filter("date_range", "统计期间", "date_range", true)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![],
        }
    }

    /// 构建库存周转率报表模板
    fn build_inventory_turnover_template() -> ReportTemplate {
        ReportTemplate {
            id: "inventory_turnover".to_string(),
            name: "库存周转率报表".to_string(),
            description: "按产品/仓库分析库存周转率".to_string(),
            category: "inventory".to_string(),
            data_source: "inventory".to_string(),
            report_type: "inventory".to_string(),
            columns: vec![
                Self::make_col("product_name", "产品", "string", None, Some("group"), true, true, Some(150), Some("left")),
                Self::make_col("avg_stock", "平均库存", "decimal", Some("#,##0.00"), Some("avg"), true, false, Some(120), Some("right")),
                Self::make_col("outbound", "出库量", "decimal", Some("#,##0.00"), Some("sum"), true, false, Some(120), Some("right")),
                Self::make_col("turnover_rate", "周转率", "decimal", Some("0.00"), None, true, false, Some(100), Some("right")),
            ],
            filters: vec![Self::make_filter("date_range", "统计期间", "date_range", true)],
            supported_formats: vec!["excel".to_string(), "pdf".to_string()],
            parameters: vec![],
        }
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

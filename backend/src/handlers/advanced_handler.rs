use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::product::Entity as ProductEntity;
use crate::models::purchase_contract::Entity as PurchaseContractEntity;
use crate::models::purchase_price::Entity as PurchasePriceEntity;
use crate::models::sales_contract::Entity as SalesContractEntity;
use crate::models::sales_price::Entity as SalesPriceEntity;
use crate::models::sales_return::Entity as SalesReturnEntity;
use crate::models::tenant::Entity as TenantEntity;
use crate::services::ai_analysis_service::AiAnalysisService;
use crate::services::report_engine_service::{ExportFormat, ReportEngineService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use sea_orm::DatabaseConnection;

// ============================================================================
// 销售预测 - 使用真实时间序列算法（移动平均 + 指数平滑）
// ============================================================================

/// 销售预测
pub async fn sales_forecast(
    State(state): State<AppState>,
    Json(payload): Json<SalesForecastRequest>,
) -> impl IntoResponse {
    let service = AiAnalysisService::new(state.db);

    let days = match payload.period.as_str() {
        "3m" => 90,
        "6m" => 180,
        "12m" => 365,
        _ => 30,
    };

    let product_id = payload.product_id.unwrap_or(0) as i32;

    match service.forecast_sales(product_id, days).await {
        Ok(forecasts) => {
            let total_amount: f64 = forecasts
                .iter()
                .map(|f| f.predicted_quantity.to_f64().unwrap_or(0.0))
                .sum();
            let order_count = forecasts.len() as u32;
            let avg_confidence = if forecasts.is_empty() {
                0.0
            } else {
                forecasts.iter().map(|f| f.confidence).sum::<f64>() / forecasts.len() as f64
            };
            let trend = if forecasts.is_empty() {
                "无数据".to_string()
            } else {
                forecasts[0].trend.clone()
            };

            let response = SalesForecastResponse {
                sales_amount: total_amount,
                order_count,
                confidence: (avg_confidence * 100.0).round() as u32,
                trend,
                period: payload.period,
                detail: forecasts
                    .into_iter()
                    .take(30)
                    .map(|f| ForecastDetail {
                        date: f.forecast_date.to_string(),
                        predicted_quantity: f.predicted_quantity.to_string(),
                        confidence: f.confidence,
                        trend: f.trend,
                    })
                    .collect(),
            };

            Json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("销售预测失败: {}", e);
            Json(ApiResponse::error(format!("销售预测失败: {}", e)))
        }
    }
}

// ============================================================================
// 库存优化 - 基于历史出库数据
// ============================================================================

/// 库存优化建议
pub async fn inventory_optimization(
    State(state): State<AppState>,
    payload: Option<Json<InventoryOptimizationRequest>>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let service = AiAnalysisService::new(state.db);

    let product_id = payload.as_ref().and_then(|p| p.product_id.map(|pid| pid as i32));

    match service.optimize_inventory(product_id).await {
        Ok(suggestions) => {
            let high_count = suggestions
                .iter()
                .filter(|s| {
                    let current = s.current_stock.to_f64().unwrap_or(0.0);
                    current < s.reorder_point.to_f64().unwrap_or(0.0)
                })
                .count();
            let overstock_count = suggestions
                .iter()
                .filter(|s| {
                    let current = s.current_stock.to_f64().unwrap_or(0.0);
                    let suggested = s.suggested_stock.to_f64().unwrap_or(0.0);
                    current > suggested * 2.0
                })
                .count();

            let mut items: Vec<InventorySuggestion> = Vec::new();
            for s in suggestions {
                let current = s.current_stock.to_f64().unwrap_or(0.0);
                let reorder_point = s.reorder_point.to_f64().unwrap_or(0.0);
                let suggested = s.suggested_stock.to_f64().unwrap_or(0.0);

                if current >= reorder_point && current <= suggested * 2.0 && current > 0.0 {
                    continue;
                }

                let priority = if current <= 0.0 || current < reorder_point * 0.5 {
                    "high"
                } else if current < reorder_point {
                    "medium"
                } else {
                    "low"
                };

                let product_name = match get_product_name(&db, s.product_id).await {
                    Ok(name) => name,
                    Err(_) => format!("产品 {}", s.product_id),
                };

                items.push(InventorySuggestion {
                    product_name,
                    suggestion: format!(
                        "{} (当前库存: {:.0}, 再订货点: {:.0}, 建议订货: {:.0})",
                        s.reason, current, reorder_point, s.reorder_quantity.to_f64().unwrap_or(0.0)
                    ),
                    priority: priority.to_string(),
                });
            }

            let summary = format!(
                "检测到 {} 个产品需要补货，{} 个产品库存积压",
                high_count, overstock_count
            );

            let response = InventoryOptimizationResponse { summary, items };
            Json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("库存优化失败: {}", e);
            Json(ApiResponse::error(format!("库存优化失败: {}", e)))
        }
    }
}

// ============================================================================
// 异常检测 - 使用统计方法（Z-score + IQR）
// ============================================================================

/// 异常检测
pub async fn anomaly_detection(
    State(state): State<AppState>,
    Json(payload): Json<AnomalyDetectionRequest>,
) -> impl IntoResponse {
    let service = AiAnalysisService::new(state.db);

    let days = payload
        .date_range
        .as_ref()
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(30);

    match service.detect_anomalies(days).await {
        Ok(anomalies) => {
            let filtered = match payload.data_type.as_str() {
                "sales" => anomalies
                    .into_iter()
                    .filter(|a| a.entity_type == "SALES")
                    .collect::<Vec<_>>(),
                "inventory" => anomalies
                    .into_iter()
                    .filter(|a| a.entity_type == "INVENTORY")
                    .collect::<Vec<_>>(),
                "quality" => anomalies,
                _ => anomalies,
            };

            let items: Vec<AnomalyItem> = filtered
                .into_iter()
                .map(|a| {
                    let severity = match a.severity.as_str() {
                        "CRITICAL" => "critical",
                        "WARNING" => "warning",
                        "MEDIUM" => "warning",
                        _ => "info",
                    };

                    let anomaly_type = match a.anomaly_type.as_str() {
                        "SPIKE" => "突增",
                        "DROP" => "突降",
                        "ZERO_STOCK" => "零库存",
                        "LOW_STOCK" => "低于安全线",
                        "OVERSTOCK" => "库存积压",
                        "SLOW_MOVING" => "滞销",
                        other => other,
                    };

                    AnomalyItem {
                        item: format!("{} #{}", a.entity_type, a.entity_id),
                        anomaly_type: anomaly_type.to_string(),
                        description: a.description,
                        severity: severity.to_string(),
                        detected_at: a.detected_at.to_rfc3339(),
                    }
                })
                .collect();

            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("异常检测失败: {}", e);
            Json(ApiResponse::error(format!("异常检测失败: {}", e)))
        }
    }
}

// ============================================================================
// 智能推荐 - 关联规则推荐
// ============================================================================

/// 智能推荐
pub async fn recommendations(
    State(state): State<AppState>,
    payload: Option<Json<RecommendationRequest>>,
) -> impl IntoResponse {
    let service = AiAnalysisService::new(state.db);

    let rec_type = payload
        .as_ref()
        .and_then(|p| p.recommendation_type.clone())
        .unwrap_or_else(|| "all".to_string());
    let limit = payload.as_ref().and_then(|p| p.limit).unwrap_or(10);

    match service.generate_recommendations(rec_type, limit).await {
        Ok(recs) => {
            let items: Vec<Recommendation> = recs
                .into_iter()
                .map(|r| {
                    let rec_type_label = match r.recommendation_type.as_str() {
                        "REORDER" => "补货建议",
                        "BUNDLE" => "关联推荐",
                        "TREND" => "趋势推荐",
                        "PRICE_ADJUST" => "价格调整",
                        _ => "综合推荐",
                    };

                    Recommendation {
                        content: r.reason,
                        recommendation_type: rec_type_label.to_string(),
                        created_at: Utc::now().format("%Y-%m-%d %H:%M").to_string(),
                        score: r.score,
                        target_id: r.target_id,
                    }
                })
                .collect();

            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("生成推荐失败: {}", e);
            Json(ApiResponse::error(format!("生成推荐失败: {}", e)))
        }
    }
}

// ============================================================================
// 报表相关端点 - 使用真实报表引擎
// ============================================================================

/// 报表模板列表
pub async fn list_report_templates() -> impl IntoResponse {
    let templates = ReportEngineService::get_predefined_templates();

    let items: Vec<ReportTemplate> = templates
        .into_iter()
        .map(|t| {
            let category = match t.report_type {
                crate::services::report_engine_service::ReportType::Sales => "销售",
                crate::services::report_engine_service::ReportType::Purchase => "采购",
                crate::services::report_engine_service::ReportType::Inventory => "库存",
                crate::services::report_engine_service::ReportType::Financial => "财务",
                crate::services::report_engine_service::ReportType::Custom => "自定义",
            };

            ReportTemplate {
                template_name: t.name,
                template_code: t.id,
                category: category.to_string(),
                description: format!(
                    "包含 {} 个列: {}",
                    t.columns.len(),
                    t.columns.iter().map(|c| c.title.as_str()).collect::<Vec<_>>().join(", ")
                ),
                created_at: Utc::now().format("%Y-%m-%d").to_string(),
            }
        })
        .collect();

    Json(ApiResponse::success(items))
}

/// 执行报表
pub async fn execute_report(
    State(state): State<AppState>,
    Json(payload): Json<ReportExecuteRequest>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db);

    match service
        .execute_report(&payload.template_code, Vec::new(), 1, 100)
        .await
    {
        Ok(report_data) => {
            let columns = report_data.columns;
            let rows: Vec<HashMap<String, String>> = report_data
                .rows
                .into_iter()
                .map(|row| {
                    let mut map = HashMap::new();
                    for (i, col) in columns.iter().enumerate() {
                        if let Some(val) = row.get(i) {
                            map.insert(col.clone(), val.clone());
                        }
                    }
                    map
                })
                .collect();

            Json(ApiResponse::success(serde_json::json!({
                "columns": columns,
                "data": rows,
                "total_count": report_data.total_count,
            })))
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Json(ApiResponse::error(format!("执行报表失败: {}", e)))
        }
    }
}

/// 导出报表
pub async fn export_report(
    State(state): State<AppState>,
    Json(payload): Json<ReportExportRequest>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db);

    match service
        .execute_report(&payload.template_code, Vec::new(), 1, 1000)
        .await
    {
        Ok(report_data) => {
            let format = match payload.format.as_str() {
                "csv" => ExportFormat::CSV,
                "excel" | "xlsx" => ExportFormat::Excel,
                "pdf" => ExportFormat::PDF,
                _ => ExportFormat::JSON,
            };

            match service.export_report(&report_data, format) {
                Ok(bytes) => {
                    let size_kb = bytes.len() / 1024;
                    Json(ApiResponse::success(serde_json::json!({
                        "status": "success",
                        "format": payload.format,
                        "size_bytes": bytes.len(),
                        "size_kb": size_kb,
                        "record_count": report_data.total_count,
                    })))
                }
                Err(e) => {
                    tracing::error!("导出报表失败: {}", e);
                    Json(ApiResponse::error(format!("导出报表失败: {}", e)))
                }
            }
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Json(ApiResponse::error(format!("执行报表失败: {}", e)))
        }
    }
}

// ============================================================================
// 采购合同相关 - 使用真实数据库查询
// ============================================================================

/// 采购合同列表
pub async fn list_purchase_contracts(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match PurchaseContractEntity::find()
        .order_by_desc(crate::models::purchase_contract::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(contracts) => {
            let items: Vec<PurchaseContract> = contracts
                .into_iter()
                .map(|c| PurchaseContract {
                    id: c.id as u32,
                    contract_no: c.contract_no,
                    supplier_name: c.supplier_name.unwrap_or_else(|| format!("供应商 #{}", c.supplier_id)),
                    contract_date: c.signed_date.map_or_else(|| "".to_string(), |d| d.to_string()),
                    total_amount: c.total_amount.and_then(|d| d.to_f64()).unwrap_or(0.0),
                    status: c.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询采购合同失败: {}", e);
            Json(ApiResponse::error(format!("查询采购合同失败: {}", e)))
        }
    }
}

/// 创建采购合同
pub async fn create_purchase_contract(
    State(state): State<AppState>,
    Json(payload): Json<CreatePurchaseContractRequest>,
) -> impl IntoResponse {
    use crate::models::purchase_contract::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();
    let contract_no = format!("PC{}", now.format("%Y%m%d%H%M%S"));

    let active = ActiveModel {
        contract_no: sea_orm::ActiveValue::Set(contract_no.clone()),
        contract_name: sea_orm::ActiveValue::Set(payload.contract_name.clone()),
        contract_type: sea_orm::ActiveValue::Set(None),
        supplier_id: sea_orm::ActiveValue::Set(payload.supplier_id),
        supplier_name: sea_orm::ActiveValue::Set(None),
        total_amount: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.total_amount).ok(),
        ),
        signed_date: sea_orm::ActiveValue::Set(Some(now.naive_utc().date())),
        effective_date: sea_orm::ActiveValue::Set(None),
        expiry_date: sea_orm::ActiveValue::Set(None),
        payment_terms: sea_orm::ActiveValue::Set(None),
        payment_method: sea_orm::ActiveValue::Set(None),
        delivery_date: sea_orm::ActiveValue::Set(None),
        delivery_location: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("pending".to_string()),
        created_by: sea_orm::ActiveValue::Set(0),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(PurchaseContract {
            id: inserted.id as u32,
            contract_no: inserted.contract_no,
            supplier_name: inserted.supplier_name.unwrap_or_else(|| format!("供应商 #{}", inserted.supplier_id)),
            contract_date: inserted.signed_date.map_or_else(|| "".to_string(), |d| d.to_string()),
            total_amount: inserted.total_amount.and_then(|d| d.to_f64()).unwrap_or(0.0),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建采购合同失败: {}", e);
            Json(ApiResponse::error(format!("创建采购合同失败: {}", e)))
        }
    }
}

/// 获取采购合同
pub async fn get_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID".to_string()))
}

/// 更新采购合同
pub async fn update_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID和更新数据".to_string()))
}

/// 删除采购合同
pub async fn delete_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID".to_string()))
}

/// 审批采购合同
pub async fn approve_purchase_contract(
    State(state): State<AppState>,
) -> impl IntoResponse {
    use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter};

    match PurchaseContractEntity::find()
        .filter(crate::models::purchase_contract::Column::Status.eq("pending"))
        .one(&*state.db)
        .await
    {
        Ok(Some(contract)) => {
            let mut active: crate::models::purchase_contract::ActiveModel = contract.into();
            active.status = sea_orm::ActiveValue::Set("approved".to_string());
            active.updated_at = sea_orm::ActiveValue::Set(Utc::now());

            match active.update(&*state.db).await {
                Ok(updated) => Json(ApiResponse::<String>::success(format!(
                    "合同 {} 已审批",
                    updated.contract_no
                ))),
                Err(e) => Json(ApiResponse::<String>::error(format!("审批失败: {}", e))),
            }
        }
        Ok(None) => Json(ApiResponse::<String>::error("没有待审批的合同".to_string())),
        Err(e) => Json(ApiResponse::<String>::error(format!("查询失败: {}", e))),
    }
}

/// 执行采购合同
pub async fn execute_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID".to_string()))
}

// ============================================================================
// 采购价格相关 - 使用真实数据库查询
// ============================================================================

/// 采购价格列表
pub async fn list_purchase_prices(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match PurchasePriceEntity::find()
        .order_by_desc(crate::models::purchase_price::Column::EffectiveDate)
        .all(&*state.db)
        .await
    {
        Ok(prices) => {
            let items: Vec<PurchasePrice> = prices
                .into_iter()
                .map(|p| PurchasePrice {
                    id: p.id as u32,
                    product_name: format!("产品 #{}", p.product_id),
                    supplier_name: format!("供应商 #{}", p.supplier_id),
                    price: p.price.to_f64().unwrap_or(0.0),
                    currency: p.currency,
                    unit: p.unit,
                    effective_date: p.effective_date.to_string(),
                    expiry_date: p.expiry_date.map_or_else(|| "".to_string(), |d| d.to_string()),
                    status: p.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询采购价格失败: {}", e);
            Json(ApiResponse::error(format!("查询采购价格失败: {}", e)))
        }
    }
}

/// 创建采购价格
pub async fn create_purchase_price(
    State(state): State<AppState>,
    Json(payload): Json<CreatePurchasePriceRequest>,
) -> impl IntoResponse {
    use crate::models::purchase_price::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();

    let active = ActiveModel {
        product_id: sea_orm::ActiveValue::Set(payload.product_id),
        supplier_id: sea_orm::ActiveValue::Set(payload.supplier_id),
        price: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.price).unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        currency: sea_orm::ActiveValue::Set(payload.currency),
        unit: sea_orm::ActiveValue::Set(payload.unit),
        min_order_qty: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ONE),
        price_type: sea_orm::ActiveValue::Set("fixed".to_string()),
        effective_date: sea_orm::ActiveValue::Set(now.naive_utc().date()),
        expiry_date: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("active".to_string()),
        approved_by: sea_orm::ActiveValue::Set(None),
        approved_at: sea_orm::ActiveValue::Set(None),
        created_by: sea_orm::ActiveValue::Set(None),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(PurchasePrice {
            id: inserted.id as u32,
            product_name: format!("产品 #{}", inserted.product_id),
            supplier_name: format!("供应商 #{}", inserted.supplier_id),
            price: inserted.price.to_f64().unwrap_or(0.0),
            currency: inserted.currency,
            unit: inserted.unit,
            effective_date: inserted.effective_date.to_string(),
            expiry_date: inserted.expiry_date.map_or_else(|| "".to_string(), |d| d.to_string()),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建采购价格失败: {}", e);
            Json(ApiResponse::error(format!("创建采购价格失败: {}", e)))
        }
    }
}

/// 更新采购价格
pub async fn update_purchase_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供价格ID和更新数据".to_string()))
}

/// 删除采购价格
pub async fn delete_purchase_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供价格ID".to_string()))
}

/// 审批采购价格
pub async fn approve_purchase_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供价格ID".to_string()))
}

// ============================================================================
// 销售合同相关 - 使用真实数据库查询
// ============================================================================

/// 销售合同列表
pub async fn list_sales_contracts(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match SalesContractEntity::find()
        .order_by_desc(crate::models::sales_contract::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(contracts) => {
            let items: Vec<SalesContract> = contracts
                .into_iter()
                .map(|c| SalesContract {
                    id: c.id as u32,
                    contract_no: c.contract_no,
                    customer_name: c.customer_name.unwrap_or_else(|| format!("客户 #{}", c.customer_id)),
                    contract_date: c.signed_date.map_or_else(|| "".to_string(), |d| d.to_string()),
                    total_amount: c.total_amount.and_then(|d| d.to_f64()).unwrap_or(0.0),
                    status: c.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询销售合同失败: {}", e);
            Json(ApiResponse::error(format!("查询销售合同失败: {}", e)))
        }
    }
}

/// 创建销售合同
pub async fn create_sales_contract(
    State(state): State<AppState>,
    Json(payload): Json<CreateSalesContractRequest>,
) -> impl IntoResponse {
    use crate::models::sales_contract::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();
    let contract_no = format!("SC{}", now.format("%Y%m%d%H%M%S"));

    let active = ActiveModel {
        contract_no: sea_orm::ActiveValue::Set(contract_no.clone()),
        contract_name: sea_orm::ActiveValue::Set(payload.contract_name.clone()),
        contract_type: sea_orm::ActiveValue::Set(None),
        customer_id: sea_orm::ActiveValue::Set(payload.customer_id),
        customer_name: sea_orm::ActiveValue::Set(None),
        total_amount: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.total_amount).ok(),
        ),
        signed_date: sea_orm::ActiveValue::Set(Some(now.naive_utc().date())),
        effective_date: sea_orm::ActiveValue::Set(None),
        expiry_date: sea_orm::ActiveValue::Set(None),
        payment_terms: sea_orm::ActiveValue::Set(None),
        payment_method: sea_orm::ActiveValue::Set(None),
        delivery_date: sea_orm::ActiveValue::Set(None),
        delivery_location: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("pending".to_string()),
        created_by: sea_orm::ActiveValue::Set(0),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(SalesContract {
            id: inserted.id as u32,
            contract_no: inserted.contract_no,
            customer_name: inserted.customer_name.unwrap_or_else(|| format!("客户 #{}", inserted.customer_id)),
            contract_date: inserted.signed_date.map_or_else(|| "".to_string(), |d| d.to_string()),
            total_amount: inserted.total_amount.and_then(|d| d.to_f64()).unwrap_or(0.0),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建销售合同失败: {}", e);
            Json(ApiResponse::error(format!("创建销售合同失败: {}", e)))
        }
    }
}

/// 获取销售合同
pub async fn get_sales_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID".to_string()))
}

/// 更新销售合同
pub async fn update_sales_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID和更新数据".to_string()))
}

/// 删除销售合同
pub async fn delete_sales_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供合同ID".to_string()))
}

/// 审批销售合同
pub async fn approve_sales_contract(
    State(state): State<AppState>,
) -> impl IntoResponse {
    use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter};

    match SalesContractEntity::find()
        .filter(crate::models::sales_contract::Column::Status.eq("pending"))
        .one(&*state.db)
        .await
    {
        Ok(Some(contract)) => {
            let mut active: crate::models::sales_contract::ActiveModel = contract.into();
            active.status = sea_orm::ActiveValue::Set("approved".to_string());
            active.updated_at = sea_orm::ActiveValue::Set(Utc::now());

            match active.update(&*state.db).await {
                Ok(updated) => Json(ApiResponse::<String>::success(format!(
                    "合同 {} 已审批",
                    updated.contract_no
                ))),
                Err(e) => Json(ApiResponse::<String>::error(format!("审批失败: {}", e))),
            }
        }
        Ok(None) => Json(ApiResponse::<String>::error("没有待审批的合同".to_string())),
        Err(e) => Json(ApiResponse::<String>::error(format!("查询失败: {}", e))),
    }
}

// ============================================================================
// 销售价格相关 - 使用真实数据库查询
// ============================================================================

/// 销售价格列表
pub async fn list_sales_prices(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match SalesPriceEntity::find()
        .order_by_desc(crate::models::sales_price::Column::EffectiveDate)
        .all(&*state.db)
        .await
    {
        Ok(prices) => {
            let items: Vec<SalesPrice> = prices
                .into_iter()
                .map(|p| SalesPrice {
                    id: p.id as u32,
                    product_name: format!("产品 #{}", p.product_id),
                    customer_name: p.customer_id.map_or_else(|| "全部客户".to_string(), |cid| format!("客户 #{}", cid)),
                    price: p.price.to_f64().unwrap_or(0.0),
                    currency: p.currency,
                    unit: p.unit,
                    effective_date: p.effective_date.to_string(),
                    status: p.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询销售价格失败: {}", e);
            Json(ApiResponse::error(format!("查询销售价格失败: {}", e)))
        }
    }
}

/// 创建销售价格
pub async fn create_sales_price(
    State(state): State<AppState>,
    Json(payload): Json<CreateSalesPriceRequest>,
) -> impl IntoResponse {
    use crate::models::sales_price::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();

    let active = ActiveModel {
        product_id: sea_orm::ActiveValue::Set(payload.product_id),
        customer_id: sea_orm::ActiveValue::Set(payload.customer_id),
        customer_type: sea_orm::ActiveValue::Set(None),
        price: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.price).unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        currency: sea_orm::ActiveValue::Set(payload.currency),
        unit: sea_orm::ActiveValue::Set(payload.unit),
        min_order_qty: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ONE),
        price_type: sea_orm::ActiveValue::Set("fixed".to_string()),
        price_level: sea_orm::ActiveValue::Set(None),
        effective_date: sea_orm::ActiveValue::Set(now.naive_utc().date()),
        expiry_date: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("active".to_string()),
        approved_by: sea_orm::ActiveValue::Set(None),
        approved_at: sea_orm::ActiveValue::Set(None),
        created_by: sea_orm::ActiveValue::Set(None),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(SalesPrice {
            id: inserted.id as u32,
            product_name: format!("产品 #{}", inserted.product_id),
            customer_name: inserted.customer_id.map_or_else(|| "全部客户".to_string(), |cid| format!("客户 #{}", cid)),
            price: inserted.price.to_f64().unwrap_or(0.0),
            currency: inserted.currency,
            unit: inserted.unit,
            effective_date: inserted.effective_date.to_string(),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建销售价格失败: {}", e);
            Json(ApiResponse::error(format!("创建销售价格失败: {}", e)))
        }
    }
}

/// 更新销售价格
pub async fn update_sales_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供价格ID和更新数据".to_string()))
}

/// 删除销售价格
pub async fn delete_sales_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供价格ID".to_string()))
}

/// 审批销售价格
pub async fn approve_sales_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供价格ID".to_string()))
}

// ============================================================================
// 销售退货相关 - 使用真实数据库查询
// ============================================================================

/// 销售退货列表
pub async fn list_sales_returns(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match SalesReturnEntity::find()
        .order_by_desc(crate::models::sales_return::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(returns) => {
            let items: Vec<SalesReturn> = returns
                .into_iter()
                .map(|r| SalesReturn {
                    id: r.id as u32,
                    return_no: r.return_no,
                    customer_name: format!("客户 #{}", r.customer_id),
                    order_no: r.sales_order_id.map_or_else(|| "".to_string(), |oid| format!("SO{}", oid)),
                    return_date: r.return_date.to_string(),
                    total_amount: r.total_amount.to_f64().unwrap_or(0.0),
                    reason: r.reason,
                    status: r.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询销售退货失败: {}", e);
            Json(ApiResponse::error(format!("查询销售退货失败: {}", e)))
        }
    }
}

/// 创建销售退货
pub async fn create_sales_return(
    State(state): State<AppState>,
    Json(payload): Json<CreateSalesReturnRequest>,
) -> impl IntoResponse {
    use crate::models::sales_return::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();
    let return_no = format!("SR{}", now.format("%Y%m%d%H%M%S"));

    let active = ActiveModel {
        return_no: sea_orm::ActiveValue::Set(return_no.clone()),
        sales_order_id: sea_orm::ActiveValue::Set(Some(payload.order_id)),
        customer_id: sea_orm::ActiveValue::Set(payload.customer_id),
        return_date: sea_orm::ActiveValue::Set(now.naive_utc().date()),
        warehouse_id: sea_orm::ActiveValue::Set(1),
        reason: sea_orm::ActiveValue::Set(payload.reason.clone()),
        status: sea_orm::ActiveValue::Set("pending".to_string()),
        total_amount: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.total_amount).unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        remarks: sea_orm::ActiveValue::Set(None),
        approved_by: sea_orm::ActiveValue::Set(None),
        approved_at: sea_orm::ActiveValue::Set(None),
        rejected_reason: sea_orm::ActiveValue::Set(None),
        created_by: sea_orm::ActiveValue::Set(0),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(SalesReturn {
            id: inserted.id as u32,
            return_no: inserted.return_no,
            customer_name: format!("客户 #{}", inserted.customer_id),
            order_no: inserted.sales_order_id.map_or_else(|| "".to_string(), |oid| format!("SO{}", oid)),
            return_date: inserted.return_date.to_string(),
            total_amount: inserted.total_amount.to_f64().unwrap_or(0.0),
            reason: inserted.reason,
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建销售退货失败: {}", e);
            Json(ApiResponse::error(format!("创建销售退货失败: {}", e)))
        }
    }
}

/// 获取销售退货
pub async fn get_sales_return() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供退货ID".to_string()))
}

/// 更新销售退货
pub async fn update_sales_return() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供退货ID和更新数据".to_string()))
}

/// 删除销售退货
pub async fn delete_sales_return() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供退货ID".to_string()))
}

// ============================================================================
// 租户管理相关 - 使用真实数据库查询
// ============================================================================

/// 租户列表
pub async fn list_tenants(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match TenantEntity::find()
        .order_by_asc(crate::models::tenant::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(tenants) => {
            let items: Vec<Tenant> = tenants
                .into_iter()
                .map(|t| Tenant {
                    id: t.id as u32,
                    tenant_code: t.code,
                    tenant_name: t.name,
                    domain: t.custom_domain.unwrap_or_else(|| "".to_string()),
                    subscription_plan: t.plan_id.map_or_else(|| "free".to_string(), |pid| format!("plan_{}", pid)),
                    current_users: 0,
                    max_users: 0,
                    status: t.status,
                    subscription_start_date: "".to_string(),
                    subscription_end_date: t.expired_at.map_or_else(|| "".to_string(), |d| d.format("%Y-%m-%d").to_string()),
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询租户列表失败: {}", e);
            Json(ApiResponse::error(format!("查询租户列表失败: {}", e)))
        }
    }
}

/// 创建租户
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantRequest>,
) -> impl IntoResponse {
    use crate::models::tenant::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();

    let active = ActiveModel {
        code: sea_orm::ActiveValue::Set(payload.tenant_code.clone()),
        name: sea_orm::ActiveValue::Set(payload.tenant_name.clone()),
        description: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("active".to_string()),
        plan_id: sea_orm::ActiveValue::Set(None),
        admin_user_id: sea_orm::ActiveValue::Set(None),
        db_schema: sea_orm::ActiveValue::Set(None),
        custom_domain: sea_orm::ActiveValue::Set(Some(payload.domain.clone())),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        expired_at: sea_orm::ActiveValue::Set(Some(now + chrono::Duration::days(365))),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(Tenant {
            id: inserted.id as u32,
            tenant_code: inserted.code,
            tenant_name: inserted.name,
            domain: inserted.custom_domain.unwrap_or_else(|| "".to_string()),
            subscription_plan: inserted.plan_id.map_or_else(|| "free".to_string(), |pid| format!("plan_{}", pid)),
            current_users: 0,
            max_users: 0,
            status: inserted.status,
            subscription_start_date: "".to_string(),
            subscription_end_date: inserted.expired_at.map_or_else(|| "".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        })),
        Err(e) => {
            tracing::error!("创建租户失败: {}", e);
            Json(ApiResponse::error(format!("创建租户失败: {}", e)))
        }
    }
}

/// 获取租户
pub async fn get_tenant() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供租户ID".to_string()))
}

/// 更新租户
pub async fn update_tenant() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error("请提供租户ID和更新数据".to_string()))
}

// ============================================================================
// 辅助函数
// ============================================================================

async fn get_product_name(db: &Arc<DatabaseConnection>, product_id: i32) -> Result<String, AppError> {
    match ProductEntity::find_by_id(product_id).one(&**db).await {
        Ok(Some(product)) => Ok(product.name),
        Ok(None) => Ok(format!("产品 #{}", product_id)),
        Err(e) => Err(AppError::DatabaseError(e.to_string())),
    }
}

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastRequest {
    pub period: String,
    pub product_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastResponse {
    pub sales_amount: f64,
    pub order_count: u32,
    pub confidence: u32,
    pub trend: String,
    pub period: String,
    pub detail: Vec<ForecastDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForecastDetail {
    pub date: String,
    pub predicted_quantity: String,
    pub confidence: f64,
    pub trend: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryOptimizationRequest {
    pub warehouse_id: Option<u32>,
    pub product_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryOptimizationResponse {
    pub summary: String,
    pub items: Vec<InventorySuggestion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventorySuggestion {
    pub product_name: String,
    pub suggestion: String,
    pub priority: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyDetectionRequest {
    pub data_type: String,
    pub date_range: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyItem {
    pub item: String,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
    pub detected_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub recommendation_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub content: String,
    pub recommendation_type: String,
    pub created_at: String,
    pub score: f64,
    pub target_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePurchaseContractRequest {
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchaseContract {
    pub id: u32,
    pub contract_no: String,
    pub supplier_name: String,
    pub contract_date: String,
    pub total_amount: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePurchasePriceRequest {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: f64,
    pub currency: String,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchasePrice {
    pub id: u32,
    pub product_name: String,
    pub supplier_name: String,
    pub price: f64,
    pub currency: String,
    pub unit: String,
    pub effective_date: String,
    pub expiry_date: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSalesContractRequest {
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesContract {
    pub id: u32,
    pub contract_no: String,
    pub customer_name: String,
    pub contract_date: String,
    pub total_amount: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSalesPriceRequest {
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub price: f64,
    pub currency: String,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesPrice {
    pub id: u32,
    pub product_name: String,
    pub customer_name: String,
    pub price: f64,
    pub currency: String,
    pub unit: String,
    pub effective_date: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSalesReturnRequest {
    pub order_id: i32,
    pub customer_id: i32,
    pub total_amount: f64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesReturn {
    pub id: u32,
    pub return_no: String,
    pub customer_name: String,
    pub order_no: String,
    pub return_date: String,
    pub total_amount: f64,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub template_name: String,
    pub template_code: String,
    pub category: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportExecuteRequest {
    pub template_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportExportRequest {
    pub template_code: String,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_code: String,
    pub tenant_name: String,
    pub domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tenant {
    pub id: u32,
    pub tenant_code: String,
    pub tenant_name: String,
    pub domain: String,
    pub subscription_plan: String,
    pub current_users: u32,
    pub max_users: u32,
    pub status: String,
    pub subscription_start_date: String,
    pub subscription_end_date: String,
}

//! 通用打印 Handler

use crate::services::print_service::PrintService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    response::Html,
};

async fn render_print_html(doc_type: &str, doc_id: i32) -> Result<Html<String>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(doc_type, doc_id).await?;
    let html = service.generate_pdf(&print_data)?;
    Ok(Html(html))
}

pub async fn sales_order_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("sales_order", doc_id).await
}

pub async fn sales_contract_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("sales_contract", doc_id).await
}

pub async fn purchase_order_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("purchase_order", doc_id).await
}

pub async fn purchase_receipt_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("purchase_receipt", doc_id).await
}

pub async fn inventory_transfer_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("inventory_transfer", doc_id).await
}


/// 打印模板列表响应
#[derive(serde::Serialize)]
pub struct PrintTemplateDto {
    pub id: i32,
    pub name: String,
    pub doc_type: String,
    pub template_content: String,
    pub is_default: bool,
    pub created_at: String,
}

/// 批次 126 v8 复审 P2 修复：系统内置打印模板静态列表
///
/// 设计说明：打印模板为系统内置（对应 PrintService 支持的 6 种单据类型），
/// 不需要动态 CRUD 管理。模板内容字段为简短描述（实际渲染逻辑在 PrintService.generate_pdf）。
/// 若未来需支持用户自定义模板，可新增 print_templates 表 + model + service。
fn builtin_print_templates() -> Vec<PrintTemplateDto> {
    vec![
        PrintTemplateDto {
            id: 1,
            name: "销售订单打印模板".to_string(),
            doc_type: "sales_order".to_string(),
            template_content: "标准销售订单打印模板（含客户信息、订单明细、金额合计）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 2,
            name: "销售合同打印模板".to_string(),
            doc_type: "sales_contract".to_string(),
            template_content: "标准销售合同打印模板（含合同条款、双方信息、签章位置）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 3,
            name: "采购订单打印模板".to_string(),
            doc_type: "purchase_order".to_string(),
            template_content: "标准采购订单打印模板（含供应商信息、采购明细、金额合计）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 4,
            name: "采购收货单打印模板".to_string(),
            doc_type: "purchase_receipt".to_string(),
            template_content: "标准采购收货单打印模板（含收货明细、质检结果、入库确认）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 5,
            name: "库存调拨单打印模板".to_string(),
            doc_type: "inventory_transfer".to_string(),
            template_content: "标准库存调拨单打印模板（含调出/调入仓库、调拨明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 6,
            name: "会计凭证打印模板".to_string(),
            doc_type: "voucher".to_string(),
            template_content: "标准会计凭证打印模板（含科目分录、借贷金额、凭证摘要）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ]
}

/// 获取打印模板列表
///
/// 批次 126 v8 复审 P2 修复：从原空列表占位改为返回系统内置 6 种单据打印模板。
/// 模板对应 PrintService 支持的 6 种单据类型（sales_order/sales_contract/purchase_order/
/// purchase_receipt/inventory_transfer/voucher）。
pub async fn list_print_templates(
    State(_): State<AppState>,
) -> Result<axum::Json<ApiResponse<Vec<PrintTemplateDto>>>, AppError> {
    Ok(axum::Json(ApiResponse::success(builtin_print_templates())))
}

/// 获取单个打印模板详情
///
/// 批次 126 v8 复审 P2 修复：从原硬编码 not_found 改为从内置模板列表按 id 查找。
/// 找不到时返回 404 not_found。
pub async fn get_print_template(
    Path(id): Path<i32>,
    State(_): State<AppState>,
) -> Result<axum::Json<ApiResponse<PrintTemplateDto>>, AppError> {
    let template = builtin_print_templates()
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;
    Ok(axum::Json(ApiResponse::success(template)))
}

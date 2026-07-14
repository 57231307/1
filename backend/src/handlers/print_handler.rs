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

#[cfg(test)]
mod tests {
    //! 打印 Handler 单元测试（批次 394 补测）
    //!
    //! 覆盖目标：
    //! - builtin_print_templates 静态模板列表（5 个测试）

    use super::*;

    /// 测试_builtin_print_templates返回6个模板
    ///
    /// 验证内置打印模板数量为 6（对应 6 种单据类型）
    #[test]
    fn 测试_builtin_print_templates返回6个模板() {
        let templates = builtin_print_templates();
        assert_eq!(templates.len(), 6, "应有 6 个内置打印模板");
    }

    /// 测试_builtin_print_templates_id唯一且连续
    ///
    /// 验证 6 个模板的 id 为 1-6，唯一且连续
    #[test]
    fn 测试_builtin_print_templates_id唯一且连续() {
        let templates = builtin_print_templates();
        let ids: Vec<i32> = templates.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5, 6], "id 应为 1-6 连续");

        // 唯一性检查
        let unique_ids: std::collections::HashSet<i32> = ids.iter().copied().collect();
        assert_eq!(unique_ids.len(), 6, "id 应唯一");
    }

    /// 测试_builtin_print_templates_doc_type唯一
    ///
    /// 验证 6 个模板的 doc_type 互不相同
    #[test]
    fn 测试_builtin_print_templates_doc_type唯一() {
        let templates = builtin_print_templates();
        let doc_types: Vec<&str> = templates.iter().map(|t| t.doc_type.as_str()).collect();
        let unique: std::collections::HashSet<&str> = doc_types.iter().copied().collect();
        assert_eq!(unique.len(), 6, "doc_type 应唯一");
    }

    /// 测试_builtin_print_templates全部为默认模板
    ///
    /// 验证所有内置模板的 is_default 均为 true
    #[test]
    fn 测试_builtin_print_templates全部为默认模板() {
        let templates = builtin_print_templates();
        for t in &templates {
            assert!(t.is_default, "模板 {} 应为默认模板", t.name);
        }
    }

    /// 测试_builtin_print_templates覆盖6种单据类型
    ///
    /// 验证模板覆盖全部 6 种业务单据类型：
    /// sales_order / sales_contract / purchase_order / purchase_receipt / inventory_transfer / voucher
    #[test]
    fn 测试_builtin_print_templates覆盖6种单据类型() {
        let templates = builtin_print_templates();
        let doc_types: Vec<&str> = templates.iter().map(|t| t.doc_type.as_str()).collect();

        let expected = [
            "sales_order",
            "sales_contract",
            "purchase_order",
            "purchase_receipt",
            "inventory_transfer",
            "voucher",
        ];
        for t in &expected {
            assert!(
                doc_types.contains(&t),
                "应包含单据类型 {}",
                t
            );
        }

        // 名称不应为空
        for t in &templates {
            assert!(!t.name.is_empty(), "模板 {} 的名称不应为空", t.doc_type);
            assert!(
                !t.template_content.is_empty(),
                "模板 {} 的内容不应为空",
                t.doc_type
            );
        }
    }
}

//! 采购检验模型

use serde::{Deserialize, Serialize};

/// 采购检验单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseInspection {
    /// 检验单 ID
    pub id: i32,
    /// 检验单号
    pub inspection_no: String,
    /// 采购订单 ID
    pub purchase_order_id: Option<i32>,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 供应商名称
    pub supplier_name: Option<String>,
    /// 检验日期
    pub inspection_date: String,
    /// 质检员 ID
    pub inspector_id: Option<i32>,
    /// 质检员名称
    pub inspector_name: Option<String>,
    /// 检验结果
    pub result: String,
    /// 合格数量
    pub qualified_quantity: String,
    /// 不合格数量
    pub unqualified_quantity: String,
    /// 不合格原因
    pub unqualified_reason: Option<String>,
    /// 备注
    pub remarks: Option<String>,
    /// 创建人
    pub created_by: i32,
    /// 创建时间
    pub created_at: Option<String>,
    /// 更新时间
    pub updated_at: Option<String>,
}

/// 创建采购检验单请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseInspectionRequest {
    /// 入库单 ID
    pub receipt_id: i32,
    /// 采购订单 ID
    pub order_id: Option<i32>,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 检验日期
    pub inspection_date: String,
    /// 质检员 ID
    pub inspector_id: Option<i32>,
    /// 检验类型
    pub inspection_type: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 更新采购检验单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseInspectionRequest {
    /// 抽样数量
    pub sample_size: Option<i32>,
    /// 缺陷描述
    pub defect_description: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 完成检验请求
#[derive(Debug, Clone, Serialize)]
pub struct CompleteInspectionRequest {
    /// 合格数量
    pub pass_quantity: String,
    /// 不合格数量
    pub reject_quantity: String,
    /// 检验结果
    pub inspection_result: String,
}

/// 检验单查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseInspectionQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// 检验单列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct InspectionListResponse {
    pub items: Vec<PurchaseInspection>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

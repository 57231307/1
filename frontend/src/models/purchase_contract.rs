//! 采购合同模型

use serde::{Deserialize, Serialize};

/// 采购合同数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PurchaseContract {
    /// 合同ID
    pub id: i32,
    /// 合同编号
    pub contract_no: String,
    /// 合同名称
    pub contract_name: String,
    /// 供应商ID
    pub supplier_id: i32,
    /// 供应商名称（关联查询返回）
    pub supplier_name: Option<String>,
    /// 合同总金额
    pub total_amount: String,
    /// 付款条款
    pub payment_terms: Option<String>,
    /// 交货日期
    pub delivery_date: String,
    /// 合同状态
    pub status: String,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 创建用户ID
    pub created_by: Option<i32>,
}

/// 采购合同查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseContractQueryParams {
    /// 搜索关键词
    pub keyword: Option<String>,
    /// 合同状态
    pub status: Option<String>,
    /// 供应商ID
    pub supplier_id: Option<i32>,
    /// 页码
    pub page: Option<i64>,
    /// 每页数量
    pub page_size: Option<i64>,
}

impl Default for PurchaseContractQueryParams {
    fn default() -> Self {
        Self {
            keyword: None,
            status: None,
            supplier_id: None,
            page: Some(1),
            page_size: Some(10),
        }
    }
}

/// 创建采购合同请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseContractRequest {
    /// 合同编号
    pub contract_no: String,
    /// 合同名称
    pub contract_name: String,
    /// 供应商ID
    pub supplier_id: i32,
    /// 合同总金额
    pub total_amount: String,
    /// 付款条款
    pub payment_terms: Option<String>,
    /// 交货日期
    pub delivery_date: String,
    /// 备注
    pub remark: Option<String>,
}

/// 合同执行请求
#[derive(Debug, Clone, Serialize)]
pub struct ExecutePurchaseContractRequest {
    /// 执行类型
    pub execution_type: String,
    /// 执行金额
    pub execution_amount: String,
    /// 关联单据类型
    pub related_bill_type: Option<String>,
    /// 关联单据ID
    pub related_bill_id: Option<i32>,
    /// 备注
    pub remark: Option<String>,
}

/// 取消合同请求
#[derive(Debug, Clone, Serialize)]
pub struct CancelPurchaseContractRequest {
    /// 取消原因
    pub reason: String,
}

/// 采购合同列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct PurchaseContractListResponse {
    /// 合同列表
    pub items: Vec<PurchaseContract>,
    /// 总数
    pub total: i64,
    /// 页码
    pub page: u64,
    /// 每页数量
    pub page_size: u64,
}

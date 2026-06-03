use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 跟进记录创建请求
#[derive(Debug, Deserialize)]
pub struct FollowUpRequest {
    /// 跟进类型：phone/meeting/email/wechat/visit
    #[serde(default)]
    pub r#type: Option<String>,
    /// 跟进内容
    pub content: Option<String>,
    /// 下次跟进日期（YYYY-MM-DD）
    pub next_follow_date: Option<String>,
}

/// RFM 评分响应
#[derive(Debug, Serialize, Clone)]
pub struct RfmScoreResponse {
    /// 最近一次订单距今天数
    pub recency: i32,
    /// 订单频次
    pub frequency: i32,
    /// 累计消费金额
    pub monetary: Decimal,
    /// 客户等级：A/B/C/D/E
    pub level: char,
    /// 等级标签
    pub label: String,
}

/// 客户增强信息更新请求
#[derive(Debug, Deserialize, Default)]
pub struct UpdateCustomerEnhancedRequest {
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: Option<Decimal>,
    pub payment_terms: Option<i32>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub status: Option<String>,
    pub customer_type: Option<String>,
    pub notes: Option<String>,
    pub customer_industry: Option<String>,
    pub main_products: Option<String>,
    pub annual_purchase: Option<Decimal>,
    pub quality_requirement: Option<String>,
    pub inspection_standard: Option<String>,
}

/// 批量领取公海客户请求
#[derive(Debug, Deserialize)]
pub struct BatchClaimRequest {
    pub customer_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLeadRequest {
    pub lead_no: Option<String>,
    pub lead_source: Option<String>,
    pub lead_status: Option<String>,
    pub company_name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_title: Option<String>,
    pub mobile_phone: Option<String>,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub address: Option<String>,
    pub product_interest: Option<String>,
    pub estimated_quantity: Option<Decimal>,
    pub estimated_amount: Option<Decimal>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub requirement_desc: Option<String>,
    pub priority: Option<String>,
    pub rating: Option<i32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOpportunityRequest {
    pub opportunity_no: Option<String>,
    pub opportunity_name: String,
    pub customer_id: i32,
    pub lead_id: Option<i32>,
    pub opportunity_type: Option<String>,
    pub opportunity_stage: Option<String>,
    pub win_probability: Option<Decimal>,
    pub estimated_amount: Option<Decimal>,
    pub actual_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub expected_close_date: Option<NaiveDate>,
    pub actual_close_date: Option<NaiveDate>,
    pub product_ids: Option<Vec<i32>>,
    pub product_names: Option<Vec<String>>,
    pub product_desc: Option<String>,
    pub priority: Option<String>,
    pub rating: Option<i32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LeadQuery {
    pub lead_status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub opportunity_stage: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ConvertLeadRequest {
    pub customer_type: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateLeadRequest {
    pub lead_source: Option<String>,
    pub lead_status: Option<String>,
    pub company_name: Option<String>,
    pub contact_name: Option<String>,
    pub contact_title: Option<String>,
    pub mobile_phone: Option<String>,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub address: Option<String>,
    pub product_interest: Option<String>,
    pub estimated_quantity: Option<Decimal>,
    pub estimated_amount: Option<Decimal>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub requirement_desc: Option<String>,
    pub priority: Option<String>,
    pub rating: Option<i32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOpportunityRequest {
    pub opportunity_name: Option<String>,
    pub customer_id: Option<i32>,
    pub lead_id: Option<i32>,
    pub opportunity_type: Option<String>,
    pub opportunity_stage: Option<String>,
    pub win_probability: Option<Decimal>,
    pub estimated_amount: Option<Decimal>,
    pub actual_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub expected_close_date: Option<NaiveDate>,
    pub actual_close_date: Option<NaiveDate>,
    pub product_ids: Option<Vec<i32>>,
    pub product_names: Option<Vec<String>>,
    pub product_desc: Option<String>,
    pub priority: Option<String>,
    pub rating: Option<i32>,
    pub tags: Option<Vec<String>>,
}

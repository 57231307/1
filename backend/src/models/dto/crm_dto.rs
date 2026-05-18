use serde::Deserialize;
use rust_decimal::Decimal;
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct CreateLeadRequest {
    pub lead_no: Option<String>,
    pub lead_source: String,
    pub lead_status: Option<String>,
    pub company_name: Option<String>,
    pub contact_name: String,
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

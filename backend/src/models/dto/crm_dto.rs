use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateLeadRequest {
    pub name: String,
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub source: String,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOpportunityRequest {
    pub name: String,
    pub customer_id: Option<i32>,
    pub lead_id: Option<i32>,
    pub amount: Decimal,
    pub expected_close_date: Option<NaiveDate>,
    pub stage: String,
    pub source: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LeadQuery {
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub stage: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

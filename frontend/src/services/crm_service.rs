use serde::{Deserialize, Serialize};
use crate::services::api::ApiService;
use rust_decimal::Decimal;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CrmLead {
    pub id: i32,
    pub lead_no: String,
    pub name: String,
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub source: String,
    pub status: String,
    pub remarks: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CrmOpportunity {
    pub id: i32,
    pub opportunity_no: String,
    pub name: String,
    pub customer_id: Option<i32>,
    pub lead_id: Option<i32>,
    pub amount: Decimal,
    pub stage: String,
    pub source: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PageResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

pub struct CrmService;

impl CrmService {
    pub async fn list_leads(page: u64, page_size: u64) -> Result<PageResponse<CrmLead>, String> {
        ApiService::get(&format!("/crm/leads?page={}&page_size={}", page, page_size)).await
    }

    pub async fn list_opportunities(page: u64, page_size: u64) -> Result<PageResponse<CrmOpportunity>, String> {
        ApiService::get(&format!("/crm/opportunities?page={}&page_size={}", page, page_size)).await
    }
}

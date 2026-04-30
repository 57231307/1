use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: i32,
    pub customer_code: String,
    pub customer_name: String,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: Option<String>,
    pub payment_terms: Option<i32>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub customer_type: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_type: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerListResponse {
    pub data: Vec<Customer>,
    pub items: Vec<Customer>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateCustomerRequest {
    pub customer_code: String,
    pub customer_name: String,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: Option<String>,
    pub payment_terms: Option<i32>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub customer_type: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCustomerRequest {
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: Option<String>,
    pub payment_terms: Option<i32>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

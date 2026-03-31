//! 供应商管理模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    pub id: i32,
    pub supplier_name: String,
    pub supplier_short_name: String,
    pub supplier_type: String,
    pub credit_code: String,
    pub registered_address: String,
    pub business_address: Option<String>,
    pub legal_representative: String,
    pub registered_capital: String,
    pub establishment_date: String,
    pub business_term: Option<String>,
    pub business_scope: Option<String>,
    pub taxpayer_type: String,
    pub bank_name: String,
    pub bank_account: String,
    pub contact_phone: String,
    pub fax: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub main_business: Option<String>,
    pub main_market: Option<String>,
    pub employee_count: Option<i32>,
    pub annual_revenue: Option<String>,
    pub grade: Option<String>,
    pub grade_score: Option<String>,
    pub status: String,
    pub is_enabled: bool,
    pub remarks: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierContact {
    pub id: i32,
    pub supplier_id: i32,
    pub contact_name: String,
    pub department: Option<String>,
    pub position: Option<String>,
    pub mobile_phone: String,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub is_primary: bool,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierQualification {
    pub id: i32,
    pub supplier_id: i32,
    pub qualification_name: String,
    pub qualification_type: String,
    pub qualification_no: String,
    pub issuing_authority: String,
    pub issue_date: String,
    pub valid_until: String,
    pub attachment_path: Option<String>,
    pub need_annual_check: bool,
    pub annual_check_record: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupplierQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub supplier_type: Option<String>,
    pub grade: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSupplierRequest {
    pub supplier_name: String,
    pub supplier_short_name: String,
    pub supplier_type: String,
    pub credit_code: String,
    pub registered_address: String,
    pub business_address: Option<String>,
    pub legal_representative: String,
    pub registered_capital: String,
    pub establishment_date: String,
    pub business_term: Option<String>,
    pub business_scope: Option<String>,
    pub taxpayer_type: String,
    pub bank_name: String,
    pub bank_account: String,
    pub contact_phone: String,
    pub fax: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub main_business: Option<String>,
    pub main_market: Option<String>,
    pub employee_count: Option<i32>,
    pub annual_revenue: Option<String>,
    pub contacts: Vec<CreateContactRequest>,
    pub qualifications: Vec<CreateQualificationRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateContactRequest {
    pub contact_name: String,
    pub department: Option<String>,
    pub position: Option<String>,
    pub mobile_phone: String,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub is_primary: bool,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateQualificationRequest {
    pub qualification_name: String,
    pub qualification_type: String,
    pub qualification_no: String,
    pub issuing_authority: String,
    pub issue_date: String,
    pub valid_until: String,
    pub attachment_path: Option<String>,
    pub need_annual_check: bool,
    pub annual_check_record: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateSupplierRequest {
    pub supplier_name: Option<String>,
    pub supplier_short_name: Option<String>,
    pub supplier_type: Option<String>,
    pub credit_code: Option<String>,
    pub registered_address: Option<String>,
    pub business_address: Option<String>,
    pub legal_representative: Option<String>,
    pub registered_capital: Option<String>,
    pub establishment_date: Option<String>,
    pub business_term: Option<String>,
    pub business_scope: Option<String>,
    pub taxpayer_type: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub contact_phone: Option<String>,
    pub fax: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    pub main_business: Option<String>,
    pub main_market: Option<String>,
    pub employee_count: Option<i32>,
    pub annual_revenue: Option<String>,
    pub grade: Option<String>,
    pub grade_score: Option<String>,
    pub status: Option<String>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateContactRequest {
    pub contact_name: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub mobile_phone: Option<String>,
    pub tel_phone: Option<String>,
    pub email: Option<String>,
    pub wechat: Option<String>,
    pub qq: Option<String>,
    pub is_primary: Option<bool>,
    pub remarks: Option<String>,
}

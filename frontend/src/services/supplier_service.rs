//! 供应商管理服务 API 客户端
//! 提供供应商管理相关的 API 调用方法

use crate::services::api::ApiService;
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

pub struct SupplierService;

impl SupplierService {
    pub async fn list(query: SupplierQuery) -> Result<Vec<Supplier>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(ref supplier_type) = query.supplier_type {
            params.push(format!("supplier_type={}", supplier_type));
        }
        if let Some(ref grade) = query.grade {
            params.push(format!("grade={}", grade));
        }
        if let Some(ref status) = query.status {
            params.push(format!("status={}", status));
        }
        if let Some(ref keyword) = query.keyword {
            params.push(format!("keyword={}", keyword));
        }
        if let Some(ref sort_by) = query.sort_by {
            params.push(format!("sort_by={}", sort_by));
        }
        if let Some(ref sort_order) = query.sort_order {
            params.push(format!("sort_order={}", sort_order));
        }
        if let Some(is_enabled) = query.is_enabled {
            params.push(format!("is_enabled={}", is_enabled));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/suppliers{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let suppliers: Vec<Supplier> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(suppliers)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get(id: i32) -> Result<Supplier, String> {
        let response: serde_json::Value = ApiService::get(&format!("/suppliers/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取供应商详情失败".to_string())
    }

    pub async fn create(req: CreateSupplierRequest) -> Result<Supplier, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/suppliers", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建供应商失败".to_string())
    }

    pub async fn update(id: i32, req: UpdateSupplierRequest) -> Result<Supplier, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/suppliers/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新供应商失败".to_string())
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/suppliers/{}", id)).await
    }

    pub async fn toggle_status(id: i32, enable: bool) -> Result<Supplier, String> {
        let body = serde_json::json!({ "enable": enable });
        let response: serde_json::Value = ApiService::post(&format!("/suppliers/{}/toggle-status", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "切换供应商状态失败".to_string())
    }

    pub async fn list_contacts(supplier_id: i32) -> Result<Vec<SupplierContact>, String> {
        let response: serde_json::Value = ApiService::get(&format!("/suppliers/{}/contacts", supplier_id)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let contacts: Vec<SupplierContact> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(contacts)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn create_contact(supplier_id: i32, req: CreateContactRequest) -> Result<SupplierContact, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/suppliers/{}/contacts", supplier_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建联系人失败".to_string())
    }

    pub async fn update_contact(supplier_id: i32, contact_id: i32, req: UpdateContactRequest) -> Result<SupplierContact, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/suppliers/{}/contacts/{}", supplier_id, contact_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新联系人失败".to_string())
    }

    pub async fn delete_contact(supplier_id: i32, contact_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/suppliers/{}/contacts/{}", supplier_id, contact_id)).await
    }

    pub async fn list_qualifications(supplier_id: i32) -> Result<Vec<SupplierQualification>, String> {
        let response: serde_json::Value = ApiService::get(&format!("/suppliers/{}/qualifications", supplier_id)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let qualifications: Vec<SupplierQualification> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(qualifications)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn create_qualification(supplier_id: i32, req: CreateQualificationRequest) -> Result<SupplierQualification, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/suppliers/{}/qualifications", supplier_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建资质失败".to_string())
    }
}
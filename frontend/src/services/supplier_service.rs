//! 供应商管理服务 API 客户端
//! 提供供应商管理相关的 API 调用方法

use crate::models::api_response::ApiResponse;
use crate::models::supplier::{
    CreateContactRequest, CreateQualificationRequest, CreateSupplierRequest, Supplier,
    SupplierContact, SupplierQualification, SupplierQuery, UpdateContactRequest,
    UpdateSupplierRequest,
};
use crate::services::api::ApiService;

pub struct SupplierService;

impl SupplierService {
    pub async fn list(
        query: SupplierQuery,
    ) -> Result<crate::models::supplier::SupplierListResponse, String> {
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
            params.push(format!("keyword={}", urlencoding::encode(keyword)));
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

        let response: ApiResponse<crate::models::supplier::SupplierListResponse> =
            ApiService::get(&format!("/suppliers{}", query_string)).await?;
        response.into_result()
    }

    pub async fn get(id: i32) -> Result<Supplier, String> {
        let response: ApiResponse<Supplier> =
            ApiService::get(&format!("/suppliers/{}", id)).await?;
        response.into_result()
    }

    pub async fn create(req: CreateSupplierRequest) -> Result<Supplier, String> {
        let response: ApiResponse<Supplier> = ApiService::post("/suppliers", &req).await?;
        response.into_result()
    }

    pub async fn update(id: i32, req: UpdateSupplierRequest) -> Result<Supplier, String> {
        let response: ApiResponse<Supplier> =
            ApiService::put(&format!("/suppliers/{}", id), &req).await?;
        response.into_result()
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/suppliers/{}", id)).await
    }

    pub async fn toggle_status(id: i32, enable: bool) -> Result<Supplier, String> {
        let response: ApiResponse<Supplier> = ApiService::post(
            &format!("/suppliers/{}/toggle-status", id),
            &serde_json::json!({ "enable": enable }),
        )
        .await?;
        response.into_result()
    }

    pub async fn list_contacts(supplier_id: i32) -> Result<Vec<SupplierContact>, String> {
        let response: ApiResponse<Vec<SupplierContact>> =
            ApiService::get(&format!("/suppliers/{}/contacts", supplier_id)).await?;
        response.into_result()
    }

    pub async fn create_contact(
        supplier_id: i32,
        req: CreateContactRequest,
    ) -> Result<SupplierContact, String> {
        let response: ApiResponse<SupplierContact> =
            ApiService::post(&format!("/suppliers/{}/contacts", supplier_id), &req).await?;
        response.into_result()
    }

    pub async fn update_contact(
        supplier_id: i32,
        contact_id: i32,
        req: UpdateContactRequest,
    ) -> Result<SupplierContact, String> {
        let response: ApiResponse<SupplierContact> = ApiService::put(
            &format!("/suppliers/{}/contacts/{}", supplier_id, contact_id),
            &req,
        )
        .await?;
        response.into_result()
    }

    pub async fn delete_contact(supplier_id: i32, contact_id: i32) -> Result<(), String> {
        ApiService::delete(&format!(
            "/suppliers/{}/contacts/{}",
            supplier_id, contact_id
        ))
        .await
    }

    pub async fn list_qualifications(
        supplier_id: i32,
    ) -> Result<Vec<SupplierQualification>, String> {
        let response: ApiResponse<Vec<SupplierQualification>> =
            ApiService::get(&format!("/suppliers/{}/qualifications", supplier_id)).await?;
        response.into_result()
    }

    pub async fn create_qualification(
        supplier_id: i32,
        req: CreateQualificationRequest,
    ) -> Result<SupplierQualification, String> {
        let response: ApiResponse<SupplierQualification> =
            ApiService::post(&format!("/suppliers/{}/qualifications", supplier_id), &req).await?;
        response.into_result()
    }
}

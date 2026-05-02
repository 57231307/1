//! 供应商管理服务 API 客户端
//! 提供供应商管理相关的 API 调用方法

use crate::models::supplier::{
    CreateContactRequest, CreateQualificationRequest, CreateSupplierRequest, Supplier,
    SupplierContact, SupplierQualification, SupplierQuery, UpdateContactRequest,
    UpdateSupplierRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct SupplierService;

impl CrudService for SupplierService {
    type Model = Supplier;
    type ListResponse = crate::models::supplier::SupplierListResponse;
    type CreateRequest = CreateSupplierRequest;
    type UpdateRequest = UpdateSupplierRequest;

    fn base_path() -> &'static str {
        "/suppliers"
    }
}

impl SupplierService {
    pub async fn toggle_status(id: i32, enable: bool) -> Result<Supplier, String> {
        ApiService::post(&format!("/suppliers/{}/toggle-status", id), &serde_json::json!({ "enable": enable })).await
    }

    pub async fn list_contacts(supplier_id: i32) -> Result<Vec<SupplierContact>, String> {
        ApiService::get(&format!("/suppliers/{}/contacts", supplier_id)).await
    }

    pub async fn create_contact(supplier_id: i32, req: CreateContactRequest) -> Result<SupplierContact, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/suppliers/{}/contacts", supplier_id), &body).await
    }

    pub async fn update_contact(supplier_id: i32, contact_id: i32, req: UpdateContactRequest) -> Result<SupplierContact, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::put(&format!("/suppliers/{}/contacts/{}", supplier_id, contact_id), &body).await
    }

    pub async fn delete_contact(supplier_id: i32, contact_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/suppliers/{}/contacts/{}", supplier_id, contact_id)).await
    }

    pub async fn list_qualifications(supplier_id: i32) -> Result<Vec<SupplierQualification>, String> {
        ApiService::get(&format!("/suppliers/{}/qualifications", supplier_id)).await
    }

    pub async fn create_qualification(supplier_id: i32, req: CreateQualificationRequest) -> Result<SupplierQualification, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/suppliers/{}/qualifications", supplier_id), &body).await
    }
}

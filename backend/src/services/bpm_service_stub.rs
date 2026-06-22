//! BPM 服务占位方法（待后续迭代实现）
//!
//! 拆分自 bpm_service.rs：原 impl BpmService 第二段 7 个未实现方法，
//! 当前统一返回 AppError::bad_request/not_found 占位。

use crate::models::dto::bpm_dto::{
    CreateProcessDefinitionRequest, CreateVersionRequest, ProcessDefinitionQuery,
    UpdateProcessDefinitionRequest,
};
use crate::models::dto::PageResponse;
use crate::models::{bpm_process_definition, bpm_process_instance};
use crate::utils::error::AppError;

use super::bpm_service::BpmService;

impl BpmService {
    pub async fn create_process_definition(
        &self,
        _req: CreateProcessDefinitionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
    pub async fn get_process_definition(
        &self,
        _id: i32,
    ) -> Result<Option<bpm_process_definition::Model>, AppError> {
        Err(AppError::not_found(format!(
            "Process definition not found: {}",
            _id
        )))
    }
    pub async fn update_process_definition(
        &self,
        _id: i32,
        _req: UpdateProcessDefinitionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
    pub async fn delete_process_definition(&self, _id: i32) -> Result<(), AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
    pub async fn list_process_definitions(
        &self,
        _query: ProcessDefinitionQuery,
    ) -> Result<PageResponse<bpm_process_definition::Model>, AppError> {
        Ok(PageResponse {
            data: vec![],
            total: 0,
            page: 1,
            page_size: 10,
            total_pages: 0,
        })
    }
    pub async fn create_process_version(
        &self,
        _req: CreateVersionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
    pub async fn list_process_versions(
        &self,
        _definition_id: i32,
    ) -> Result<Vec<bpm_process_definition::Model>, AppError> {
        Ok(vec![])
    }
    pub async fn activate_process_version(
        &self,
        _id: i32,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
    pub async fn save_as_template(&self, _id: i32, _name: String) -> Result<(), AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
    pub async fn list_templates(
        &self,
        _query: TemplateQuery,
    ) -> Result<PageResponse<bpm_process_definition::Model>, AppError> {
        Ok(PageResponse {
            data: vec![],
            total: 0,
            page: 1,
            page_size: 10,
            total_pages: 0,
        })
    }
    pub async fn create_from_template(
        &self,
        _template_id: i32,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::bad_request("Not implemented"))
    }
}

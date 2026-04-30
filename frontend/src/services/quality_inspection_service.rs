use crate::models::quality_inspection::{
    CreateInspectionRecordRequest, CreateInspectionStandardRequest, CreateQualityDefectRequest,
    HandleDefectRequest, InspectionRecord, InspectionStandard, QualityDefect, QualityStatistics,
};
use crate::services::api::ApiService;

pub struct QualityInspectionService;

impl QualityInspectionService {
    pub async fn list_standards(
        product_id: Option<i32>,
        inspection_type: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<InspectionStandard>, String> {
        let mut query = String::new();
        if let Some(pid) = product_id {
            query.push_str(&format!("product_id={}&", pid));
        }
        if let Some(it) = inspection_type {
            query.push_str(&format!("inspection_type={}&", it));
        }
        if let Some(s) = status {
            query.push_str(&format!("status={}&", s));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));

        ApiService::get(&format!("/quality-standards?{}", query)).await
    }

    pub async fn get_standard(id: i32) -> Result<InspectionStandard, String> {
        ApiService::get(&format!("/quality-standards/{}", id)).await
    }

    pub async fn create_standard(
        req: CreateInspectionStandardRequest,
    ) -> Result<InspectionStandard, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/quality-standards", &payload).await
    }

    pub async fn update_standard(
        id: i32,
        req: CreateInspectionStandardRequest,
    ) -> Result<InspectionStandard, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/quality-standards/{}", id), &payload).await
    }

    pub async fn delete_standard(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/quality-standards/{}", id)).await
    }

    pub async fn list_records(
        product_id: Option<i32>,
        batch_number: Option<&str>,
        inspection_result: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<InspectionRecord>, String> {
        let mut query = String::new();
        if let Some(pid) = product_id {
            query.push_str(&format!("product_id={}&", pid));
        }
        if let Some(bn) = batch_number {
            query.push_str(&format!("batch_number={}&", bn));
        }
        if let Some(ir) = inspection_result {
            query.push_str(&format!("inspection_result={}&", ir));
        }
        if let Some(sd) = start_date {
            query.push_str(&format!("start_date={}&", sd));
        }
        if let Some(ed) = end_date {
            query.push_str(&format!("end_date={}&", ed));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));

        ApiService::get(&format!("/quality-inspection/records?{}", query)).await
    }

    pub async fn get_record(id: i32) -> Result<InspectionRecord, String> {
        ApiService::get(&format!("/quality-inspection/records/{}", id)).await
    }

    pub async fn create_record(
        req: CreateInspectionRecordRequest,
    ) -> Result<InspectionRecord, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/quality-inspections/records", &payload).await
    }

    pub async fn get_statistics(product_id: Option<i32>) -> Result<QualityStatistics, String> {
        let query = if let Some(pid) = product_id {
            format!("?product_id={}", pid)
        } else {
            String::new()
        };
        ApiService::get(&format!("/quality-inspection/statistics{}", query)).await
    }

    pub async fn list_defects(
        record_id: Option<i32>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<QualityDefect>, String> {
        let mut query = String::new();
        if let Some(rid) = record_id {
            query.push_str(&format!("record_id={}&", rid));
        }
        if let Some(s) = status {
            query.push_str(&format!("status={}&", s));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));

        ApiService::get(&format!("/quality-inspection/defects?{}", query)).await
    }

    pub async fn create_defect(req: CreateQualityDefectRequest) -> Result<QualityDefect, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/quality-inspections/defects", &payload).await
    }

    pub async fn handle_defect(id: i32, req: HandleDefectRequest) -> Result<QualityDefect, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(
            &format!("/quality-inspections/defects/{}/handle", id),
            &payload,
        )
        .await
    }
}

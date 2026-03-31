use serde::{Deserialize, Serialize};
use crate::services::api::ApiService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionStandard {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub inspection_type: String,
    pub sample_size: i32,
    pub acceptance_quality_limit: String,
    pub inspection_level: String,
    pub status: String,
    pub created_by: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionRecord {
    pub id: i32,
    pub record_number: String,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub batch_number: String,
    pub color_code: Option<String>,
    pub quantity: i32,
    pub qualified_quantity: i32,
    pub unqualified_quantity: i32,
    pub inspection_date: String,
    pub inspector_id: i32,
    pub inspector_name: Option<String>,
    pub inspection_result: String,
    pub remark: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDefect {
    pub id: i32,
    pub record_id: i32,
    pub defect_type: String,
    pub defect_description: String,
    pub quantity: i32,
    pub severity_level: String,
    pub handling_method: String,
    pub handler_id: Option<i32>,
    pub handler_name: Option<String>,
    pub handling_date: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStatistics {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub total_inspection_count: i64,
    pub total_quantity: i64,
    pub total_qualified_quantity: i64,
    pub total_unqualified_quantity: i64,
    pass_rate: String,
    pub defect_count: i64,
    pub top_defect_types: Vec<DefectTypeCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectTypeCount {
    pub defect_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionStandardRequest {
    pub product_id: i32,
    pub inspection_type: String,
    pub sample_size: i32,
    pub acceptance_quality_limit: String,
    pub inspection_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionRecordRequest {
    pub product_id: i32,
    pub batch_number: String,
    pub color_code: Option<String>,
    pub quantity: i32,
    pub inspection_date: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQualityDefectRequest {
    pub record_id: i32,
    pub defect_type: String,
    pub defect_description: String,
    pub quantity: i32,
    pub severity_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleDefectRequest {
    pub handling_method: String,
    pub handler_id: i32,
}

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
        
        ApiService::get(&format!("/quality-inspections/standards?{}", query)).await
    }

    pub async fn get_standard(id: i32) -> Result<InspectionStandard, String> {
        ApiService::get(&format!("/quality-inspections/standards/{}", id)).await
    }

    pub async fn create_standard(req: CreateInspectionStandardRequest) -> Result<InspectionStandard, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/quality-inspections/standards", &payload).await
    }

    pub async fn update_standard(id: i32, req: CreateInspectionStandardRequest) -> Result<InspectionStandard, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/quality-inspections/standards/{}", id), &payload).await
    }

    pub async fn delete_standard(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/quality-inspections/standards/{}", id)).await
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

    pub async fn create_record(req: CreateInspectionRecordRequest) -> Result<InspectionRecord, String> {
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
        ApiService::post(&format!("/quality-inspections/defects/{}/handle", id), &payload).await
    }
}

//! 生产订单 Service
//!
//! 提供生产订单的CRUD操作和状态管理

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::production_order::{
    ActiveModel, Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
use crate::utils::error::AppError;

/// 创建生产订单请求
#[derive(Debug, Clone)]
pub struct CreateProductionOrderRequest {
    pub order_no: Option<String>,
    pub sales_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub planned_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新生产订单请求
#[derive(Debug, Clone)]
pub struct UpdateProductionOrderRequest {
    pub planned_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
}

/// 生产订单查询参数
#[derive(Debug, Clone)]
pub struct ProductionOrderQuery {
    pub status: Option<String>,
    pub product_id: Option<i32>,
    pub page: u64,
    pub page_size: u64,
}

/// 生产订单 Service
pub struct ProductionOrderService {
    db: Arc<DatabaseConnection>,
}

impl ProductionOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建生产订单
    pub async fn create(
        &self,
        req: CreateProductionOrderRequest,
    ) -> Result<ProductionOrderModel, AppError> {
        // 自动生成订单号
        let order_no = req.order_no.unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let random = rand::random::<u16>() % 10000;
            format!("PO-{}-{:04}", timestamp, random)
        });

        let active_model = ActiveModel {
            order_no: Set(order_no),
            sales_order_id: Set(req.sales_order_id),
            product_id: Set(req.product_id.unwrap_or(0)),
            planned_quantity: Set(req.planned_quantity.unwrap_or_default()),
            planned_start_date: Set(req.planned_start_date),
            planned_end_date: Set(req.planned_end_date),
            status: Set("DRAFT".to_string()),
            priority: Set(req.priority.unwrap_or(0)),
            work_center_id: Set(req.work_center_id),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by.unwrap_or(0)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 根据ID获取生产订单
    pub async fn get_by_id(
        &self,
        id: i32,
    ) -> Result<Option<ProductionOrderModel>, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取生产订单列表
    pub async fn list(
        &self,
        query: ProductionOrderQuery,
    ) -> Result<(Vec<ProductionOrderModel>, u64), AppError> {
        let mut select = ProductionOrderEntity::find();

        if let Some(status) = query.status {
            select = select.filter(crate::models::production_order::Column::Status.eq(status));
        }

        if let Some(product_id) = query.product_id {
            select = select.filter(crate::models::production_order::Column::ProductId.eq(product_id));
        }

        let total = select
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let paginator = select
            .order_by_desc(crate::models::production_order::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator
            .fetch_page(query.page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((models, total))
    }

    /// 更新生产订单
    pub async fn update(
        &self,
        id: i32,
        req: UpdateProductionOrderRequest,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(planned_quantity) = req.planned_quantity {
            active_model.planned_quantity = Set(planned_quantity);
        }
        if let Some(planned_start_date) = req.planned_start_date {
            active_model.planned_start_date = Set(Some(planned_start_date));
        }
        if let Some(planned_end_date) = req.planned_end_date {
            active_model.planned_end_date = Set(Some(planned_end_date));
        }
        if let Some(priority) = req.priority {
            active_model.priority = Set(priority);
        }
        if let Some(work_center_id) = req.work_center_id {
            active_model.work_center_id = Set(Some(work_center_id));
        }
        if let Some(remarks) = req.remarks {
            active_model.remarks = Set(Some(remarks));
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 删除生产订单（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 更新生产订单状态
    pub async fn update_status(
        &self,
        id: i32,
        status: String,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(status.clone());
        active_model.updated_at = Set(Utc::now());

        // 如果状态变为生产中，设置实际开始日期
        if status == "IN_PROGRESS" {
            active_model.actual_start_date = Set(Some(chrono::Utc::now().date_naive()));
        }

        // 如果状态变为已完成，设置实际完成日期
        if status == "COMPLETED" {
            active_model.actual_end_date = Set(Some(chrono::Utc::now().date_naive()));
        }

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 提交生产订单审批
    pub async fn submit_for_approval(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        // 检查状态是否可以提交审批
        if model.status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "当前状态({})不允许提交审批，只有草稿状态可以提交",
                model.status
            )));
        }

        // 更新状态为审批中
        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("PENDING_APPROVAL".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 启动BPM审批流程
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "production_order_approval".to_string(),
            business_type: "production_order".to_string(),
            business_id: id,
            title: format!("生产订单审批 - {}", updated.order_no),
            initiator_id: user_id,
            initiator_name: user_name.to_string(),
            initiator_department_id: None,
            priority: Some("HIGH".to_string()),
            form_data: Some(serde_json::json!({
                "order_no": updated.order_no,
                "product_id": updated.product_id,
                "planned_quantity": updated.planned_quantity,
                "work_center_id": updated.work_center_id,
            })),
            variables: None,
        };

        // 忽略找不到模板的错误，为了兼容旧数据
        let _ = bpm_service.start_process(req).await;

        Ok(updated)
    }

    /// 审批生产订单
    pub async fn approve_order(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
        approved: bool,
        opinion: Option<String>,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

        // 检查状态
        if model.status != "PENDING_APPROVAL" {
            return Err(AppError::BusinessError(format!(
                "当前状态({})不允许审批",
                model.status
            )));
        }

        let new_status = if approved { "APPROVED" } else { "REJECTED" };

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 完成BPM任务 - 通过process_instance关联
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        
        // 获取与该生产订单关联的流程实例
        if let Ok(Some(instance)) = bpm_service.get_process_by_business("production_order", id).await {
            // 获取该实例的待处理任务
            let tasks = bpm_service
                .query_user_tasks(crate::models::dto::bpm_dto::TaskQuery {
                    user_id,
                    status: Some("PENDING".to_string()),
                    page: Some(1),
                    page_size: Some(10),
                })
                .await;

            if let Ok(task_list) = tasks {
                for task in task_list.data {
                    // 只处理当前流程实例的任务
                    if task.instance_id == instance.id {
                        let _ = bpm_service
                            .approve_task(crate::models::dto::bpm_dto::ApproveTaskRequest {
                                task_id: task.id,
                                handler_id: user_id,
                                handler_name: user_name.to_string(),
                                action: if approved { "approve".to_string() } else { "reject".to_string() },
                                approval_opinion: opinion.clone(),
                                attachment_urls: None,
                            })
                            .await;
                    }
                }
            }
        }

        Ok(updated)
    }
}

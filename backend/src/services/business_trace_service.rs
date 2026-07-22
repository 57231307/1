
use chrono::Utc;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, Set};
use serde_json::json;
use std::sync::Arc;

use crate::models::{business_trace_chain, business_trace_snapshot};
use crate::utils::error::AppError;

/// 业务追溯服务
#[derive(Debug, Clone)]
pub struct BusinessTraceService {
    db: Arc<DatabaseConnection>,
}

impl BusinessTraceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 按五维 ID 查询完整追溯链
    pub async fn find_trace_chain_by_five_dimension(
        &self,
        five_dimension_id: &str,
    ) -> Result<Vec<business_trace_chain::Model>, AppError> {
        business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::FiveDimensionId.eq(five_dimension_id))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 按追溯链 ID 查询
    pub async fn find_trace_chain_by_id(
        &self,
        trace_chain_id: &str,
    ) -> Result<Vec<business_trace_chain::Model>, AppError> {
        business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::TraceChainId.eq(trace_chain_id))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 正向追溯：从供应商到客户
    pub async fn forward_trace(
        &self,
        supplier_id: i32,
        batch_no: &str,
    ) -> Result<Vec<business_trace_chain::Model>, AppError> {
        // 找到所有从该供应商开始的追溯链
        business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::SupplierId.eq(supplier_id))
            .filter(business_trace_chain::Column::BatchNo.eq(batch_no))
            .filter(business_trace_chain::Column::CurrentStage.eq("PURCHASE_RECEIPT"))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 反向追溯：从客户到供应商
    pub async fn backward_trace(
        &self,
        customer_id: i32,
        batch_no: &str,
    ) -> Result<Vec<business_trace_chain::Model>, AppError> {
        // 找到所有到该客户的追溯链
        let traces = business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::CustomerId.eq(customer_id))
            .filter(business_trace_chain::Column::BatchNo.eq(batch_no))
            .filter(business_trace_chain::Column::CurrentStage.eq("SALES_DELIVERY"))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Desc)
            .all(&*self.db)
            .await?;

        // 完整的反向追溯：递归查询 previous_trace_id
        let mut all_traces = traces;
        let mut trace_ids: Vec<i32> = all_traces.iter().map(|t| t.id).collect();

        while !trace_ids.is_empty() {
            let parent_traces = business_trace_chain::Entity::find()
                .filter(business_trace_chain::Column::PreviousTraceId.is_in(trace_ids.clone()))
                .all(&*self.db)
                .await?;

            if parent_traces.is_empty() {
                break;
            }

            trace_ids = parent_traces.iter().map(|t| t.id).collect();
            all_traces.extend(parent_traces);
        }

        // 按创建时间排序
        all_traces.sort_by_key(|a| a.created_at);

        Ok(all_traces)
    }

    /// 创建追溯快照
    pub async fn create_snapshot(
        &self,
        trace_chain_id: &str,
    ) -> Result<business_trace_snapshot::Model, AppError> {
        // 查询追溯链
        let traces = self.find_trace_chain_by_id(trace_chain_id).await?;

        if traces.is_empty() {
            return Err(AppError::not_found("追溯链不存在"));
        }

        let first_trace = &traces[0];
        let last_trace = traces
            .last()
            .ok_or_else(|| AppError::not_found("No trace found"))?;

        // 第一个环节有供应商，最后一个环节有客户
        let supplier_name =
            Self::fetch_supplier_name(&*self.db, first_trace.supplier_id).await?;
        let customer_name =
            Self::fetch_customer_name(&*self.db, last_trace.customer_id).await?;
        let trace_path = Self::build_trace_path(&traces);

        let active_snapshot = Self::build_snapshot_active_model(
            trace_chain_id,
            first_trace,
            last_trace,
            supplier_name,
            customer_name,
            trace_path,
        );

        active_snapshot
            .insert(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 查询供应商名称（supplier_id 为 None 时返回 None）
    async fn fetch_supplier_name(
        db: &DatabaseConnection,
        supplier_id: Option<i32>,
    ) -> Result<Option<String>, AppError> {
        use crate::models::supplier;
        match supplier_id {
            Some(id) => Ok(supplier::Entity::find_by_id(id)
                .one(db)
                .await?
                .map(|s| s.supplier_name)),
            None => Ok(None),
        }
    }

    /// 查询客户名称（customer_id 为 None 时返回 None）
    async fn fetch_customer_name(
        db: &DatabaseConnection,
        customer_id: Option<i32>,
    ) -> Result<Option<String>, AppError> {
        use crate::models::customer;
        match customer_id {
            Some(id) => Ok(customer::Entity::find_by_id(id)
                .one(db)
                .await?
                .map(|c| c.customer_name)),
            None => Ok(None),
        }
    }

    /// 构建追溯路径 JSON（stage/bill_type/bill_no/quantity_meters/warehouse_id/created_at）
    fn build_trace_path(traces: &[business_trace_chain::Model]) -> serde_json::Value {
        json!(traces
            .iter()
            .map(|t| {
                json!({
                    "stage": t.current_stage,
                    "bill_type": t.current_bill_type,
                    "bill_no": t.current_bill_no,
                    "quantity_meters": t.quantity_meters.to_string(),
                    "warehouse_id": t.warehouse_id,
                    "created_at": t.created_at
                })
            })
            .collect::<Vec<_>>())
    }

    /// 构建追溯快照 ActiveModel（聚合首/末环节信息与供应商/客户名称）
    fn build_snapshot_active_model(
        trace_chain_id: &str,
        first_trace: &business_trace_chain::Model,
        last_trace: &business_trace_chain::Model,
        supplier_name: Option<String>,
        customer_name: Option<String>,
        trace_path: serde_json::Value,
    ) -> business_trace_snapshot::ActiveModel {
        business_trace_snapshot::ActiveModel {
            id: Default::default(),
            trace_chain_id: Set(trace_chain_id.to_string()),
            five_dimension_id: Set(first_trace.five_dimension_id.clone()),
            product_id: Set(first_trace.product_id),
            batch_no: Set(first_trace.batch_no.clone()),
            color_no: Set(first_trace.color_no.clone()),
            grade: Set(first_trace.grade.clone()),
            current_stage: Set(last_trace.current_stage.clone()),
            warehouse_id: Set(last_trace.warehouse_id),
            current_quantity_meters: Set(last_trace.quantity_meters),
            current_quantity_kg: Set(last_trace.quantity_kg),
            supplier_name: Set(supplier_name),
            customer_name: Set(customer_name),
            trace_path: Set(trace_path),
            snapshot_time: Set(Utc::now()),
        }
    }

}

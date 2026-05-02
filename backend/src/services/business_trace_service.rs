use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, Set};
use serde_json::json;
use std::sync::Arc;

use crate::models::{business_trace_chain, business_trace_snapshot};
use crate::utils::fabric_five_dimension::FabricFiveDimension;

/// 业务追溯服务
#[derive(Debug, Clone)]
pub struct BusinessTraceService {
    db: Arc<DatabaseConnection>,
}

impl BusinessTraceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建追溯链起点（采购收货）
    pub async fn create_trace_start(
        &self,
        five_dimension: &FabricFiveDimension,
        bill_type: String,
        bill_no: String,
        bill_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        warehouse_id: i32,
        supplier_id: i32,
        created_by: Option<i32>,
    ) -> Result<business_trace_chain::Model, sea_orm::DbErr> {
        // 生成追溯链 ID
        let trace_chain_id = self.generate_trace_chain_id(five_dimension, &bill_type, &bill_no);

        let active_trace = business_trace_chain::ActiveModel {
            id: Set(0),
            trace_chain_id: Set(trace_chain_id),
            five_dimension_id: Set(five_dimension.generate_unique_id()),
            product_id: Set(five_dimension.product_id),
            batch_no: Set(five_dimension.batch_no.clone()),
            color_no: Set(five_dimension.color_no.clone()),
            dye_lot_no: Set(five_dimension.dye_lot_no.clone()),
            grade: Set(five_dimension.grade.clone()),
            current_stage: Set("PURCHASE_RECEIPT".to_string()),
            current_bill_type: Set(bill_type),
            current_bill_no: Set(bill_no),
            current_bill_id: Set(bill_id),
            previous_trace_id: Set(None),
            next_trace_id: Set(None),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            warehouse_id: Set(warehouse_id),
            supplier_id: Set(Some(supplier_id)),
            customer_id: Set(None),
            workshop_id: Set(None),
            trace_status: Set("ACTIVE".to_string()),
            remarks: Set(None),
            created_at: Set(Utc::now()),
            created_by: Set(created_by),
        };

        active_trace.insert(&*self.db).await
    }

    /// 添加追溯环节
    pub async fn add_trace_stage(
        &self,
        previous_trace_id: i32,
        stage: String,
        bill_type: String,
        bill_no: String,
        bill_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        warehouse_id: i32,
        customer_id: Option<i32>,
        workshop_id: Option<i32>,
        created_by: Option<i32>,
    ) -> Result<business_trace_chain::Model, sea_orm::DbErr> {
        // 查询上一环节
        let previous_trace = business_trace_chain::Entity::find_by_id(previous_trace_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound("上一环节不存在".to_string()))?;

        // 创建新环节
        let active_trace = business_trace_chain::ActiveModel {
            id: Set(0),
            trace_chain_id: Set(previous_trace.trace_chain_id.clone()),
            five_dimension_id: Set(previous_trace.five_dimension_id.clone()),
            product_id: Set(previous_trace.product_id),
            batch_no: Set(previous_trace.batch_no.clone()),
            color_no: Set(previous_trace.color_no.clone()),
            dye_lot_no: Set(previous_trace.dye_lot_no.clone()),
            grade: Set(previous_trace.grade.clone()),
            current_stage: Set(stage.clone()),
            current_bill_type: Set(bill_type),
            current_bill_no: Set(bill_no),
            current_bill_id: Set(bill_id),
            previous_trace_id: Set(Some(previous_trace_id)),
            next_trace_id: Set(None),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            warehouse_id: Set(warehouse_id),
            supplier_id: Set(None),
            customer_id: Set(customer_id),
            workshop_id: Set(workshop_id),
            trace_status: Set("ACTIVE".to_string()),
            remarks: Set(None),
            created_at: Set(Utc::now()),
            created_by: Set(created_by),
        };

        let new_trace = active_trace.insert(&*self.db).await?;

        // 更新上一环节的 next_trace_id
        let mut previous_active: business_trace_chain::ActiveModel = previous_trace.into();
        previous_active.next_trace_id = Set(Some(new_trace.id));
        let new_status = if stage == "SALES_DELIVERY" {
            "COMPLETED".to_string()
        } else {
            "ACTIVE".to_string()
        };
        previous_active.trace_status = Set(new_status);
        previous_active.update(&*self.db).await?;

        Ok(new_trace)
    }

    /// 按五维 ID 查询完整追溯链
    pub async fn find_trace_chain_by_five_dimension(
        &self,
        five_dimension_id: &str,
    ) -> Result<Vec<business_trace_chain::Model>, sea_orm::DbErr> {
        business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::FiveDimensionId.eq(five_dimension_id))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
    }

    /// 按追溯链 ID 查询
    pub async fn find_trace_chain_by_id(
        &self,
        trace_chain_id: &str,
    ) -> Result<Vec<business_trace_chain::Model>, sea_orm::DbErr> {
        business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::TraceChainId.eq(trace_chain_id))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
    }

    /// 正向追溯：从供应商到客户
    pub async fn forward_trace(
        &self,
        supplier_id: i32,
        batch_no: &str,
    ) -> Result<Vec<business_trace_chain::Model>, sea_orm::DbErr> {
        // 找到所有从该供应商开始的追溯链
        business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::SupplierId.eq(supplier_id))
            .filter(business_trace_chain::Column::BatchNo.eq(batch_no))
            .filter(business_trace_chain::Column::CurrentStage.eq("PURCHASE_RECEIPT"))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Asc)
            .all(&*self.db)
            .await
    }

    /// 反向追溯：从客户到供应商
    pub async fn backward_trace(
        &self,
        customer_id: i32,
        batch_no: &str,
    ) -> Result<Vec<business_trace_chain::Model>, sea_orm::DbErr> {
        // 找到所有到该客户的追溯链
        let traces = business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::CustomerId.eq(customer_id))
            .filter(business_trace_chain::Column::BatchNo.eq(batch_no))
            .filter(business_trace_chain::Column::CurrentStage.eq("SALES_DELIVERY"))
            .order_by(business_trace_chain::Column::CreatedAt, Order::Desc)
            .all(&*self.db)
            .await?;

        // 如果需要完整的反向追溯，可以继续查询 previous_trace_id
        // 这里简化实现，只返回直接相关的记录
        Ok(traces)
    }

    /// 创建追溯快照
    pub async fn create_snapshot(
        &self,
        trace_chain_id: &str,
    ) -> Result<business_trace_snapshot::Model, sea_orm::DbErr> {
        // 查询追溯链
        let traces = self.find_trace_chain_by_id(trace_chain_id).await?;

        if traces.is_empty() {
            return Err(sea_orm::DbErr::RecordNotFound("追溯链不存在".to_string()));
        }

        let first_trace = &traces[0];
        let last_trace = traces.last().ok_or_else(|| AppError::NotFound("No trace found".into()))?;

        // 获取追溯链中的供应商ID和客户ID（第一个环节有供应商，最后一个环节有客户）
        let supplier_id = first_trace.supplier_id;
        let customer_id = last_trace.customer_id;

        // 查询供应商名称
        let supplier_name = if let Some(supplier_id_val) = supplier_id {
            use crate::models::supplier;
            supplier::Entity::find_by_id(supplier_id_val)
                .one(&*self.db)
                .await?
                .map(|s| s.supplier_name)
        } else {
            None
        };

        // 查询客户名称
        let customer_name = if let Some(customer_id_val) = customer_id {
            use crate::models::customer;
            customer::Entity::find_by_id(customer_id_val)
                .one(&*self.db)
                .await?
                .map(|c| c.customer_name)
        } else {
            None
        };

        // 构建追溯路径
        let trace_path = json!(traces
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
            .collect::<Vec<_>>());

        let active_snapshot = business_trace_snapshot::ActiveModel {
            id: Set(0),
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
        };

        active_snapshot.insert(&*self.db).await
    }

    /// 生成追溯链 ID
    fn generate_trace_chain_id(
        &self,
        five_dimension: &FabricFiveDimension,
        bill_type: &str,
        bill_no: &str,
    ) -> String {
        format!(
            "TC-{}-{}-{}",
            five_dimension.generate_unique_id(),
            bill_type,
            bill_no
        )
    }
}

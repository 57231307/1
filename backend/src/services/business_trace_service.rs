use chrono::Utc;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, Set};
use serde_json::json;
use std::sync::Arc;

use crate::models::{business_trace_assist_link, business_trace_chain, business_trace_snapshot};
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
        let supplier_name = Self::fetch_supplier_name(&*self.db, first_trace.supplier_id).await?;
        let customer_name = Self::fetch_customer_name(&*self.db, last_trace.customer_id).await?;
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

    // ============================================================
    // V15 P2-06 业务追溯生产者（producer）
    //
    // 背景：业务追溯三张表（chain/snapshot/assist_links）此前只有读侧 producer，
    //      任何上游业务（采购收货、库存出入库、生产、委外、销售发货）直接 INSERT
    //      都可能绕过 V15 新增的 UNIQUE / CHECK / 触发器约束，触发 500。
    //
    // 本节提供"符合约束语义"的高层写入助手：
    //   * upsert_chain_node：处理 head/tail 唯一、自环检测、shape 校验。
    //   * link_assist：      通过 chain_id 找到 head.id 再插 link，避开直接持有 id。
    //   * upsert_snapshot：  依赖 V15 触发器做"存在 head + 字段自洽"校验，
    //                       重复 trace_chain_id 时改为 UPDATE（与"每 chain 一份最新"语义一致）。
    //
    // 上游业务只需调用这三个方法即可享受约束保护，无需了解 PG 层细节。
    // ============================================================

    /// 插入或更新一个 chain 节点（同 trace_chain_id 内部按 (current_stage, current_bill_no) 幂等）
    ///
    /// * 若同一 (trace_chain_id, current_stage, current_bill_no) 已存在，则原地更新数量/仓库/供应商/客户。
    /// * is_head = true 时，把同 trace_chain_id 其它 head 的 previous_trace_id 置为新节点 id，
    ///   从而保证 partial unique (trace_chain_id) WHERE previous_trace_id IS NULL 不冲突。
    /// * is_tail = true 时同理。
    ///
    /// 调用方必须把 new_node 的必填字段（trace_chain_id/current_stage/current_bill_no/...）置为 Set，
    /// 否则内部 try_as_ref() 返回 None 时直接 AppError::validation。
    pub async fn upsert_chain_node(
        &self,
        new_node: business_trace_chain::ActiveModel,
        is_head: bool,
        is_tail: bool,
    ) -> Result<business_trace_chain::Model, AppError> {
        use sea_orm::ActiveValue;

        // 必填字段先取出（不能 Set 为 NotSet）
        let trace_chain_id = match new_node.trace_chain_id.try_as_ref() {
            Some(v) => v.clone(),
            None => {
                return Err(AppError::validation("chain 缺少 trace_chain_id"));
            }
        };
        let current_stage = match new_node.current_stage.try_as_ref() {
            Some(v) => v.clone(),
            None => {
                return Err(AppError::validation("chain 缺少 current_stage"));
            }
        };
        let current_bill_no = match new_node.current_bill_no.try_as_ref() {
            Some(v) => v.clone(),
            None => {
                return Err(AppError::validation("chain 缺少 current_bill_no"));
            }
        };

        // 反向预检：自环 (由 CHECK 触发，但前置报 AppError 更友好)
        if let (Some(prev), Some(next)) = (
            new_node.previous_trace_id.try_as_ref(),
            new_node.next_trace_id.try_as_ref(),
        ) {
            if prev == next {
                return Err(AppError::validation(
                    "chain 节点禁止 previous_trace_id == next_trace_id 自环",
                ));
            }
        }

        // 量化字段非负（CHECK 也会兜底）
        if let (Some(qm), Some(qk)) = (
            new_node.quantity_meters.try_as_ref(),
            new_node.quantity_kg.try_as_ref(),
        ) {
            if *qm < rust_decimal::Decimal::ZERO || *qk < rust_decimal::Decimal::ZERO {
                return Err(AppError::validation("chain 数量字段禁止负值"));
            }
        }

        // 若 (trace_chain_id, current_stage, current_bill_no) 已存在则更新
        let existing = business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::TraceChainId.eq(trace_chain_id.clone()))
            .filter(business_trace_chain::Column::CurrentStage.eq(current_stage.clone()))
            .filter(business_trace_chain::Column::CurrentBillNo.eq(current_bill_no.clone()))
            .one(&*self.db)
            .await?;

        let saved = if let Some(prev_model) = existing {
            let mut upd: business_trace_chain::ActiveModel = prev_model.clone().into();
            if let ActiveValue::Set(q) = new_node.quantity_meters {
                upd.quantity_meters = ActiveValue::Set(q);
            }
            if let ActiveValue::Set(q) = new_node.quantity_kg {
                upd.quantity_kg = ActiveValue::Set(q);
            }
            if let ActiveValue::Set(w) = new_node.warehouse_id {
                upd.warehouse_id = ActiveValue::Set(w);
            }
            if let ActiveValue::Set(s) = new_node.supplier_id {
                upd.supplier_id = ActiveValue::Set(s);
            }
            if let ActiveValue::Set(c) = new_node.customer_id {
                upd.customer_id = ActiveValue::Set(c);
            }
            if let ActiveValue::Set(p) = new_node.previous_trace_id {
                upd.previous_trace_id = ActiveValue::Set(p);
            }
            if let ActiveValue::Set(n) = new_node.next_trace_id {
                upd.next_trace_id = ActiveValue::Set(n);
            }
            if let ActiveValue::Set(s) = new_node.trace_status {
                upd.trace_status = ActiveValue::Set(s);
            }
            upd.update(&*self.db).await.map_err(AppError::from)?
        } else {
            new_node.insert(&*self.db).await.map_err(AppError::from)?
        };

        // 头/尾晋升：清除同 trace_chain_id 旧 head/tail 的 (previous|next)_trace_id
        if is_head {
            let _ = business_trace_chain::Entity::update_many()
                .col_expr(
                    business_trace_chain::Column::PreviousTraceId,
                    sea_orm::sea_query::Expr::value(Option::<i32>::None),
                )
                .filter(business_trace_chain::Column::TraceChainId.eq(trace_chain_id.clone()))
                .filter(business_trace_chain::Column::Id.ne(saved.id))
                .filter(business_trace_chain::Column::PreviousTraceId.is_null())
                .exec(&*self.db)
                .await?;
        }
        if is_tail {
            let _ = business_trace_chain::Entity::update_many()
                .col_expr(
                    business_trace_chain::Column::NextTraceId,
                    sea_orm::sea_query::Expr::value(Option::<i32>::None),
                )
                .filter(business_trace_chain::Column::TraceChainId.eq(trace_chain_id.clone()))
                .filter(business_trace_chain::Column::Id.ne(saved.id))
                .filter(business_trace_chain::Column::NextTraceId.is_null())
                .exec(&*self.db)
                .await?;
        }

        Ok(saved)
    }

    /// 关联一条辅助核算记录到 chain head（trace_id 由 trace_chain_id 解析）
    pub async fn link_assist(
        &self,
        trace_chain_id: &str,
        mut assist: business_trace_assist_link::ActiveModel,
    ) -> Result<business_trace_assist_link::Model, AppError> {
        let head = business_trace_chain::Entity::find()
            .filter(business_trace_chain::Column::TraceChainId.eq(trace_chain_id.to_string()))
            .filter(business_trace_chain::Column::PreviousTraceId.is_null())
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("追溯链 head 节点不存在"))?;

        assist.id = Default::default();
        assist.trace_id = Set(head.id);
        assist.insert(&*self.db).await.map_err(AppError::from)
    }

    /// 写入一份 chain 最新快照（重复 trace_chain_id 时原地刷新）
    ///
    /// 依赖 V15 触发器做"存在 head + 字段自洽"校验；本方法对"同一 chain 多次调用"
    /// 自动改写为 UPDATE（每 chain 仅保留一份当前快照）。
    pub async fn upsert_snapshot(
        &self,
        trace_chain_id: &str,
        first: &business_trace_chain::Model,
        last: &business_trace_chain::Model,
        supplier_name: Option<String>,
        customer_name: Option<String>,
    ) -> Result<business_trace_snapshot::Model, AppError> {
        let trace_path = Self::build_trace_path(&[first.clone(), last.clone()]);

        let existing = business_trace_snapshot::Entity::find()
            .filter(business_trace_snapshot::Column::TraceChainId.eq(trace_chain_id.to_string()))
            .one(&*self.db)
            .await?;

        if let Some(prev) = existing {
            let mut upd: business_trace_snapshot::ActiveModel = prev.into();
            upd.current_stage = Set(last.current_stage.clone());
            upd.warehouse_id = Set(last.warehouse_id);
            upd.current_quantity_meters = Set(last.quantity_meters);
            upd.current_quantity_kg = Set(last.quantity_kg);
            upd.supplier_name = Set(supplier_name);
            upd.customer_name = Set(customer_name);
            upd.trace_path = Set(trace_path);
            upd.snapshot_time = Set(Utc::now());
            upd.update(&*self.db).await.map_err(AppError::from)
        } else {
            let active = Self::build_snapshot_active_model(
                trace_chain_id,
                first,
                last,
                supplier_name,
                customer_name,
                trace_path,
            );
            active.insert(&*self.db).await.map_err(AppError::from)
        }
    }

    /// 采购收货接入：为入库单每条明细创建追溯链 head 节点（best-effort，失败仅记日志不阻塞业务）
    /// trace_chain_id 生成规则：TC-{batch_no}-{product_id}-{color_no}，batch_no 为空时用 product_id+时间戳
    pub async fn record_purchase_receipt(
        &self,
        receipt: &crate::models::purchase_receipt::Model,
        items: &[crate::models::purchase_receipt_item::Model],
        user_id: i32,
    ) {
        use crate::models::business_trace_chain;
        use rust_decimal::Decimal;

        for item in items {
            let batch_no = item.batch_no.clone().unwrap_or_default();
            let color_no = item.color_code.clone().unwrap_or_default();
            let grade = item.grade.clone().unwrap_or_default();
            // trace_chain_id：batch_no 非空用批次键，为空用 product_id+时间戳保证唯一
            let trace_chain_id = if batch_no.is_empty() {
                format!(
                    "TC-{}-{}",
                    item.product_id,
                    chrono::Utc::now().timestamp_millis()
                )
            } else {
                format!("TC-{}-{}-{}", batch_no, item.product_id, color_no)
            };

            let node = business_trace_chain::ActiveModel {
                trace_chain_id: Set(trace_chain_id),
                five_dimension_id: Set(format!("FD-{}-{}-{}", item.product_id, batch_no, color_no)),
                product_id: Set(item.product_id),
                batch_no: Set(batch_no),
                color_no: Set(color_no),
                dye_lot_no: Set(item.internal_dye_lot_no.clone()),
                grade: Set(grade),
                current_stage: Set("PURCHASE_RECEIPT".to_string()),
                current_bill_type: Set("PURCHASE_RECEIPT".to_string()),
                current_bill_no: Set(receipt.receipt_no.clone()),
                current_bill_id: Set(receipt.id),
                previous_trace_id: Set(None),
                next_trace_id: Set(None),
                quantity_meters: Set(item.quantity),
                quantity_kg: Set(item.quantity_alt.unwrap_or(Decimal::ZERO)),
                warehouse_id: Set(receipt.warehouse_id),
                supplier_id: Set(Some(receipt.supplier_id)),
                customer_id: Set(None),
                workshop_id: Set(None),
                trace_status: Set("ACTIVE".to_string()),
                remarks: Set(None),
                created_at: Set(chrono::Utc::now()),
                created_by: Set(Some(user_id)),
                ..Default::default()
            };

            // best-effort：失败仅记日志，不阻塞采购收货业务
            if let Err(e) = self.upsert_chain_node(node, true, false).await {
                tracing::warn!(
                    "业务追溯 chain head 写入失败 receipt={} item={}: {}",
                    receipt.id,
                    item.id,
                    e
                );
            }
        }
    }

    /// 销售发货接入：为发货单每条明细创建追溯链 tail 节点（best-effort，失败仅记日志不阻塞业务）
    /// trace_chain_id 通过 batch_no+product_id+color_no 与采购收货 head 节点对齐
    pub async fn record_sales_delivery(
        &self,
        delivery: &crate::models::sales_delivery::Model,
        items: &[crate::models::sales_delivery_item::Model],
        user_id: i32,
    ) {
        use crate::models::business_trace_chain;
        use rust_decimal::Decimal;

        for item in items {
            let batch_no = item.batch_no.clone().unwrap_or_default();
            let color_no = item.color_no.clone().unwrap_or_default();
            let trace_chain_id = if batch_no.is_empty() {
                format!(
                    "TC-{}-{}",
                    item.product_id,
                    chrono::Utc::now().timestamp_millis()
                )
            } else {
                format!("TC-{}-{}-{}", batch_no, item.product_id, color_no)
            };

            let node = business_trace_chain::ActiveModel {
                trace_chain_id: Set(trace_chain_id.clone()),
                five_dimension_id: Set(format!("FD-{}-{}-{}", item.product_id, batch_no, color_no)),
                product_id: Set(item.product_id),
                batch_no: Set(batch_no),
                color_no: Set(color_no),
                dye_lot_no: Set(item.dye_lot_no.clone()),
                grade: Set("N/A".to_string()),
                current_stage: Set("SALES_DELIVERY".to_string()),
                current_bill_type: Set("SALES_DELIVERY".to_string()),
                current_bill_no: Set(delivery.delivery_no.clone()),
                current_bill_id: Set(delivery.id),
                previous_trace_id: Set(None),
                next_trace_id: Set(None),
                quantity_meters: Set(item.quantity),
                quantity_kg: Set(Decimal::ZERO),
                warehouse_id: Set(delivery.warehouse_id),
                supplier_id: Set(None),
                customer_id: Set(Some(delivery.customer_id)),
                workshop_id: Set(None),
                trace_status: Set("COMPLETED".to_string()),
                remarks: Set(None),
                created_at: Set(chrono::Utc::now()),
                created_by: Set(Some(user_id)),
                ..Default::default()
            };

            if let Err(e) = self.upsert_chain_node(node, false, true).await {
                tracing::warn!(
                    "业务追溯 chain tail 写入失败 delivery={} item={}: {}",
                    delivery.id,
                    item.id,
                    e
                );
            }

            // 刷新 chain 快照：查询 head 节点 + 当前 tail 节点
            let head = business_trace_chain::Entity::find()
                .filter(business_trace_chain::Column::TraceChainId.eq(&trace_chain_id))
                .filter(business_trace_chain::Column::CurrentStage.eq("PURCHASE_RECEIPT"))
                .one(&*self.db)
                .await
                .ok()
                .flatten();
            let tail = business_trace_chain::Entity::find()
                .filter(business_trace_chain::Column::TraceChainId.eq(&trace_chain_id))
                .filter(business_trace_chain::Column::CurrentStage.eq("SALES_DELIVERY"))
                .one(&*self.db)
                .await
                .ok()
                .flatten();
            if let (Some(first), Some(last)) = (head, tail) {
                if let Err(e) = self
                    .upsert_snapshot(&trace_chain_id, &first, &last, None, None)
                    .await
                {
                    tracing::warn!("业务追溯 snapshot 刷新失败 chain={}: {}", trace_chain_id, e);
                }
            }
        }
    }

    /// 关联辅助单据到追溯链 head（best-effort）
    pub async fn link_assist_to_chain(
        &self,
        trace_chain_id: &str,
        assist_type: &str,
        assist_id: i32,
        assist_code: &str,
        assist_name: &str,
    ) {
        use crate::models::business_trace_assist_link;
        let assist = business_trace_assist_link::ActiveModel {
            assist_type: Set(assist_type.to_string()),
            assist_id: Set(assist_id),
            assist_code: Set(assist_code.to_string()),
            assist_name: Set(assist_name.to_string()),
            remarks: Set(None),
            ..Default::default()
        };
        if let Err(e) = self.link_assist(trace_chain_id, assist).await {
            tracing::warn!(
                "业务追溯 assist link 写入失败 chain={}: {}",
                trace_chain_id,
                e
            );
        }
    }
}

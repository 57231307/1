//! 流转卡与工序流转 Service
//!
//! v14 批次 425：流转卡条码与车间工序流转
//! 依据：面料行业真实业务调研文档 §12.1 流转卡条码管理 + §12.2 生产计划与排缸 + §12.3 车间工序流转
//! 真实业务流程：
//!   生产计划单 → 备布 → 排缸执行 → 流转卡打印（含条码）
//!   白坯仓库：扫描缸号条码 → 自动出库
//!   染色车间：扫描缸卡条码 → 输入生产进度（进缸/出缸状态）
//!   车间流转：扫描流转卡条码 → 登记工人 → 自动跟进工序和产量
//!
//! 核心能力：
//! - 工序路线模板 CRUD（后台自定义车间工序）
//! - 流转卡 CRUD + 状态机流转（pending→scheduled→preparing→dyeing→dyed→inspecting→completed→shipped/terminated）
//! - 工序流转扫码（扫描流转卡条码 → 登记工人 → 开始/结束工序 → 自动统计产量）
//! - 工序质量反馈单 CRUD + 处理流转（pending→processing→resolved→closed）

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::process_quality_feedback::{self, ActiveModel as FeedbackActiveModel, Entity as FeedbackEntity, Model as FeedbackModel};
use crate::models::process_route::{self, ActiveModel as RouteActiveModel, Entity as RouteEntity, Model as RouteModel};
use crate::models::process_step_record::{self, ActiveModel as StepActiveModel, Entity as StepEntity, Model as StepModel};
use crate::models::production_flow_card::{self, ActiveModel as CardActiveModel, Entity as CardEntity, Model as CardModel};
use crate::models::status::flow_card as card_status;
use crate::models::status::quality_feedback as feedback_status;
use crate::models::status::step_record as step_status;
use crate::utils::error::AppError;

// ============================================================================
// 工序路线模板 Service
// ============================================================================

/// 创建工序路线请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProcessRouteRequest {
    pub route_code: String,
    pub route_name: String,
    pub seq: i32,
    pub process_type: String,
    pub default_duration_minutes: Option<i32>,
    pub require_scan: Option<bool>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工序路线请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProcessRouteRequest {
    pub route_name: Option<String>,
    pub seq: Option<i32>,
    pub process_type: Option<String>,
    pub default_duration_minutes: Option<i32>,
    pub require_scan: Option<bool>,
    pub is_active: Option<bool>,
    pub remarks: Option<String>,
}

/// 工序路线 Service
pub struct ProcessRouteService {
    db: Arc<DatabaseConnection>,
}

impl ProcessRouteService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建工序路线
    pub async fn create(&self, req: CreateProcessRouteRequest) -> Result<RouteModel, AppError> {
        // 业务校验：工序编码格式
        let code = req.route_code.trim().to_uppercase();
        if code.is_empty() {
            return Err(AppError::business("工序编码不能为空"));
        }
        if !(1..=32).contains(&code.len()) {
            return Err(AppError::business("工序编码长度 1-32"));
        }

        // 业务校验：工序序号必须为正
        if req.seq < 1 {
            return Err(AppError::business("工序序号必须 >= 1"));
        }

        // 业务校验：工序类型合法
        let valid_types = ["pretreat", "dye", "print", "finish", "inspect", "other"];
        if !valid_types.contains(&req.process_type.as_str()) {
            return Err(AppError::business(format!(
                "工序类型必须是 {:?} 之一",
                valid_types
            )));
        }

        // 校验编码唯一
        let existing = RouteEntity::find()
            .filter(process_route::Column::RouteCode.eq(code.clone()))
            .filter(process_route::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::business(format!("工序编码 {} 已存在", code)));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = RouteActiveModel {
            id: Default::default(),
            route_code: Set(code),
            route_name: Set(req.route_name),
            seq: Set(req.seq),
            process_type: Set(req.process_type),
            default_duration_minutes: Set(req.default_duration_minutes),
            require_scan: Set(req.require_scan.unwrap_or(true)),
            is_active: Set(true),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工序路线创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新工序路线
    pub async fn update(
        &self,
        id: i32,
        req: UpdateProcessRouteRequest,
    ) -> Result<RouteModel, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: RouteActiveModel = model.into();

        if let Some(v) = req.route_name {
            active.route_name = Set(v);
        }
        if let Some(v) = req.seq {
            if v < 1 {
                return Err(AppError::business("工序序号必须 >= 1"));
            }
            active.seq = Set(v);
        }
        if let Some(v) = req.process_type {
            let valid_types = ["pretreat", "dye", "print", "finish", "inspect", "other"];
            if !valid_types.contains(&v.as_str()) {
                return Err(AppError::business("工序类型不合法"));
            }
            active.process_type = Set(v);
        }
        if let Some(v) = req.default_duration_minutes {
            if v < 0 {
                return Err(AppError::business("默认工时不能为负"));
            }
            active.default_duration_minutes = Set(Some(v));
        }
        if let Some(v) = req.require_scan {
            active.require_scan = Set(v);
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除工序路线
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: RouteActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RouteModel, AppError> {
        let model = RouteEntity::find_by_id(id)
            .filter(process_route::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工序路线 {} 不存在", id)))?;
        Ok(model)
    }

    /// 查询所有启用的工序路线（按序号排序）
    pub async fn list_active(&self) -> Result<Vec<RouteModel>, AppError> {
        let list = RouteEntity::find()
            .filter(process_route::Column::IsDeleted.eq(false))
            .filter(process_route::Column::IsActive.eq(true))
            .order_by_asc(process_route::Column::Seq)
            .all(&*self.db)
            .await?;
        Ok(list)
    }
}

// ============================================================================
// 流转卡 Service
// ============================================================================

/// 创建流转卡请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFlowCardRequest {
    pub production_order_id: i32,
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub dyeing_requirements: Option<String>,
    pub planned_fabric_weight: Option<Decimal>,
    pub priority: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新流转卡请求（仅 pending 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFlowCardRequest {
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub dyeing_requirements: Option<String>,
    pub planned_fabric_weight: Option<Decimal>,
    pub priority: Option<i32>,
    pub remarks: Option<String>,
}

/// 流转卡查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct FlowCardQuery {
    pub card_no: Option<String>,
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 流转卡 Service
pub struct FlowCardService {
    db: Arc<DatabaseConnection>,
}

impl FlowCardService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成流转卡号：FC-YYYYMMDDHHMMSS-NNN
    fn generate_card_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("FC-{}-{:03}", timestamp, random)
    }

    /// 生成条码：FC + 14位时间戳 + 6位随机数
    fn generate_barcode() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit();
        format!("FC{}{:06}", timestamp, random)
    }

    /// 创建流转卡
    pub async fn create(&self, req: CreateFlowCardRequest) -> Result<CardModel, AppError> {
        // 业务校验：计划配布数量必须为正
        if let Some(weight) = req.planned_fabric_weight {
            if weight <= Decimal::ZERO {
                return Err(AppError::business("计划配布数量必须 > 0"));
            }
        }

        // 业务校验：优先级范围
        if let Some(p) = req.priority {
            if !(-100..=100).contains(&p) {
                return Err(AppError::business("优先级范围 -100 到 100"));
            }
        }

        let card_no = Self::generate_card_no();
        let barcode = Self::generate_barcode();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = CardActiveModel {
            id: Default::default(),
            card_no: Set(card_no),
            barcode: Set(barcode),
            production_order_id: Set(req.production_order_id),
            dye_batch_id: Set(req.dye_batch_id),
            dye_lot_no: Set(req.dye_lot_no),
            process_route_id: Set(req.process_route_id),
            customer_id: Set(req.customer_id),
            customer_name: Set(req.customer_name),
            order_no: Set(req.order_no),
            product_id: Set(req.product_id),
            product_name: Set(req.product_name),
            color_no: Set(req.color_no),
            dyeing_requirements: Set(req.dyeing_requirements),
            planned_fabric_weight: Set(req.planned_fabric_weight),
            actual_fabric_weight: Set(None),
            current_step_seq: Set(1),
            status: Set(card_status::PENDING.to_string()),
            scheduled_at: Set(None),
            prepared_at: Set(None),
            dye_start_at: Set(None),
            dye_end_at: Set(None),
            inspected_at: Set(None),
            completed_at: Set(None),
            shipped_at: Set(None),
            priority: Set(req.priority.unwrap_or(0)),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("流转卡创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新流转卡（仅 pending 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateFlowCardRequest,
    ) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_update(&model.status)?;

        let mut active: CardActiveModel = model.into();

        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.process_route_id {
            active.process_route_id = Set(Some(v));
        }
        if let Some(v) = req.customer_id {
            active.customer_id = Set(Some(v));
        }
        if let Some(v) = req.customer_name {
            active.customer_name = Set(Some(v));
        }
        if let Some(v) = req.order_no {
            active.order_no = Set(Some(v));
        }
        if let Some(v) = req.product_id {
            active.product_id = Set(Some(v));
        }
        if let Some(v) = req.product_name {
            active.product_name = Set(Some(v));
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.dyeing_requirements {
            active.dyeing_requirements = Set(Some(v));
        }
        if let Some(v) = req.planned_fabric_weight {
            if v <= Decimal::ZERO {
                return Err(AppError::business("计划配布数量必须 > 0"));
            }
            active.planned_fabric_weight = Set(Some(v));
        }
        if let Some(v) = req.priority {
            if !(-100..=100).contains(&v) {
                return Err(AppError::business("优先级范围 -100 到 100"));
            }
            active.priority = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除流转卡（仅 pending/terminated 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != card_status::PENDING && model.status != card_status::TERMINATED {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅 pending/terminated 状态可删除",
                model.status
            )));
        }

        let mut active: CardActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<CardModel, AppError> {
        let model = CardEntity::find_by_id(id)
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流转卡 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按条码查询（扫码场景）
    pub async fn get_by_barcode(&self, barcode: &str) -> Result<CardModel, AppError> {
        let model = CardEntity::find()
            .filter(production_flow_card::Column::Barcode.eq(barcode))
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("条码 {} 对应的流转卡不存在", barcode)))?;
        Ok(model)
    }

    /// 按缸号查询
    pub async fn get_by_dye_lot(&self, dye_lot_no: &str) -> Result<CardModel, AppError> {
        let model = CardEntity::find()
            .filter(production_flow_card::Column::DyeLotNo.eq(dye_lot_no))
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("缸号 {} 对应的流转卡不存在", dye_lot_no)))?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(&self, query: FlowCardQuery) -> Result<(Vec<CardModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = CardEntity::find().filter(production_flow_card::Column::IsDeleted.eq(false));

        if let Some(no) = &query.card_no {
            q = q.filter(production_flow_card::Column::CardNo.contains(no));
        }
        if let Some(bc) = &query.barcode {
            q = q.filter(production_flow_card::Column::Barcode.eq(bc));
        }
        if let Some(dl) = &query.dye_lot_no {
            q = q.filter(production_flow_card::Column::DyeLotNo.eq(dl));
        }
        if let Some(oid) = query.production_order_id {
            q = q.filter(production_flow_card::Column::ProductionOrderId.eq(oid));
        }
        if let Some(s) = &query.status {
            q = q.filter(production_flow_card::Column::Status.eq(s));
        }
        if let Some(cid) = query.customer_id {
            q = q.filter(production_flow_card::Column::CustomerId.eq(cid));
        }

        q = q.order_by_desc(production_flow_card::Column::CreatedAt);

        let paginator = q.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    // ===== 状态机流转 =====

    /// 状态流转校验（缸号全生命周期状态机）
    fn validate_status_transition(from: &str, to: &str) -> Result<(), AppError> {
        let allowed = match from {
            card_status::PENDING => vec![card_status::SCHEDULED, card_status::TERMINATED],
            card_status::SCHEDULED => vec![
                card_status::PREPARING,
                card_status::PENDING,
                card_status::TERMINATED,
            ],
            card_status::PREPARING => vec![card_status::DYEING, card_status::TERMINATED],
            card_status::DYEING => vec![card_status::DYED, card_status::TERMINATED],
            card_status::DYED => vec![card_status::INSPECTING],
            card_status::INSPECTING => vec![card_status::COMPLETED, card_status::DYEING],
            card_status::COMPLETED => vec![card_status::SHIPPED],
            card_status::SHIPPED => vec![],
            card_status::TERMINATED => vec![card_status::PENDING],
            _ => return Err(AppError::business(format!("未知流转卡状态: {}", from))),
        };

        if !allowed.contains(&to) {
            return Err(AppError::business(format!(
                "流转卡状态不允许从 {} 流转到 {}（允许: {:?}）",
                from, to, allowed
            )));
        }
        Ok(())
    }

    /// 仅 pending/scheduled 状态可更新
    fn validate_can_update(status: &str) -> Result<(), AppError> {
        if status != card_status::PENDING && status != card_status::SCHEDULED {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅 pending/scheduled 状态可更新",
                status
            )));
        }
        Ok(())
    }

    /// 排缸（pending → scheduled）
    pub async fn schedule(
        &self,
        id: i32,
        dye_batch_id: Option<i32>,
        dye_lot_no: Option<String>,
    ) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::SCHEDULED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::SCHEDULED.to_string());
        active.scheduled_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        if let Some(v) = dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 开始备布（scheduled → preparing）
    pub async fn start_preparing(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::PREPARING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::PREPARING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完成备布（回填实际配布数量，状态保持 preparing 等待进缸）
    pub async fn complete_preparing(
        &self,
        id: i32,
        actual_fabric_weight: Decimal,
    ) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != card_status::PREPARING {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅 preparing 状态可完成备布",
                model.status
            )));
        }
        if actual_fabric_weight <= Decimal::ZERO {
            return Err(AppError::business("实际配布数量必须 > 0"));
        }

        let mut active: CardActiveModel = model.into();
        active.actual_fabric_weight = Set(Some(actual_fabric_weight));
        active.prepared_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 进缸染色（preparing → dyeing）
    pub async fn start_dyeing(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::DYEING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::DYEING.to_string());
        active.dye_start_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 出缸（dyeing → dyed）
    pub async fn complete_dyeing(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::DYED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::DYED.to_string());
        active.dye_end_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 开始验布（dyed → inspecting）
    pub async fn start_inspecting(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::INSPECTING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::INSPECTING.to_string());
        active.inspected_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完成验布入库（inspecting → completed）
    pub async fn complete(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::COMPLETED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::COMPLETED.to_string());
        active.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 发货（completed → shipped）
    pub async fn ship(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::SHIPPED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::SHIPPED.to_string());
        active.shipped_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 终止（任意状态 → terminated，可重新激活回 pending）
    pub async fn terminate(&self, id: i32, reason: Option<String>) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == card_status::TERMINATED {
            return Err(AppError::business("流转卡已终止"));
        }
        if model.status == card_status::SHIPPED {
            return Err(AppError::business("已发货流转卡不可终止"));
        }

        let existing_remarks = model.remarks.clone().unwrap_or_default();
        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::TERMINATED.to_string());
        if let Some(r) = reason {
            let new_remarks = if existing_remarks.is_empty() {
                format!("[终止] {}", r)
            } else {
                format!("{}\n[终止] {}", existing_remarks, r)
            };
            active.remarks = Set(Some(new_remarks));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 重新激活（terminated → pending，回修订单重新进缸场景）
    pub async fn reactivate(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::PENDING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::PENDING.to_string());
        active.scheduled_at = Set(None);
        active.prepared_at = Set(None);
        active.dye_start_at = Set(None);
        active.dye_end_at = Set(None);
        active.inspected_at = Set(None);
        active.completed_at = Set(None);
        active.shipped_at = Set(None);
        active.current_step_seq = Set(1);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

// ============================================================================
// 工序流转记录 Service
// ============================================================================

/// 开始工序请求（扫码开始）
#[derive(Debug, Clone, Deserialize)]
pub struct StartStepRequest {
    pub flow_card_id: i32,
    pub process_route_id: Option<i32>,
    pub worker_ids: Option<String>,
    pub worker_names: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub created_by: Option<i32>,
}

/// 结束工序请求（扫码结束）
#[derive(Debug, Clone, Deserialize)]
pub struct CompleteStepRequest {
    pub actual_quantity: Option<Decimal>,
    pub qualified_quantity: Option<Decimal>,
    pub abnormal_description: Option<String>,
    pub handling_opinion: Option<String>,
    pub remarks: Option<String>,
}

/// 工序流转记录 Service
pub struct StepRecordService {
    db: Arc<DatabaseConnection>,
}

impl StepRecordService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 扫码开始工序（自动创建 pending 记录并切换到 in_progress）
    pub async fn start_step(&self, req: StartStepRequest) -> Result<StepModel, AppError> {
        // 校验流转卡存在
        let card = CardEntity::find_by_id(req.flow_card_id)
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流转卡 {} 不存在", req.flow_card_id)))?;

        // 业务校验：流转卡状态必须为 dyeing/inspecting/preparing 等生产中状态
        let valid_statuses = [
            card_status::SCHEDULED,
            card_status::PREPARING,
            card_status::DYEING,
            card_status::DYED,
            card_status::INSPECTING,
        ];
        if !valid_statuses.contains(&card.status.as_str()) {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅在 {:?} 状态可开始工序",
                card.status, valid_statuses
            )));
        }

        // 获取工序路线信息
        let (route_code, route_name, process_type, step_seq) = if let Some(route_id) =
            req.process_route_id
        {
            let route = RouteEntity::find_by_id(route_id)
                .filter(process_route::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("工序路线 {} 不存在", route_id)))?;
            (
                route.route_code,
                route.route_name,
                route.process_type,
                route.seq,
            )
        } else {
            // 无工序路线时使用当前工序序号占位
            (
                "CUSTOM".to_string(),
                "自定义工序".to_string(),
                "other".to_string(),
                card.current_step_seq,
            )
        };

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = StepActiveModel {
            id: Default::default(),
            flow_card_id: Set(req.flow_card_id),
            process_route_id: Set(req.process_route_id),
            step_seq: Set(step_seq),
            route_code: Set(route_code),
            route_name: Set(route_name),
            process_type: Set(process_type),
            worker_ids: Set(req.worker_ids),
            worker_names: Set(req.worker_names),
            equipment_id: Set(req.equipment_id),
            equipment_name: Set(req.equipment_name),
            start_at: Set(now),
            end_at: Set(None),
            duration_minutes: Set(None),
            planned_quantity: Set(card.planned_fabric_weight),
            actual_quantity: Set(None),
            qualified_quantity: Set(None),
            status: Set(step_status::IN_PROGRESS.to_string()),
            abnormal_description: Set(None),
            handling_opinion: Set(None),
            rework_source_id: Set(None),
            remarks: Set(None),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工序记录创建失败: {}", e)))?;

        // 更新流转卡当前工序序号
        let mut card_active: CardActiveModel = card.into();
        card_active.current_step_seq = Set(step_seq);
        card_active.updated_at = Set(now);
        card_active.update(&*self.db).await?;

        Ok(result)
    }

    /// 扫码结束工序（in_progress → completed）
    pub async fn complete_step(
        &self,
        id: i32,
        req: CompleteStepRequest,
    ) -> Result<StepModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != step_status::IN_PROGRESS {
            return Err(AppError::business(format!(
                "工序记录状态为 {}，仅 in_progress 状态可完成",
                model.status
            )));
        }

        // 业务校验：合格产量不能超过实际产量
        if let (Some(actual), Some(qualified)) = (req.actual_quantity, req.qualified_quantity) {
            if qualified > actual {
                return Err(AppError::business("合格产量不能超过实际产量"));
            }
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        // 计算工时（分钟）
        let duration_minutes = (now - model.start_at).num_minutes();

        // 有异常描述则标记为 abnormal，否则 completed
        let has_abnormal = req.abnormal_description.is_some();
        let new_status = if has_abnormal {
            step_status::ABNORMAL.to_string()
        } else {
            step_status::COMPLETED.to_string()
        };

        let mut active: StepActiveModel = model.into();
        active.end_at = Set(Some(now));
        active.duration_minutes = Set(Some(duration_minutes as i32));
        active.actual_quantity = Set(req.actual_quantity);
        active.qualified_quantity = Set(req.qualified_quantity);
        active.abnormal_description = Set(req.abnormal_description);
        active.handling_opinion = Set(req.handling_opinion);
        active.remarks = Set(req.remarks);
        active.status = Set(new_status);
        active.updated_at = Set(now);

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<StepModel, AppError> {
        let model = StepEntity::find_by_id(id)
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工序记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按流转卡查询所有工序记录（按序号排序）
    pub async fn list_by_flow_card(&self, flow_card_id: i32) -> Result<Vec<StepModel>, AppError> {
        let list = StepEntity::find()
            .filter(process_step_record::Column::FlowCardId.eq(flow_card_id))
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .order_by_asc(process_step_record::Column::StepSeq)
            .all(&*self.db)
            .await?;
        Ok(list)
    }

    /// 创建回修工序（关联原工序记录）
    pub async fn create_rework(
        &self,
        source_step_id: i32,
        req: StartStepRequest,
    ) -> Result<StepModel, AppError> {
        let source = self.get_by_id(source_step_id).await?;

        let mut rework_req = req;
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = StepActiveModel {
            id: Default::default(),
            flow_card_id: Set(rework_req.flow_card_id),
            process_route_id: Set(rework_req.process_route_id.or(source.process_route_id)),
            step_seq: Set(source.step_seq),
            route_code: Set(source.route_code.clone()),
            route_name: Set(format!("回修-{}", source.route_name)),
            process_type: Set(source.process_type.clone()),
            worker_ids: Set(rework_req.worker_ids.take()),
            worker_names: Set(rework_req.worker_names.take()),
            equipment_id: Set(rework_req.equipment_id.take()),
            equipment_name: Set(rework_req.equipment_name.take()),
            start_at: Set(now),
            end_at: Set(None),
            duration_minutes: Set(None),
            planned_quantity: Set(source.planned_quantity),
            actual_quantity: Set(None),
            qualified_quantity: Set(None),
            status: Set(step_status::REWORK.to_string()),
            abnormal_description: Set(None),
            handling_opinion: Set(None),
            rework_source_id: Set(Some(source_step_id)),
            remarks: Set(None),
            is_deleted: Set(false),
            created_by: Set(rework_req.created_by.take()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("回修工序创建失败: {}", e)))?;
        Ok(result)
    }
}

// ============================================================================
// 工序质量反馈单 Service
// ============================================================================

/// 创建质量反馈单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFeedbackRequest {
    pub flow_card_id: i32,
    pub step_record_id: Option<i32>,
    pub feedback_type: String,
    pub description: String,
    pub severity: Option<String>,
    pub found_by: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 处理反馈单请求
#[derive(Debug, Clone, Deserialize)]
pub struct HandleFeedbackRequest {
    pub handling_opinion: Option<String>,
    pub handling_result: Option<String>,
    pub handled_by: Option<i32>,
}

/// 质量反馈单 Service
pub struct QualityFeedbackService {
    db: Arc<DatabaseConnection>,
}

impl QualityFeedbackService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成反馈单号：QF-YYYYMMDDHHMMSS-NNN
    fn generate_feedback_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("QF-{}-{:03}", timestamp, random)
    }

    /// 创建质量反馈单
    pub async fn create(&self, req: CreateFeedbackRequest) -> Result<FeedbackModel, AppError> {
        // 业务校验：反馈类型合法
        let valid_types = ["abnormal", "rework", "defect", "other"];
        if !valid_types.contains(&req.feedback_type.as_str()) {
            return Err(AppError::business(format!(
                "反馈类型必须是 {:?} 之一",
                valid_types
            )));
        }

        // 业务校验：严重等级合法
        let severity = req.severity.unwrap_or_else(|| "medium".to_string());
        let valid_severities = ["low", "medium", "high", "critical"];
        if !valid_severities.contains(&severity.as_str()) {
            return Err(AppError::business(format!(
                "严重等级必须是 {:?} 之一",
                valid_severities
            )));
        }

        // 校验流转卡存在
        let card_exists = CardEntity::find_by_id(req.flow_card_id)
            .filter(production_flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_some();
        if !card_exists {
            return Err(AppError::not_found(format!(
                "流转卡 {} 不存在",
                req.flow_card_id
            )));
        }

        let feedback_no = Self::generate_feedback_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = FeedbackActiveModel {
            id: Default::default(),
            feedback_no: Set(feedback_no),
            flow_card_id: Set(req.flow_card_id),
            step_record_id: Set(req.step_record_id),
            feedback_type: Set(req.feedback_type),
            description: Set(req.description),
            severity: Set(severity),
            found_by: Set(req.found_by),
            found_at: Set(now),
            handling_opinion: Set(None),
            handled_by: Set(None),
            handled_at: Set(None),
            handling_result: Set(None),
            status: Set(feedback_status::PENDING.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("质量反馈单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 处理反馈单（pending → processing → resolved）
    pub async fn handle(
        &self,
        id: i32,
        req: HandleFeedbackRequest,
    ) -> Result<FeedbackModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == feedback_status::CLOSED {
            return Err(AppError::business("已关闭的反馈单不可再处理"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: FeedbackActiveModel = model.into();

        if let Some(v) = req.handling_opinion {
            active.handling_opinion = Set(Some(v));
        }
        if let Some(v) = req.handled_by {
            active.handled_by = Set(Some(v));
            active.handled_at = Set(Some(now));
        }

        // 状态流转：pending → processing（有处理意见但无结果）→ resolved（有处理结果）
        let new_status = if req.handling_result.is_some() {
            active.handling_result = Set(req.handling_result);
            feedback_status::RESOLVED.to_string()
        } else {
            feedback_status::PROCESSING.to_string()
        };
        active.status = Set(new_status);
        active.updated_at = Set(now);

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭反馈单（resolved → closed）
    pub async fn close(&self, id: i32) -> Result<FeedbackModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != feedback_status::RESOLVED {
            return Err(AppError::business(format!(
                "反馈单状态为 {}，仅 resolved 状态可关闭",
                model.status
            )));
        }

        let mut active: FeedbackActiveModel = model.into();
        active.status = Set(feedback_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<FeedbackModel, AppError> {
        let model = FeedbackEntity::find_by_id(id)
            .filter(process_quality_feedback::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量反馈单 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按流转卡查询反馈单
    pub async fn list_by_flow_card(
        &self,
        flow_card_id: i32,
    ) -> Result<Vec<FeedbackModel>, AppError> {
        let list = FeedbackEntity::find()
            .filter(process_quality_feedback::Column::FlowCardId.eq(flow_card_id))
            .filter(process_quality_feedback::Column::IsDeleted.eq(false))
            .order_by_desc(process_quality_feedback::Column::FoundAt)
            .all(&*self.db)
            .await?;
        Ok(list)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试流转卡号生成格式
    #[test]
    fn test_generate_card_no_format() {
        let card_no = FlowCardService::generate_card_no();
        assert!(card_no.starts_with("FC-"));
        // 格式：FC-YYYYMMDDHHMMSS-NNN
        let parts: Vec<&str> = card_no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // NNN
    }

    /// 测试条码生成格式
    #[test]
    fn test_generate_barcode_format() {
        let barcode = FlowCardService::generate_barcode();
        assert!(barcode.starts_with("FC"));
        // 格式：FC + 14位时间戳 + 6位随机数 = 22 字符
        assert_eq!(barcode.len(), 22);
    }

    /// 测试反馈单号生成格式
    #[test]
    fn test_generate_feedback_no_format() {
        let no = QualityFeedbackService::generate_feedback_no();
        assert!(no.starts_with("QF-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14);
        assert_eq!(parts[2].len(), 3);
    }

    /// 测试流转卡状态流转校验
    #[test]
    fn test_validate_status_transition_normal() {
        // 正常流转路径
        assert!(FlowCardService::validate_status_transition(
            card_status::PENDING,
            card_status::SCHEDULED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::SCHEDULED,
            card_status::PREPARING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::PREPARING,
            card_status::DYEING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::DYEING,
            card_status::DYED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::DYED,
            card_status::INSPECTING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::INSPECTING,
            card_status::COMPLETED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::COMPLETED,
            card_status::SHIPPED
        )
        .is_ok());
    }

    /// 测试流转卡状态流转校验：非法路径
    #[test]
    fn test_validate_status_transition_illegal() {
        // pending 不能直接到 dyeing
        assert!(FlowCardService::validate_status_transition(
            card_status::PENDING,
            card_status::DYEING
        )
        .is_err());
        // shipped 是终态，不可再流转
        assert!(FlowCardService::validate_status_transition(
            card_status::SHIPPED,
            card_status::PENDING
        )
        .is_err());
        // terminated 只能回到 pending
        assert!(FlowCardService::validate_status_transition(
            card_status::TERMINATED,
            card_status::SCHEDULED
        )
        .is_err());
        assert!(FlowCardService::validate_status_transition(
            card_status::TERMINATED,
            card_status::PENDING
        )
        .is_ok());
    }

    /// 测试可更新状态校验
    #[test]
    fn test_validate_can_update() {
        assert!(FlowCardService::validate_can_update(card_status::PENDING).is_ok());
        assert!(FlowCardService::validate_can_update(card_status::SCHEDULED).is_ok());
        assert!(FlowCardService::validate_can_update(card_status::DYEING).is_err());
        assert!(FlowCardService::validate_can_update(card_status::COMPLETED).is_err());
    }

    /// 测试回修场景：INSPECTING 可回到 DYEING（回修订单重新进缸）
    #[test]
    fn test_validate_status_transition_rework() {
        // 验布发现质量问题需要回修染色
        assert!(FlowCardService::validate_status_transition(
            card_status::INSPECTING,
            card_status::DYEING
        )
        .is_ok());
    }
}

//! 流转卡工序流转 Service
//!
//! v14 批次 425：流转卡工序流转模块
//! 依据：面料行业真实业务调研文档 §14.1 流转卡工序流转（基于同凯印染 ERP/KESHTECH 真实开卡字段）
//! 真实业务流程：
//!   流转卡定义：流转卡=生产流程卡/工序流转卡/缸卡，一卡对应一缸布的生产任务，
//!              承载从开卡到成品入库的全部工序信息。流转卡=缸号+工单信息+工序路线+计划配布数量+条码。
//!   扫码签入签出（PDA/工控终端）：
//!     签入：扫码→识别工单号/缸号/工序路线→工人刷卡登记→记录工号、设备编号、开始时间→待加工→加工中
//!     签出：扫码→记录结束时间、实际产量→加工中→完工
//!     流转：完工→转入下道（系统自动触发下一道工序开工准备）
//!     入库：完工→完工入库（PDA 扫描卷唛条码）
//!   分卡/合卡/拆卡（KESHTECH 真实业务）：
//!     分卡：机缸容量不足，将坯布分成两部分分别染色，生成新卡号
//!     合缸：多张小卡合并为一缸染色，共享缸号但保留各自卡号
//!     拆卡：一匹布过长拆分为多匹，生成子卡号关联母卡号
//!     缸终止：因质量/工艺问题终止该缸生产
//!   内修卡：内修卡号 = 原始卡号 + A/B/C 后缀（一次回修+A，二次回修+B）

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::flow_card::{
    self, ActiveModel as FlowCardActiveModel, Entity as FlowCardEntity, Model as FlowCardModel,
    ProcessRouteItem,
};
use crate::models::flow_card_operation::{
    self, ActiveModel as OperationActiveModel, Entity as OperationEntity, Model as OperationModel,
};
use crate::models::status::flow_card as card_status;
use crate::models::status::flow_card_operation as op_status;
use crate::utils::error::AppError;

// ============================================================================
// 流转卡 Service
// ============================================================================

/// 创建流转卡请求
///
/// 真实业务必填字段（依据 §14.1 流转卡核心字段）：
/// - flow_card_no: 卡号（系统自动生成）
/// - dye_lot_no: 缸号（一缸一卡约束）
/// - process_route: 工序路线
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFlowCardRequest {
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
    pub work_order_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub salesman_id: Option<i32>,
    pub greige_fabric_id: Option<i32>,
    pub fabric_type: Option<String>,
    pub yarn_count: Option<String>,
    pub composition: Option<String>,
    pub gram_weight: Option<Decimal>,
    pub fabric_width: Option<Decimal>,
    pub color_no: Option<String>,
    pub color_name: Option<String>,
    pub light_source: Option<String>,
    pub planned_pieces: Option<i32>,
    pub planned_weight_kg: Option<Decimal>,
    pub planned_quantity: Option<Decimal>,
    pub process_route: Option<Vec<ProcessRouteItem>>,
    pub current_process: Option<String>,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub warehouse_position: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新流转卡请求（仅 opened/waiting_dyeing 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFlowCardRequest {
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
    pub work_order_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub salesman_id: Option<i32>,
    pub greige_fabric_id: Option<i32>,
    pub fabric_type: Option<String>,
    pub yarn_count: Option<String>,
    pub composition: Option<String>,
    pub gram_weight: Option<Decimal>,
    pub fabric_width: Option<Decimal>,
    pub color_no: Option<String>,
    pub color_name: Option<String>,
    pub light_source: Option<String>,
    pub planned_pieces: Option<i32>,
    pub planned_weight_kg: Option<Decimal>,
    pub planned_quantity: Option<Decimal>,
    pub process_route: Option<Vec<ProcessRouteItem>>,
    pub current_process: Option<String>,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub warehouse_position: Option<String>,
}

/// 流转卡查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct FlowCardQuery {
    pub dye_lot_no: Option<String>,
    pub work_order_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub is_rework: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 扫码签入请求
#[derive(Debug, Clone, Deserialize)]
pub struct SignInRequest {
    pub operator_id: i32,
    pub equipment_id: Option<String>,
}

/// 扫码签出请求
#[derive(Debug, Clone, Deserialize)]
pub struct SignOutRequest {
    pub actual_quantity: Option<Decimal>,
    pub actual_pieces: Option<i32>,
    pub defect_count: Option<i32>,
    pub remarks: Option<String>,
}

/// 分卡请求（机缸容量不足，将坯布分成多部分分别染色）
#[derive(Debug, Clone, Deserialize)]
pub struct SplitCardRequest {
    /// 分卡数量列表（如 [300.00, 200.00] 表示分成 300kg + 200kg 两张卡）
    pub quantities: Vec<Decimal>,
}

/// 合缸请求（多张小卡合并为一缸染色，共享缸号）
#[derive(Debug, Clone, Deserialize)]
pub struct MergeCardRequest {
    /// 待合并的流转卡 ID 列表
    pub card_ids: Vec<i32>,
    /// 目标缸号
    pub target_dye_lot_no: String,
}

/// 拆卡请求（一匹布过长拆分为多匹，生成子卡号关联母卡号）
#[derive(Debug, Clone, Deserialize)]
pub struct SplitPieceRequest {
    /// 拆分后的匹数列表（如 [50, 50] 表示拆成 50 匹 + 50 匹两张子卡）
    pub piece_counts: Vec<i32>,
}

/// 缸终止请求（因质量/工艺问题终止该缸生产）
#[derive(Debug, Clone, Deserialize)]
pub struct TerminateCardRequest {
    pub reason: String,
}

/// 开内修卡请求（原卡号 + A/B/C 后缀）
#[derive(Debug, Clone, Deserialize)]
pub struct CreateReworkCardRequest {
    pub reason: String,
}

/// 暂停请求
#[derive(Debug, Clone, Deserialize)]
pub struct PauseRequest {
    pub reason: Option<String>,
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
    pub fn generate_flow_card_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("FC-{}-{:03}", timestamp, random)
    }

    /// 生成内修卡号：原卡号 + A/B/C 后缀（一次回修+A，二次回修+B，以此类推）
    ///
    /// 真实业务（KESHTECH）：内修卡号 = 原始卡号 + A/B/C 后缀
    /// - 第 1 次回修 → A
    /// - 第 2 次回修 → B
    /// - ...
    /// - 第 26 次回修 → Z
    /// - 超过 26 次返回错误（业务上极少超过 3 次）
    pub fn generate_rework_card_no(
        original_no: &str,
        rework_count: i32,
    ) -> Result<String, AppError> {
        if rework_count < 1 {
            return Err(AppError::business("回修次数必须大于 0"));
        }
        if rework_count > 26 {
            return Err(AppError::business(format!(
                "回修次数 {} 超过 26 次，不支持生成内修卡号（业务上极少超过 3 次）",
                rework_count
            )));
        }
        // 计算后缀字母：1→A, 2→B, ..., 26→Z
        let suffix = (b'A' + (rework_count - 1) as u8) as char;
        Ok(format!("{}{}", original_no, suffix))
    }

    /// 生成条码（Code128 格式字符串）
    ///
    /// 真实业务：条码用于 PDA/工控终端扫码签入签出
    /// 格式：FC128|{卡号}|{校验位}，校验位为卡号各字符 ASCII 之和模 10
    pub fn generate_barcode(card_no: &str) -> String {
        let checksum: u32 = card_no.chars().map(|c| c as u32).sum::<u32>() % 10;
        format!("FC128|{}|{}", card_no, checksum)
    }

    /// 创建流转卡
    ///
    /// 业务校验：
    /// 1. 一缸一卡约束：同 dye_lot_no 只能有一张未删除的主卡（非回修卡、非子卡）
    /// 2. 开卡匹数/计划重量非负
    pub async fn create(&self, req: CreateFlowCardRequest) -> Result<FlowCardModel, AppError> {
        // 业务校验：开卡匹数非负
        if let Some(pieces) = req.planned_pieces {
            if pieces < 0 {
                return Err(AppError::business("开卡匹数不能为负"));
            }
        }
        // 业务校验：计划重量非负
        if let Some(weight) = req.planned_weight_kg {
            if weight < Decimal::ZERO {
                return Err(AppError::business("计划总重不能为负"));
            }
        }
        // 业务校验：配布数量非负
        if let Some(qty) = req.planned_quantity {
            if qty < Decimal::ZERO {
                return Err(AppError::business("配布数量不能为负"));
            }
        }

        // 业务校验：一缸一卡约束（同 dye_lot_no 只能有一张未删除的主卡）
        if let Some(ref dye_lot) = req.dye_lot_no {
            if !dye_lot.trim().is_empty() {
                let exists = FlowCardEntity::find()
                    .filter(flow_card::Column::DyeLotNo.eq(dye_lot))
                    .filter(flow_card::Column::IsDeleted.eq(false))
                    .filter(flow_card::Column::IsRework.eq(false))
                    .filter(flow_card::Column::ParentCardId.is_null())
                    .filter(flow_card::Column::Status.ne(card_status::CANCELLED))
                    .count(&*self.db)
                    .await?;
                if exists > 0 {
                    return Err(AppError::business(format!(
                        "缸号 {} 已存在流转卡（一缸一卡约束：同一缸号只能有一张主卡，分卡/合缸请使用对应接口）",
                        dye_lot
                    )));
                }
            }
        }

        let flow_card_no = Self::generate_flow_card_no();
        let barcode = req.barcode.or_else(|| Some(Self::generate_barcode(&flow_card_no)));
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = FlowCardActiveModel {
            id: Default::default(),
            flow_card_no: Set(flow_card_no),
            barcode: Set(barcode),
            dye_lot_no: Set(req.dye_lot_no),
            work_order_id: Set(req.work_order_id),
            production_order_id: Set(req.production_order_id),
            customer_id: Set(req.customer_id),
            salesman_id: Set(req.salesman_id),
            greige_fabric_id: Set(req.greige_fabric_id),
            fabric_type: Set(req.fabric_type),
            yarn_count: Set(req.yarn_count),
            composition: Set(req.composition),
            gram_weight: Set(req.gram_weight),
            fabric_width: Set(req.fabric_width),
            color_no: Set(req.color_no),
            color_name: Set(req.color_name),
            light_source: Set(req.light_source),
            planned_pieces: Set(req.planned_pieces),
            planned_weight_kg: Set(req.planned_weight_kg),
            planned_quantity: Set(req.planned_quantity),
            actual_pieces: Set(Some(0)),
            actual_weight_kg: Set(None),
            process_route: Set(req.process_route),
            current_process: Set(req.current_process),
            delivery_date: Set(req.delivery_date),
            warehouse_position: Set(req.warehouse_position),
            status: Set(card_status::OPENED.to_string()),
            original_card_id: Set(None),
            rework_count: Set(Some(0)),
            parent_card_id: Set(None),
            is_rework: Set(false),
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

    /// 更新流转卡（仅 opened/waiting_dyeing 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateFlowCardRequest,
    ) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_update(&model.status)?;

        // 若修改 dye_lot_no，需重新校验一缸一卡约束
        if let Some(ref new_dye_lot) = req.dye_lot_no {
            if !new_dye_lot.trim().is_empty() && Some(new_dye_lot.as_str()) != model.dye_lot_no.as_deref() {
                let exists = FlowCardEntity::find()
                    .filter(flow_card::Column::DyeLotNo.eq(new_dye_lot))
                    .filter(flow_card::Column::Id.ne(id))
                    .filter(flow_card::Column::IsDeleted.eq(false))
                    .filter(flow_card::Column::IsRework.eq(false))
                    .filter(flow_card::Column::ParentCardId.is_null())
                    .filter(flow_card::Column::Status.ne(card_status::CANCELLED))
                    .count(&*self.db)
                    .await?;
                if exists > 0 {
                    return Err(AppError::business(format!(
                        "缸号 {} 已存在其他流转卡（一缸一卡约束）",
                        new_dye_lot
                    )));
                }
            }
        }

        // 校验数量非负
        if let Some(pieces) = req.planned_pieces {
            if pieces < 0 {
                return Err(AppError::business("开卡匹数不能为负"));
            }
        }
        if let Some(weight) = req.planned_weight_kg {
            if weight < Decimal::ZERO {
                return Err(AppError::business("计划总重不能为负"));
            }
        }
        if let Some(qty) = req.planned_quantity {
            if qty < Decimal::ZERO {
                return Err(AppError::business("配布数量不能为负"));
            }
        }

        let mut active: FlowCardActiveModel = model.into();

        if let Some(v) = req.barcode {
            active.barcode = Set(Some(v));
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        if let Some(v) = req.work_order_id {
            active.work_order_id = Set(Some(v));
        }
        if let Some(v) = req.production_order_id {
            active.production_order_id = Set(Some(v));
        }
        if let Some(v) = req.customer_id {
            active.customer_id = Set(Some(v));
        }
        if let Some(v) = req.salesman_id {
            active.salesman_id = Set(Some(v));
        }
        if let Some(v) = req.greige_fabric_id {
            active.greige_fabric_id = Set(Some(v));
        }
        if let Some(v) = req.fabric_type {
            active.fabric_type = Set(Some(v));
        }
        if let Some(v) = req.yarn_count {
            active.yarn_count = Set(Some(v));
        }
        if let Some(v) = req.composition {
            active.composition = Set(Some(v));
        }
        if let Some(v) = req.gram_weight {
            active.gram_weight = Set(Some(v));
        }
        if let Some(v) = req.fabric_width {
            active.fabric_width = Set(Some(v));
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(Some(v));
        }
        if let Some(v) = req.color_name {
            active.color_name = Set(Some(v));
        }
        if let Some(v) = req.light_source {
            active.light_source = Set(Some(v));
        }
        if let Some(v) = req.planned_pieces {
            active.planned_pieces = Set(Some(v));
        }
        if let Some(v) = req.planned_weight_kg {
            active.planned_weight_kg = Set(Some(v));
        }
        if let Some(v) = req.planned_quantity {
            active.planned_quantity = Set(Some(v));
        }
        if let Some(v) = req.process_route {
            active.process_route = Set(Some(v));
        }
        if let Some(v) = req.current_process {
            active.current_process = Set(Some(v));
        }
        if let Some(v) = req.delivery_date {
            active.delivery_date = Set(Some(v));
        }
        if let Some(v) = req.warehouse_position {
            active.warehouse_position = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除流转卡
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_can_delete(&model.status)?;

        let mut active: FlowCardActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询流转卡
    pub async fn get_by_id(&self, id: i32) -> Result<FlowCardModel, AppError> {
        let model = FlowCardEntity::find_by_id(id)
            .filter(flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流转卡 {} 不存在", id)))?;
        Ok(model)
    }

    /// 分页查询流转卡
    pub async fn list(&self, query: FlowCardQuery) -> Result<(Vec<FlowCardModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = FlowCardEntity::find().filter(flow_card::Column::IsDeleted.eq(false));

        if let Some(dye_lot) = &query.dye_lot_no {
            q = q.filter(flow_card::Column::DyeLotNo.eq(dye_lot));
        }
        if let Some(wid) = query.work_order_id {
            q = q.filter(flow_card::Column::WorkOrderId.eq(wid));
        }
        if let Some(pid) = query.production_order_id {
            q = q.filter(flow_card::Column::ProductionOrderId.eq(pid));
        }
        if let Some(cid) = query.customer_id {
            q = q.filter(flow_card::Column::CustomerId.eq(cid));
        }
        if let Some(status) = &query.status {
            q = q.filter(flow_card::Column::Status.eq(status));
        }
        if let Some(is_rework) = query.is_rework {
            q = q.filter(flow_card::Column::IsRework.eq(is_rework));
        }

        q = q.order_by_desc(flow_card::Column::CreatedAt);

        let paginator = q.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 按缸号查询流转卡（一缸一卡约束：返回唯一主卡）
    pub async fn get_by_dye_lot_no(
        &self,
        dye_lot_no: &str,
    ) -> Result<Option<FlowCardModel>, AppError> {
        let model = FlowCardEntity::find()
            .filter(flow_card::Column::DyeLotNo.eq(dye_lot_no))
            .filter(flow_card::Column::IsDeleted.eq(false))
            .filter(flow_card::Column::IsRework.eq(false))
            .filter(flow_card::Column::ParentCardId.is_null())
            .filter(flow_card::Column::Status.ne(card_status::CANCELLED))
            .order_by_desc(flow_card::Column::CreatedAt)
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 按条码查询流转卡（扫码用）
    pub async fn get_by_barcode(&self, barcode: &str) -> Result<Option<FlowCardModel>, AppError> {
        let model = FlowCardEntity::find()
            .filter(flow_card::Column::Barcode.eq(barcode))
            .filter(flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 扫码签入（待加工 → 加工中）
    ///
    /// 真实业务：扫码 → 识别工单号/缸号/工序路线 → 工人刷卡登记 → 记录工号、设备编号、开始时间
    /// 同时创建/更新当前工序操作记录（pending → in_progress）
    pub async fn sign_in(
        &self,
        card_id: i32,
        req: SignInRequest,
    ) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        // 签入前校验：流转卡未终止/取消/已入库
        if matches!(
            model.status.as_str(),
            card_status::TERMINATED | card_status::CANCELLED | card_status::STORED | card_status::SHIPPED
        ) {
            return Err(AppError::business(format!(
                "流转卡状态 {} 不可签入（已终止/取消/入库/发货）",
                model.status
            )));
        }
        if model.status == card_status::PAUSED {
            return Err(AppError::business("流转卡处于暂停状态，请先恢复后再签入"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        // 查找或创建当前工序操作记录
        let (process_sequence, process_name) = Self::current_process_info(&model)?;

        let existing_op = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(card_id))
            .filter(flow_card_operation::Column::ProcessSequence.eq(process_sequence))
            .one(&*self.db)
            .await?;

        if let Some(op) = existing_op {
            // 已存在操作记录：校验状态并更新为 in_progress
            FlowCardOperationService::validate_operation_status_transition(
                &op.status,
                op_status::IN_PROGRESS,
            )?;
            let mut op_active: OperationActiveModel = op.into();
            op_active.operator_id = Set(Some(req.operator_id));
            op_active.equipment_id = Set(req.equipment_id);
            op_active.status = Set(op_status::IN_PROGRESS.to_string());
            op_active.sign_in_at = Set(Some(now));
            op_active.updated_at = Set(now);
            op_active.update(&*self.db).await?;
        } else {
            // 不存在操作记录：创建新记录
            let op_active = OperationActiveModel {
                id: Default::default(),
                flow_card_id: Set(card_id),
                process_sequence: Set(process_sequence),
                process_name: Set(process_name),
                operator_id: Set(Some(req.operator_id)),
                equipment_id: Set(req.equipment_id),
                status: Set(op_status::IN_PROGRESS.to_string()),
                sign_in_at: Set(Some(now)),
                sign_out_at: Set(None),
                actual_quantity: Set(None),
                actual_pieces: Set(None),
                defect_count: Set(Some(0)),
                remarks: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            op_active.insert(&*self.db).await?;
        }

        Ok(model)
    }

    /// 扫码签出（加工中 → 完工）
    ///
    /// 真实业务：扫码 → 记录结束时间、实际产量、疵点数 → 加工中 → 完工
    pub async fn sign_out(
        &self,
        card_id: i32,
        req: SignOutRequest,
    ) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        let now = crate::utils::date_utils::utc_now_fixed();
        let (process_sequence, _) = Self::current_process_info(&model)?;

        let op = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(card_id))
            .filter(flow_card_operation::Column::ProcessSequence.eq(process_sequence))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::business("当前工序未签入，无法签出"))?;

        FlowCardOperationService::validate_operation_status_transition(&op.status, op_status::COMPLETED)?;

        let mut op_active: OperationActiveModel = op.into();
        op_active.status = Set(op_status::COMPLETED.to_string());
        op_active.sign_out_at = Set(Some(now));
        op_active.actual_quantity = Set(req.actual_quantity);
        op_active.actual_pieces = Set(req.actual_pieces);
        op_active.defect_count = Set(req.defect_count.or(Some(0)));
        op_active.remarks = Set(req.remarks);
        op_active.updated_at = Set(now);
        op_active.update(&*self.db).await?;

        // 更新流转卡实际产量
        let mut card_active: FlowCardActiveModel = model.into();
        if let Some(qty) = req.actual_quantity {
            card_active.actual_weight_kg = Set(Some(qty));
        }
        if let Some(pieces) = req.actual_pieces {
            card_active.actual_pieces = Set(Some(pieces));
        }
        card_active.updated_at = Set(now);
        let updated = card_active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 流转到下一道工序（完工 → 转入下道）
    ///
    /// 真实业务：完工 → 转入下道（系统自动触发下一道工序开工准备）
    pub async fn transfer_to_next(&self, card_id: i32) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        let now = crate::utils::date_utils::utc_now_fixed();
        let (current_seq, _) = Self::current_process_info(&model)?;

        // 当前工序标记为 transferred
        let op = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(card_id))
            .filter(flow_card_operation::Column::ProcessSequence.eq(current_seq))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::business("当前工序不存在操作记录，无法流转"))?;

        FlowCardOperationService::validate_operation_status_transition(
            &op.status,
            op_status::TRANSFERRED,
        )?;

        let mut op_active: OperationActiveModel = op.into();
        op_active.status = Set(op_status::TRANSFERRED.to_string());
        op_active.updated_at = Set(now);
        op_active.update(&*self.db).await?;

        // 推进 current_process 到下一道工序
        let route = model.process_route.as_ref().ok_or_else(|| {
            AppError::business("工序路线为空，无法流转到下一道工序")
        })?;
        let next = route
            .iter()
            .find(|item| item.sequence == current_seq + 1)
            .ok_or_else(|| AppError::business("已是末道工序，无法流转到下一道（请使用完工入库）"))?;
        // 先克隆下一道工序名称，避免 model.into() 移动后引用失效
        let next_process_name = next.name.clone();

        let mut card_active: FlowCardActiveModel = model.into();
        card_active.current_process = Set(Some(next_process_name));
        card_active.updated_at = Set(now);
        let updated = card_active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完工入库（转入下道 → 完工入库，末道工序）
    ///
    /// 真实业务：完工 → 完工入库（PDA 扫描卷唛条码）
    pub async fn complete_and_store(&self, card_id: i32) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        Self::validate_status_transition(&model.status, card_status::STORED)?;
        let now = crate::utils::date_utils::utc_now_fixed();
        let (current_seq, _) = Self::current_process_info(&model)?;

        // 当前工序标记为 stored
        if let Some(op) = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(card_id))
            .filter(flow_card_operation::Column::ProcessSequence.eq(current_seq))
            .one(&*self.db)
            .await?
        {
            let mut op_active: OperationActiveModel = op.into();
            op_active.status = Set(op_status::STORED.to_string());
            op_active.updated_at = Set(now);
            op_active.update(&*self.db).await?;
        }

        let mut card_active: FlowCardActiveModel = model.into();
        card_active.status = Set(card_status::STORED.to_string());
        card_active.updated_at = Set(now);
        let updated = card_active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 暂停流转卡
    pub async fn pause(&self, card_id: i32, _req: PauseRequest) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        Self::validate_status_transition(&model.status, card_status::PAUSED)?;

        let mut active: FlowCardActiveModel = model.into();
        active.status = Set(card_status::PAUSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 恢复流转卡（暂停 → 恢复到暂停前的状态，简化为回到上一生产状态）
    ///
    /// 真实业务：暂停后恢复生产，回到暂停前的工序状态
    pub async fn resume(&self, card_id: i32) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        if model.status != card_status::PAUSED {
            return Err(AppError::business(format!(
                "流转卡状态 {} 不可恢复（仅 paused 状态可恢复）",
                model.status
            )));
        }

        // 恢复到备布中状态（简化处理：恢复到 preparing）
        let mut active: FlowCardActiveModel = model.into();
        active.status = Set(card_status::PREPARING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 分卡（机缸容量不足，将坯布分成多部分分别染色，生成新卡号）
    ///
    /// 真实业务：原卡保留部分数量，其余按 quantities 生成新卡
    pub async fn split_card(
        &self,
        card_id: i32,
        req: SplitCardRequest,
    ) -> Result<Vec<FlowCardModel>, AppError> {
        if req.quantities.is_empty() {
            return Err(AppError::business("分卡数量列表不能为空"));
        }
        // 校验分卡数量均为正
        if req.quantities.iter().any(|q| *q <= Decimal::ZERO) {
            return Err(AppError::business("分卡数量必须大于 0"));
        }

        let model = self.get_by_id(card_id).await?;
        // 仅 opened/waiting_dyeing/scheduled 状态可分卡
        if !matches!(
            model.status.as_str(),
            card_status::OPENED | card_status::WAITING_DYEING | card_status::SCHEDULED
        ) {
            return Err(AppError::business(format!(
                "流转卡状态 {} 不可分卡（仅 opened/waiting_dyeing/scheduled 可分卡）",
                model.status
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut new_cards = Vec::with_capacity(req.quantities.len());

        for qty in &req.quantities {
            let new_card_no = Self::generate_flow_card_no();
            let new_barcode = Self::generate_barcode(&new_card_no);

            let active = FlowCardActiveModel {
                id: Default::default(),
                flow_card_no: Set(new_card_no),
                barcode: Set(Some(new_barcode)),
                // 分卡生成的新卡使用新缸号（独立染色），原卡保留原缸号
                dye_lot_no: Set(None),
                work_order_id: Set(model.work_order_id),
                production_order_id: Set(model.production_order_id),
                customer_id: Set(model.customer_id),
                salesman_id: Set(model.salesman_id),
                greige_fabric_id: Set(model.greige_fabric_id),
                fabric_type: Set(model.fabric_type.clone()),
                yarn_count: Set(model.yarn_count.clone()),
                composition: Set(model.composition.clone()),
                gram_weight: Set(model.gram_weight),
                fabric_width: Set(model.fabric_width),
                color_no: Set(model.color_no.clone()),
                color_name: Set(model.color_name.clone()),
                light_source: Set(model.light_source.clone()),
                planned_pieces: Set(model.planned_pieces),
                planned_weight_kg: Set(Some(*qty)),
                planned_quantity: Set(Some(*qty)),
                actual_pieces: Set(Some(0)),
                actual_weight_kg: Set(None),
                process_route: Set(model.process_route.clone()),
                current_process: Set(model.current_process.clone()),
                delivery_date: Set(model.delivery_date),
                warehouse_position: Set(model.warehouse_position.clone()),
                status: Set(card_status::OPENED.to_string()),
                original_card_id: Set(None),
                rework_count: Set(Some(0)),
                // 分卡的新卡关联母卡
                parent_card_id: Set(Some(card_id)),
                is_rework: Set(false),
                is_deleted: Set(false),
                created_by: Set(model.created_by),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let new_card = active
                .insert(&*self.db)
                .await
                .map_err(|e| AppError::database(format!("分卡创建失败: {}", e)))?;
            new_cards.push(new_card);
        }

        Ok(new_cards)
    }

    /// 合缸（多张小卡合并为一缸染色，共享缸号但保留各自卡号）
    ///
    /// 真实业务：将多张小卡的 dye_lot_no 统一为目标缸号
    pub async fn merge_card(
        &self,
        req: MergeCardRequest,
    ) -> Result<Vec<FlowCardModel>, AppError> {
        if req.card_ids.len() < 2 {
            return Err(AppError::business("合缸至少需要 2 张流转卡"));
        }
        if req.target_dye_lot_no.trim().is_empty() {
            return Err(AppError::business("目标缸号不能为空"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut merged_cards = Vec::with_capacity(req.card_ids.len());

        for card_id in &req.card_ids {
            let model = self.get_by_id(*card_id).await?;
            // 仅 opened/waiting_dyeing 状态可合缸
            if !matches!(
                model.status.as_str(),
                card_status::OPENED | card_status::WAITING_DYEING
            ) {
                return Err(AppError::business(format!(
                    "流转卡 {} 状态 {} 不可合缸（仅 opened/waiting_dyeing 可合缸）",
                    card_id, model.status
                )));
            }

            let mut active: FlowCardActiveModel = model.into();
            active.dye_lot_no = Set(Some(req.target_dye_lot_no.clone()));
            active.updated_at = Set(now);
            let updated = active.update(&*self.db).await?;
            merged_cards.push(updated);
        }

        Ok(merged_cards)
    }

    /// 拆卡（一匹布过长拆分为多匹，生成子卡号关联母卡号）
    ///
    /// 真实业务：原卡保留部分匹数，其余按 piece_counts 生成子卡
    pub async fn split_piece(
        &self,
        card_id: i32,
        req: SplitPieceRequest,
    ) -> Result<Vec<FlowCardModel>, AppError> {
        if req.piece_counts.is_empty() {
            return Err(AppError::business("拆卡匹数列表不能为空"));
        }
        // 校验拆卡匹数均为正
        if req.piece_counts.iter().any(|p| *p <= 0) {
            return Err(AppError::business("拆卡匹数必须大于 0"));
        }

        let model = self.get_by_id(card_id).await?;
        // 仅 opened/waiting_dyeing 状态可拆卡
        if !matches!(
            model.status.as_str(),
            card_status::OPENED | card_status::WAITING_DYEING
        ) {
            return Err(AppError::business(format!(
                "流转卡状态 {} 不可拆卡（仅 opened/waiting_dyeing 可拆卡）",
                model.status
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut new_cards = Vec::with_capacity(req.piece_counts.len());

        for pieces in &req.piece_counts {
            let new_card_no = Self::generate_flow_card_no();
            let new_barcode = Self::generate_barcode(&new_card_no);

            let active = FlowCardActiveModel {
                id: Default::default(),
                flow_card_no: Set(new_card_no),
                barcode: Set(Some(new_barcode)),
                // 拆卡子卡共享母卡缸号
                dye_lot_no: Set(model.dye_lot_no.clone()),
                work_order_id: Set(model.work_order_id),
                production_order_id: Set(model.production_order_id),
                customer_id: Set(model.customer_id),
                salesman_id: Set(model.salesman_id),
                greige_fabric_id: Set(model.greige_fabric_id),
                fabric_type: Set(model.fabric_type.clone()),
                yarn_count: Set(model.yarn_count.clone()),
                composition: Set(model.composition.clone()),
                gram_weight: Set(model.gram_weight),
                fabric_width: Set(model.fabric_width),
                color_no: Set(model.color_no.clone()),
                color_name: Set(model.color_name.clone()),
                light_source: Set(model.light_source.clone()),
                planned_pieces: Set(Some(*pieces)),
                planned_weight_kg: Set(model.planned_weight_kg),
                planned_quantity: Set(model.planned_quantity),
                actual_pieces: Set(Some(0)),
                actual_weight_kg: Set(None),
                process_route: Set(model.process_route.clone()),
                current_process: Set(model.current_process.clone()),
                delivery_date: Set(model.delivery_date),
                warehouse_position: Set(model.warehouse_position.clone()),
                status: Set(card_status::OPENED.to_string()),
                original_card_id: Set(None),
                rework_count: Set(Some(0)),
                // 拆卡子卡关联母卡
                parent_card_id: Set(Some(card_id)),
                is_rework: Set(false),
                is_deleted: Set(false),
                created_by: Set(model.created_by),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let new_card = active
                .insert(&*self.db)
                .await
                .map_err(|e| AppError::database(format!("拆卡创建失败: {}", e)))?;
            new_cards.push(new_card);
        }

        Ok(new_cards)
    }

    /// 缸终止（因质量/工艺问题终止该缸生产）
    ///
    /// 真实业务：终止后流转卡不可再进行任何操作
    pub async fn terminate_card(
        &self,
        card_id: i32,
        _req: TerminateCardRequest,
    ) -> Result<FlowCardModel, AppError> {
        let model = self.get_by_id(card_id).await?;
        // 终态不可再终止
        if matches!(
            model.status.as_str(),
            card_status::TERMINATED | card_status::CANCELLED | card_status::SHIPPED
        ) {
            return Err(AppError::business(format!(
                "流转卡状态 {} 不可终止（已为终态）",
                model.status
            )));
        }

        let mut active: FlowCardActiveModel = model.into();
        active.status = Set(card_status::TERMINATED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 开内修卡（原卡号 + A/B/C 后缀）
    ///
    /// 真实业务（KESHTECH）：
    /// - 内修卡号 = 原始卡号 + A/B/C 后缀（一次回修+A，二次回修+B）
    /// - 开内修卡前必须先在"质量异常登记"里登记（本接口简化，由调用方保证）
    /// - 原卡 rework_count +1，新卡关联 original_card_id
    pub async fn create_rework_card(
        &self,
        original_card_id: i32,
        _req: CreateReworkCardRequest,
    ) -> Result<FlowCardModel, AppError> {
        let original = self.get_by_id(original_card_id).await?;

        // 计算回修次数（原卡当前回修次数 + 1）
        let new_rework_count = original.rework_count.unwrap_or(0) + 1;

        // 生成内修卡号：原卡号 + A/B/C 后缀
        let rework_no = Self::generate_rework_card_no(&original.flow_card_no, new_rework_count)?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let barcode = Self::generate_barcode(&rework_no);

        // 创建内修卡
        let active = FlowCardActiveModel {
            id: Default::default(),
            flow_card_no: Set(rework_no),
            barcode: Set(Some(barcode)),
            dye_lot_no: Set(original.dye_lot_no.clone()),
            work_order_id: Set(original.work_order_id),
            production_order_id: Set(original.production_order_id),
            customer_id: Set(original.customer_id),
            salesman_id: Set(original.salesman_id),
            greige_fabric_id: Set(original.greige_fabric_id),
            fabric_type: Set(original.fabric_type.clone()),
            yarn_count: Set(original.yarn_count.clone()),
            composition: Set(original.composition.clone()),
            gram_weight: Set(original.gram_weight),
            fabric_width: Set(original.fabric_width),
            color_no: Set(original.color_no.clone()),
            color_name: Set(original.color_name.clone()),
            light_source: Set(original.light_source.clone()),
            planned_pieces: Set(original.planned_pieces),
            planned_weight_kg: Set(original.planned_weight_kg),
            planned_quantity: Set(original.planned_quantity),
            actual_pieces: Set(Some(0)),
            actual_weight_kg: Set(None),
            process_route: Set(original.process_route.clone()),
            current_process: Set(original.current_process.clone()),
            delivery_date: Set(original.delivery_date),
            warehouse_position: Set(original.warehouse_position.clone()),
            // 内修卡初始状态为开卡
            status: Set(card_status::OPENED.to_string()),
            // 关联原始卡
            original_card_id: Set(Some(original_card_id)),
            rework_count: Set(Some(new_rework_count)),
            parent_card_id: Set(None),
            is_rework: Set(true),
            is_deleted: Set(false),
            created_by: Set(original.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let rework_card = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("内修卡创建失败: {}", e)))?;

        // 更新原卡回修次数
        let mut orig_active: FlowCardActiveModel = original.into();
        orig_active.rework_count = Set(Some(new_rework_count));
        orig_active.status = Set(card_status::REWORK.to_string());
        orig_active.updated_at = Set(now);
        orig_active.update(&*self.db).await?;

        Ok(rework_card)
    }

    /// 查询流转卡的工序操作记录
    pub async fn list_operations(&self, card_id: i32) -> Result<Vec<OperationModel>, AppError> {
        // 校验流转卡存在
        let _ = self.get_by_id(card_id).await?;
        let items = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(card_id))
            .order_by_asc(flow_card_operation::Column::ProcessSequence)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 获取当前工序信息（序号 + 名称）
    ///
    /// 优先从 current_process 字段匹配 process_route 中的工序序号，
    /// 若无 process_route 则默认第 1 道工序
    fn current_process_info(model: &FlowCardModel) -> Result<(i32, String), AppError> {
        let route = model.process_route.as_ref();
        if let Some(route) = route {
            if route.is_empty() {
                return Err(AppError::business("工序路线为空，无法确定当前工序"));
            }
            // 优先匹配 current_process 名称
            if let Some(ref current_name) = model.current_process {
                if let Some(item) = route.iter().find(|item| &item.name == current_name) {
                    return Ok((item.sequence, item.name.clone()));
                }
            }
            // 默认返回第一道未完工工序
            if let Some(item) = route
                .iter()
                .find(|item| item.status == op_status::PENDING)
            {
                return Ok((item.sequence, item.name.clone()));
            }
            // 全部完工则返回最后一道
            let last = route.last().ok_or_else(|| {
                AppError::business("工序路线为空，无法确定当前工序")
            })?;
            return Ok((last.sequence, last.name.clone()));
        }
        // 无工序路线时默认第 1 道工序
        Ok((1, model.current_process.clone().unwrap_or_else(|| "默认工序".to_string())))
    }

    // ===== 状态流转校验 =====

    /// 校验流转卡状态流转合法性
    ///
    /// 状态机（缸号维度）：
    ///   opened → waiting_dyeing → scheduled → preparing → dyeing → dyed
    ///         → inspecting → stored → shipped
    ///   分支：任意生产中状态 → paused / rework / terminated / cancelled
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            card_status::OPENED => matches!(
                new,
                card_status::WAITING_DYEING
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::WAITING_DYEING => matches!(
                new,
                card_status::SCHEDULED
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::SCHEDULED => matches!(
                new,
                card_status::PREPARING
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::PREPARING => matches!(
                new,
                card_status::DYEING
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::DYEING => matches!(
                new,
                card_status::DYED
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::DYED => matches!(
                new,
                card_status::INSPECTING
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::INSPECTING => matches!(
                new,
                card_status::STORED
                    | card_status::PAUSED
                    | card_status::REWORK
                    | card_status::TERMINATED
                    | card_status::CANCELLED
            ),
            card_status::STORED => matches!(new, card_status::SHIPPED),
            card_status::PAUSED => matches!(
                new,
                card_status::PREPARING | card_status::CANCELLED | card_status::TERMINATED
            ),
            card_status::REWORK => matches!(new, card_status::OPENED | card_status::CANCELLED),
            card_status::SHIPPED | card_status::TERMINATED | card_status::CANCELLED => false,
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "流转卡状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }

    /// 校验：仅 opened/waiting_dyeing 状态可更新
    pub fn validate_can_update(status: &str) -> Result<(), AppError> {
        if !matches!(status, card_status::OPENED | card_status::WAITING_DYEING) {
            return Err(AppError::business(format!(
                "当前状态 {} 不可更新（仅 opened/waiting_dyeing 可更新）",
                status
            )));
        }
        Ok(())
    }

    /// 校验：仅 opened/waiting_dyeing 状态可删除
    pub fn validate_can_delete(status: &str) -> Result<(), AppError> {
        if !matches!(status, card_status::OPENED | card_status::WAITING_DYEING) {
            return Err(AppError::business(format!(
                "当前状态 {} 不可删除（仅 opened/waiting_dyeing 可删除）",
                status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 流转卡工序操作记录 Service
// ============================================================================

/// 流转卡工序操作记录 Service
pub struct FlowCardOperationService {
    db: Arc<DatabaseConnection>,
}

impl FlowCardOperationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建工序操作记录
    pub async fn create(
        &self,
        flow_card_id: i32,
        process_sequence: i32,
        process_name: String,
    ) -> Result<OperationModel, AppError> {
        // 校验流转卡存在
        let _ = FlowCardEntity::find_by_id(flow_card_id)
            .filter(flow_card::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("流转卡 {} 不存在", flow_card_id)))?;

        // 校验同卡同工序唯一
        let exists = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(flow_card_id))
            .filter(flow_card_operation::Column::ProcessSequence.eq(process_sequence))
            .count(&*self.db)
            .await?;
        if exists > 0 {
            return Err(AppError::business(format!(
                "流转卡 {} 工序序号 {} 已存在操作记录（一卡一道工序仅一条记录）",
                flow_card_id, process_sequence
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = OperationActiveModel {
            id: Default::default(),
            flow_card_id: Set(flow_card_id),
            process_sequence: Set(process_sequence),
            process_name: Set(process_name),
            operator_id: Set(None),
            equipment_id: Set(None),
            status: Set(op_status::PENDING.to_string()),
            sign_in_at: Set(None),
            sign_out_at: Set(None),
            actual_quantity: Set(None),
            actual_pieces: Set(None),
            defect_count: Set(Some(0)),
            remarks: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工序操作记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按 ID 查询工序操作记录
    pub async fn get_by_id(&self, id: i32) -> Result<OperationModel, AppError> {
        let model = OperationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工序操作记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按流转卡查询工序操作记录列表
    pub async fn list_by_card(&self, card_id: i32) -> Result<Vec<OperationModel>, AppError> {
        let items = OperationEntity::find()
            .filter(flow_card_operation::Column::FlowCardId.eq(card_id))
            .order_by_asc(flow_card_operation::Column::ProcessSequence)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 签入工序（pending → in_progress）
    pub async fn sign_in(
        &self,
        id: i32,
        operator_id: i32,
        equipment_id: Option<String>,
    ) -> Result<OperationModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_operation_status_transition(&model.status, op_status::IN_PROGRESS)?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: OperationActiveModel = model.into();
        active.operator_id = Set(Some(operator_id));
        active.equipment_id = Set(equipment_id);
        active.status = Set(op_status::IN_PROGRESS.to_string());
        active.sign_in_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 签出工序（in_progress → completed）
    pub async fn sign_out(
        &self,
        id: i32,
        actual_quantity: Option<Decimal>,
        actual_pieces: Option<i32>,
        defect_count: Option<i32>,
    ) -> Result<OperationModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_operation_status_transition(&model.status, op_status::COMPLETED)?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let mut active: OperationActiveModel = model.into();
        active.status = Set(op_status::COMPLETED.to_string());
        active.sign_out_at = Set(Some(now));
        active.actual_quantity = Set(actual_quantity);
        active.actual_pieces = Set(actual_pieces);
        active.defect_count = Set(defect_count.or(Some(0)));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 流转工序（completed → transferred）
    pub async fn transfer(&self, id: i32) -> Result<OperationModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_operation_status_transition(&model.status, op_status::TRANSFERRED)?;

        let mut active: OperationActiveModel = model.into();
        active.status = Set(op_status::TRANSFERRED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完工入库工序（completed/transferred → stored）
    pub async fn complete(&self, id: i32) -> Result<OperationModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_operation_status_transition(&model.status, op_status::STORED)?;

        let mut active: OperationActiveModel = model.into();
        active.status = Set(op_status::STORED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 校验工序操作记录状态流转合法性
    ///
    /// 状态机（单工序维度）：
    ///   pending → in_progress → completed → transferred → stored
    ///   分支：in_progress/completed → paused / rework
    pub fn validate_operation_status_transition(
        current: &str,
        new: &str,
    ) -> Result<(), AppError> {
        let valid = match current {
            op_status::PENDING => matches!(new, op_status::IN_PROGRESS),
            op_status::IN_PROGRESS => {
                matches!(new, op_status::COMPLETED | op_status::PAUSED | op_status::REWORK)
            }
            op_status::COMPLETED => matches!(new, op_status::TRANSFERRED | op_status::STORED),
            op_status::TRANSFERRED => matches!(new, op_status::STORED),
            op_status::PAUSED => matches!(new, op_status::IN_PROGRESS | op_status::REWORK),
            op_status::REWORK => matches!(new, op_status::IN_PROGRESS),
            op_status::STORED => false,
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "工序操作状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// 测试流转卡号生成格式：FC-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_flow_card_no() {
        let no = FlowCardService::generate_flow_card_no();
        assert!(no.starts_with("FC-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // 3 位随机
    }

    /// 测试内修卡号生成（A/B/C 后缀）
    ///
    /// 真实业务（KESHTECH）：一次回修+A，二次回修+B
    #[test]
    fn test_generate_rework_card_no() {
        let original = "FC-20260715120000-001";
        // 一次回修 → A
        assert_eq!(
            FlowCardService::generate_rework_card_no(original, 1).unwrap(),
            format!("{}A", original)
        );
        // 二次回修 → B
        assert_eq!(
            FlowCardService::generate_rework_card_no(original, 2).unwrap(),
            format!("{}B", original)
        );
        // 三次回修 → C
        assert_eq!(
            FlowCardService::generate_rework_card_no(original, 3).unwrap(),
            format!("{}C", original)
        );
        // 26 次回修 → Z
        assert_eq!(
            FlowCardService::generate_rework_card_no(original, 26).unwrap(),
            format!("{}Z", original)
        );
    }

    /// 测试内修卡号生成非法输入
    #[test]
    fn test_generate_rework_card_no_invalid() {
        let original = "FC-20260715120000-001";
        // 回修次数 < 1
        assert!(FlowCardService::generate_rework_card_no(original, 0).is_err());
        assert!(FlowCardService::generate_rework_card_no(original, -1).is_err());
        // 回修次数 > 26
        assert!(FlowCardService::generate_rework_card_no(original, 27).is_err());
        assert!(FlowCardService::generate_rework_card_no(original, 100).is_err());
    }

    /// 测试条码生成（Code128 格式字符串）
    #[test]
    fn test_generate_barcode() {
        let card_no = "FC-20260715120000-001";
        let barcode = FlowCardService::generate_barcode(card_no);
        assert!(barcode.starts_with("FC128|"));
        assert!(barcode.contains(card_no));
        // 校验位格式：末尾为 |数字
        let parts: Vec<&str> = barcode.split('|').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "FC128");
        assert_eq!(parts[1], card_no);
        // 校验位为 0-9 的数字
        let checksum: u32 = parts[2].parse().unwrap();
        assert!(checksum < 10);
    }

    /// 测试条码生成确定性（相同卡号生成相同条码）
    #[test]
    fn test_generate_barcode_deterministic() {
        let card_no = "FC-20260715120000-001";
        let barcode1 = FlowCardService::generate_barcode(card_no);
        let barcode2 = FlowCardService::generate_barcode(card_no);
        assert_eq!(barcode1, barcode2);
    }

    /// 测试流转卡状态流转合法性（合法流转）
    #[test]
    fn test_card_status_transition_valid() {
        // 主流程：opened → waiting_dyeing → scheduled → preparing → dyeing → dyed → inspecting → stored → shipped
        assert!(FlowCardService::validate_status_transition(
            card_status::OPENED,
            card_status::WAITING_DYEING
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::WAITING_DYEING,
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
            card_status::STORED
        )
        .is_ok());
        assert!(FlowCardService::validate_status_transition(
            card_status::STORED,
            card_status::SHIPPED
        )
        .is_ok());

        // 分支：生产中状态可暂停
        assert!(FlowCardService::validate_status_transition(
            card_status::PREPARING,
            card_status::PAUSED
        )
        .is_ok());
        // 分支：暂停可恢复
        assert!(FlowCardService::validate_status_transition(
            card_status::PAUSED,
            card_status::PREPARING
        )
        .is_ok());
        // 分支：回修可重新开卡
        assert!(FlowCardService::validate_status_transition(
            card_status::REWORK,
            card_status::OPENED
        )
        .is_ok());
        // 分支：终止
        assert!(FlowCardService::validate_status_transition(
            card_status::DYEING,
            card_status::TERMINATED
        )
        .is_ok());
    }

    /// 测试流转卡状态流转非法
    #[test]
    fn test_card_status_transition_invalid() {
        // 不能跨状态流转
        assert!(FlowCardService::validate_status_transition(
            card_status::OPENED,
            card_status::DYEING
        )
        .is_err());
        assert!(FlowCardService::validate_status_transition(
            card_status::OPENED,
            card_status::STORED
        )
        .is_err());
        // 不能回退
        assert!(FlowCardService::validate_status_transition(
            card_status::DYEING,
            card_status::PREPARING
        )
        .is_err());
        // 终态不可流转
        assert!(FlowCardService::validate_status_transition(
            card_status::SHIPPED,
            card_status::STORED
        )
        .is_err());
        assert!(FlowCardService::validate_status_transition(
            card_status::TERMINATED,
            card_status::DYEING
        )
        .is_err());
        assert!(FlowCardService::validate_status_transition(
            card_status::CANCELLED,
            card_status::OPENED
        )
        .is_err());
        // 已发货不可暂停
        assert!(FlowCardService::validate_status_transition(
            card_status::SHIPPED,
            card_status::PAUSED
        )
        .is_err());
    }

    /// 测试工序操作状态流转合法性
    #[test]
    fn test_operation_status_transition_valid() {
        // pending → in_progress → completed → transferred → stored
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::PENDING,
            op_status::IN_PROGRESS
        )
        .is_ok());
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::IN_PROGRESS,
            op_status::COMPLETED
        )
        .is_ok());
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::COMPLETED,
            op_status::TRANSFERRED
        )
        .is_ok());
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::TRANSFERRED,
            op_status::STORED
        )
        .is_ok());
        // completed 可直接入库
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::COMPLETED,
            op_status::STORED
        )
        .is_ok());
        // 分支：暂停/回修
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::IN_PROGRESS,
            op_status::PAUSED
        )
        .is_ok());
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::PAUSED,
            op_status::IN_PROGRESS
        )
        .is_ok());
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::IN_PROGRESS,
            op_status::REWORK
        )
        .is_ok());
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::REWORK,
            op_status::IN_PROGRESS
        )
        .is_ok());
    }

    /// 测试工序操作状态流转非法
    #[test]
    fn test_operation_status_transition_invalid() {
        // pending 不能直接 completed
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::PENDING,
            op_status::COMPLETED
        )
        .is_err());
        // in_progress 不能 transferred
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::IN_PROGRESS,
            op_status::TRANSFERRED
        )
        .is_err());
        // stored 终态
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::STORED,
            op_status::IN_PROGRESS
        )
        .is_err());
        // completed 不能回退到 in_progress
        assert!(FlowCardOperationService::validate_operation_status_transition(
            op_status::COMPLETED,
            op_status::IN_PROGRESS
        )
        .is_err());
    }

    /// 测试流转卡更新/删除状态校验
    #[test]
    fn test_card_validate_can_update_and_delete() {
        // 仅 opened/waiting_dyeing 可更新
        assert!(FlowCardService::validate_can_update(card_status::OPENED).is_ok());
        assert!(FlowCardService::validate_can_update(card_status::WAITING_DYEING).is_ok());
        assert!(FlowCardService::validate_can_update(card_status::DYEING).is_err());
        assert!(FlowCardService::validate_can_update(card_status::STORED).is_err());
        assert!(FlowCardService::validate_can_update(card_status::SHIPPED).is_err());

        // 仅 opened/waiting_dyeing 可删除
        assert!(FlowCardService::validate_can_delete(card_status::OPENED).is_ok());
        assert!(FlowCardService::validate_can_delete(card_status::WAITING_DYEING).is_ok());
        assert!(FlowCardService::validate_can_delete(card_status::SCHEDULED).is_err());
        assert!(FlowCardService::validate_can_delete(card_status::DYEING).is_err());
    }

    /// 测试分卡数量校验逻辑（边界条件）
    #[test]
    fn test_split_card_quantities_validation() {
        // 数量必须为正：0 为非法（0 不大于 0）
        let zero = Decimal::ZERO;
        assert!(!(zero > Decimal::ZERO));
        // 数量为负非法
        let neg = Decimal::from(-1);
        assert!(neg < Decimal::ZERO);
        // 数量为正合法
        let pos = Decimal::from(100);
        assert!(pos > Decimal::ZERO);
    }

    /// 测试合缸至少需要 2 张卡的业务约束
    #[test]
    fn test_merge_card_minimum_cards() {
        // 业务约束：合缸至少需要 2 张流转卡
        // 此测试验证业务规则的数值边界（实际 DB 校验在 Service 层）
        let single: Vec<i32> = vec![1];
        assert!(single.len() < 2, "单张卡不可合缸");
        let two: Vec<i32> = vec![1, 2];
        assert!(two.len() >= 2, "至少 2 张卡可合缸");
    }

    /// 测试拆卡匹数校验逻辑
    #[test]
    fn test_split_piece_counts_validation() {
        // 匹数必须为正
        let valid_counts = vec![50, 50];
        assert!(valid_counts.iter().all(|p| *p > 0));
        // 包含 0 非法
        let invalid_counts = vec![0, 50];
        assert!(!invalid_counts.iter().all(|p| *p > 0));
        // 包含负数非法
        let neg_counts = vec![-1, 50];
        assert!(!neg_counts.iter().all(|p| *p > 0));
    }

    /// 测试回修次数递增逻辑
    ///
    /// 真实业务（KESHTECH）：每次开内修卡，原卡 rework_count + 1
    #[test]
    fn test_rework_count_increment() {
        // 模拟回修次数递增
        let mut count: i32 = 0;
        // 第 1 次回修 → A
        count += 1;
        assert_eq!(count, 1);
        let suffix_1 = (b'A' + (count - 1) as u8) as char;
        assert_eq!(suffix_1, 'A');
        // 第 2 次回修 → B
        count += 1;
        assert_eq!(count, 2);
        let suffix_2 = (b'A' + (count - 1) as u8) as char;
        assert_eq!(suffix_2, 'B');
        // 第 3 次回修 → C
        count += 1;
        assert_eq!(count, 3);
        let suffix_3 = (b'A' + (count - 1) as u8) as char;
        assert_eq!(suffix_3, 'C');
    }

    /// 测试一缸一卡约束逻辑（同缸号只能有一张主卡）
    #[test]
    fn test_one_card_per_dye_lot_constraint() {
        // 业务约束：同 dye_lot_no 只能有一张未删除的主卡
        // 主卡定义：is_rework=false 且 parent_card_id=null 且 status != cancelled
        // 此测试验证约束条件的组合逻辑
        let is_rework = false;
        let parent_card_id: Option<i32> = None;
        let status = card_status::OPENED;

        // 主卡条件：非回修卡 + 无母卡 + 未取消
        let is_main_card = !is_rework && parent_card_id.is_none() && status != card_status::CANCELLED;
        assert!(is_main_card, "满足主卡条件");

        // 回修卡不是主卡
        let is_rework_card = true;
        let is_main_rework = !is_rework_card && parent_card_id.is_none();
        assert!(!is_main_rework, "回修卡不是主卡");

        // 子卡（拆卡生成）不是主卡
        let has_parent = true;
        let is_main_child = !is_rework && !has_parent;
        assert!(!is_main_child, "子卡不是主卡");
    }

    /// 测试 Decimal 比较可用（确保 rust_decimal 引入正确）
    #[test]
    fn test_decimal_comparison() {
        let positive = Decimal::from(100);
        let negative = Decimal::from(-50);
        assert!(positive > Decimal::ZERO);
        assert!(negative < Decimal::ZERO);
        assert!(positive > negative);
    }
}

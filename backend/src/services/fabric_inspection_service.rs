//! 验布打卷 Service
//!
//! v14 批次 426：验布打卷流程贯通
//! 依据：面料行业真实业务调研文档 §12.4 验布打卷与成品入库
//! 真实业务流程：
//!   验布机对接码表/电子称 → 疵点采集 → 生成验布报告
//!   → 卷唛标签打印 → PDA 扫描卷唛条码 → 自动入库
//!
//! 核心能力：
//! - 验布记录 CRUD + 状态机流转（pending→inspecting→graded→rolled→closed）
//! - 疵点明细 CRUD + 四分制/十分制扣分自动计算
//! - 等级判定（首级 first / 次级 second）+ 联动 A/B/C 分级
//! - 打卷入库（生成匹号 + 创建 inventory_piece + 汇总打卷数据）

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::batch_dye_lot::{self, Entity as DyeLotEntity};
use crate::models::fabric_defect_record::{self, ActiveModel as DefectActiveModel, Entity as DefectEntity, Model as DefectModel};
use crate::models::fabric_inspection_record::{self, ActiveModel as InspectionActiveModel, Entity as InspectionEntity, Model as InspectionModel};
use crate::models::inventory_piece::{self, ActiveModel as PieceActiveModel};
use crate::models::status::fabric_grade;
use crate::models::status::fabric_inspection as inspection_status;
use crate::models::status::fabric_scoring;
use crate::models::status::inventory_piece as piece_status;
use crate::utils::error::AppError;

// ============================================================================
// 评分计算纯函数（四分制 / 十分制）
// ============================================================================

/// 四分制扣分计算
///
/// 业务规则（AATCC/ASTM D5430）：
/// - 疵点长度 ≤3寸 → 1 分
/// - 3寸 < 疵点长度 ≤6寸 → 2 分
/// - 6寸 < 疵点长度 ≤9寸 → 3 分
/// - 疵点长度 >9寸 → 4 分
/// - 破洞/连续性疵点 → 4 分（不论长度）
pub fn calculate_four_point_points(
    length_inches: Decimal,
    is_hole: bool,
    is_continuous: bool,
) -> i32 {
    if is_hole || is_continuous {
        return 4;
    }
    if length_inches <= Decimal::new(3, 0) {
        1
    } else if length_inches <= Decimal::new(6, 0) {
        2
    } else if length_inches <= Decimal::new(9, 0) {
        3
    } else {
        4
    }
}

/// 十分制扣分计算
///
/// 业务规则（梭织布专用）：
/// - 破洞 → 10 分（不论长度）
/// - 经向(warp)：1寸下=1, 1-5寸=3, 5-10寸=5, 10-36寸=10
/// - 纬向(weft)：1寸下=1, 1-5寸=3, 5寸-半门幅=5, 半门幅以上=10
pub fn calculate_ten_point_points(
    length_inches: Decimal,
    direction: &str,
    is_hole: bool,
    is_half_width: bool,
) -> i32 {
    if is_hole {
        return 10;
    }
    if length_inches < Decimal::new(1, 0) {
        return 1;
    }
    if length_inches <= Decimal::new(5, 0) {
        return 3;
    }
    match direction {
        "warp" => {
            // 经向
            if length_inches <= Decimal::new(10, 0) {
                5
            } else {
                10
            }
        }
        "weft" => {
            // 纬向：半门幅以上=10，以下=5
            if is_half_width {
                10
            } else {
                5
            }
        }
        _ => {
            // other 方向按经向规则处理
            if length_inches <= Decimal::new(10, 0) {
                5
            } else {
                10
            }
        }
    }
}

/// 计算每百平方码分数（四分制等级判定依据）
///
/// 公式：每百平方码分数 = (总扣分 × 36 × 100) / (受检码数 × 幅宽英寸)
pub fn calculate_points_per_100_sq_yards(
    total_points: i32,
    inspected_yards: Decimal,
    fabric_width_inches: Decimal,
) -> Result<Decimal, AppError> {
    if inspected_yards <= Decimal::ZERO {
        return Err(AppError::business("受检码数必须 > 0"));
    }
    if fabric_width_inches <= Decimal::ZERO {
        return Err(AppError::business("幅宽必须 > 0"));
    }
    let numerator = Decimal::from(total_points) * Decimal::new(3600, 0); // 36 * 100
    let denominator = inspected_yards * fabric_width_inches;
    Ok(numerator / denominator)
}

/// 四分制等级判定
///
/// 业务规则：每百平方码分数 ≤40 = 首级(first)，>40 = 次级(second)
pub fn determine_grade_by_four_point(points_per_100_sq_yards: Decimal) -> String {
    if points_per_100_sq_yards <= Decimal::new(40, 0) {
        fabric_grade::FIRST.to_string()
    } else {
        fabric_grade::SECOND.to_string()
    }
}

/// 十分制等级判定
///
/// 业务规则：总扣分 < 总码数 = 首级(first)，≥ 总码数 = 次级(second)
pub fn determine_grade_by_ten_point(total_points: i32, inspected_yards: Decimal) -> String {
    let yards_int = inspected_yards.to_i32().unwrap_or(0);
    if total_points < yards_int {
        fabric_grade::FIRST.to_string()
    } else {
        fabric_grade::SECOND.to_string()
    }
}

// ============================================================================
// 验布记录 Service
// ============================================================================

/// 创建验布记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateInspectionRequest {
    pub flow_card_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub inspection_date: chrono::NaiveDate,
    pub inspector_id: Option<i32>,
    pub inspector_name: Option<String>,
    pub machine_no: Option<String>,
    pub scoring_system: Option<String>,
    pub fabric_width_inches: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新验布记录请求（仅 pending 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInspectionRequest {
    pub inspector_id: Option<i32>,
    pub inspector_name: Option<String>,
    pub machine_no: Option<String>,
    pub scoring_system: Option<String>,
    pub fabric_width_inches: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 评级请求（inspecting → graded）
#[derive(Debug, Clone, Deserialize)]
pub struct GradeInspectionRequest {
    pub inspected_yards: Decimal,
    pub qualification_rate: Option<Decimal>,
}

/// 打卷入库请求（graded → rolled）
#[derive(Debug, Clone, Deserialize)]
pub struct RollFabricRequest {
    pub warehouse_id: i32,
    pub roll_length: Decimal,
    pub roll_weight: Option<Decimal>,
    pub roll_width: Option<Decimal>,
    pub roll_gram_weight: Option<Decimal>,
}

/// 验布记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct InspectionQuery {
    pub inspection_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub flow_card_id: Option<i32>,
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 验布记录 Service
pub struct FabricInspectionService {
    db: Arc<DatabaseConnection>,
}

impl FabricInspectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成验布单号：FIR-YYYYMMDDHHMMSS-NNN
    fn generate_inspection_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("FIR-{}-{:03}", timestamp, random)
    }

    /// 创建验布记录
    pub async fn create(&self, req: CreateInspectionRequest) -> Result<InspectionModel, AppError> {
        // 业务校验：评分制式合法
        let scoring_system = req.scoring_system.unwrap_or_else(|| fabric_scoring::FOUR_POINT.to_string());
        if scoring_system != fabric_scoring::FOUR_POINT && scoring_system != fabric_scoring::TEN_POINT {
            return Err(AppError::business(format!(
                "评分制式必须是 {} 或 {}",
                fabric_scoring::FOUR_POINT, fabric_scoring::TEN_POINT
            )));
        }

        // 业务校验：幅宽必须为正
        if let Some(w) = req.fabric_width_inches {
            if w <= Decimal::ZERO {
                return Err(AppError::business("幅宽必须 > 0"));
            }
        }

        let inspection_no = Self::generate_inspection_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = InspectionActiveModel {
            id: Default::default(),
            inspection_no: Set(inspection_no),
            flow_card_id: Set(req.flow_card_id),
            dye_lot_no: Set(req.dye_lot_no),
            product_id: Set(req.product_id),
            product_name: Set(req.product_name),
            color_no: Set(req.color_no),
            inspection_date: Set(req.inspection_date),
            inspector_id: Set(req.inspector_id),
            inspector_name: Set(req.inspector_name),
            machine_no: Set(req.machine_no),
            scoring_system: Set(scoring_system),
            inspected_yards: Set(Decimal::ZERO),
            fabric_width_inches: Set(req.fabric_width_inches),
            total_defect_points: Set(0),
            points_per_100_sq_yards: Set(None),
            grade: Set(None),
            qualification_rate: Set(None),
            abc_grade: Set(None),
            total_rolls: Set(0),
            total_roll_length: Set(Decimal::ZERO),
            total_roll_weight: Set(Decimal::ZERO),
            status: Set(inspection_status::PENDING.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("验布记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新验布记录（仅 pending 状态可更新）
    pub async fn update(&self, id: i32, req: UpdateInspectionRequest) -> Result<InspectionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != inspection_status::PENDING {
            return Err(AppError::business(format!(
                "仅待验布(pending)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: InspectionActiveModel = model.into();

        if let Some(v) = req.inspector_id {
            active.inspector_id = Set(Some(v));
        }
        if let Some(v) = req.inspector_name {
            active.inspector_name = Set(Some(v));
        }
        if let Some(v) = req.machine_no {
            active.machine_no = Set(Some(v));
        }
        if let Some(v) = req.scoring_system {
            if v != fabric_scoring::FOUR_POINT && v != fabric_scoring::TEN_POINT {
                return Err(AppError::business("评分制式不合法"));
            }
            active.scoring_system = Set(v);
        }
        if let Some(v) = req.fabric_width_inches {
            if v <= Decimal::ZERO {
                return Err(AppError::business("幅宽必须 > 0"));
            }
            active.fabric_width_inches = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除验布记录（仅 pending 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != inspection_status::PENDING {
            return Err(AppError::business(format!(
                "仅待验布(pending)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: InspectionActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<InspectionModel, AppError> {
        let model = InspectionEntity::find_by_id(id)
            .filter(fabric_inspection_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("验布记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按单号查询
    pub async fn get_by_no(&self, inspection_no: &str) -> Result<InspectionModel, AppError> {
        let model = InspectionEntity::find()
            .filter(fabric_inspection_record::Column::InspectionNo.eq(inspection_no))
            .filter(fabric_inspection_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("验布单号 {} 不存在", inspection_no)))?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(&self, query: InspectionQuery) -> Result<(Vec<InspectionModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = InspectionEntity::find()
            .filter(fabric_inspection_record::Column::IsDeleted.eq(false));

        if let Some(v) = query.inspection_no {
            q = q.filter(fabric_inspection_record::Column::InspectionNo.like(format!("%{}%", v)));
        }
        if let Some(v) = query.dye_lot_no {
            q = q.filter(fabric_inspection_record::Column::DyeLotNo.eq(v));
        }
        if let Some(v) = query.flow_card_id {
            q = q.filter(fabric_inspection_record::Column::FlowCardId.eq(v));
        }
        if let Some(v) = query.product_id {
            q = q.filter(fabric_inspection_record::Column::ProductId.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(fabric_inspection_record::Column::Status.eq(v));
        }

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(fabric_inspection_record::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 开始验布（pending → inspecting）
    pub async fn start_inspection(&self, id: i32) -> Result<InspectionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != inspection_status::PENDING {
            return Err(AppError::business(format!(
                "仅待验布(pending)状态可开始验布，当前状态: {}",
                model.status
            )));
        }
        let mut active: InspectionActiveModel = model.into();
        active.status = Set(inspection_status::INSPECTING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 评级（inspecting → graded）
    ///
    /// 计算总扣分/每百平方码分数/等级/A-B-C 分级
    pub async fn grade_inspection(
        &self,
        id: i32,
        req: GradeInspectionRequest,
    ) -> Result<InspectionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != inspection_status::INSPECTING {
            return Err(AppError::business(format!(
                "仅验布中(inspecting)状态可评级，当前状态: {}",
                model.status
            )));
        }

        // 业务校验：受检码数必须为正
        if req.inspected_yards <= Decimal::ZERO {
            return Err(AppError::business("受检码数必须 > 0"));
        }

        // 查询所有疵点明细，汇总总扣分
        let defects = DefectEntity::find()
            .filter(fabric_defect_record::Column::InspectionId.eq(id))
            .all(&*self.db)
            .await?;
        let total_points: i32 = defects.iter().map(|d| d.points).sum();

        // 根据评分制式判定等级
        let (grade, points_per_100_sq_yards) = if model.scoring_system == fabric_scoring::FOUR_POINT {
            let width = model.fabric_width_inches.ok_or_else(|| {
                AppError::business("四分制评级需要幅宽(fabric_width_inches)，请先设置幅宽")
            })?;
            let p100 = calculate_points_per_100_sq_yards(total_points, req.inspected_yards, width)?;
            let g = determine_grade_by_four_point(p100);
            (g, Some(p100))
        } else {
            // 十分制
            let g = determine_grade_by_ten_point(total_points, req.inspected_yards);
            (g, None)
        };

        // 联动 A/B/C 分级（基于合格率）
        let abc_grade = crate::services::quality_inspection_service::determine_quality_grade(
            req.qualification_rate,
        );

        let mut active: InspectionActiveModel = model.into();
        active.inspected_yards = Set(req.inspected_yards);
        active.total_defect_points = Set(total_points);
        active.points_per_100_sq_yards = Set(points_per_100_sq_yards);
        active.grade = Set(Some(grade));
        active.qualification_rate = Set(req.qualification_rate);
        active.abc_grade = Set(Some(abc_grade));
        active.status = Set(inspection_status::GRADED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 打卷入库（graded → rolled）
    ///
    /// 生成匹号 + 创建 inventory_piece + 汇总打卷数据
    pub async fn roll_fabric(
        &self,
        id: i32,
        req: RollFabricRequest,
    ) -> Result<InspectionModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_roll_preconditions(&model, &req)?;
        let dye_lot_no = model.dye_lot_no.clone().ok_or_else(|| {
            AppError::business("打卷入库需要缸号(dye_lot_no)，请先在验布记录中设置缸号")
        })?;
        let dye_lot = self.fetch_dye_lot(&dye_lot_no).await?;
        let dye_lot_id = dye_lot.id;
        let (piece_no, next_seq) = self
            .generate_next_piece_no(dye_lot_id, &dye_lot_no)
            .await?;
        let new_piece = Self::build_piece_active_model(
            &req, dye_lot_id, &dye_lot_no, &model, &piece_no, next_seq, id,
        );
        new_piece.insert(&*self.db).await?;
        self.apply_roll_summary(model, &req).await
    }

    /// 校验打卷前置条件（状态 + 长度）
    fn validate_roll_preconditions(
        model: &InspectionModel,
        req: &RollFabricRequest,
    ) -> Result<(), AppError> {
        if model.status != inspection_status::GRADED {
            return Err(AppError::business(format!(
                "仅已评级(graded)状态可打卷入库，当前状态: {}",
                model.status
            )));
        }
        if req.roll_length <= Decimal::ZERO {
            return Err(AppError::business("打卷长度必须 > 0"));
        }
        Ok(())
    }

    /// 按缸号查询 dye_lot 记录
    async fn fetch_dye_lot(&self, dye_lot_no: &str) -> Result<batch_dye_lot::Model, AppError> {
        DyeLotEntity::find()
            .filter(batch_dye_lot::Column::DyeLotNo.eq(dye_lot_no))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::business(format!("缸号 {} 在 batch_dye_lot 表中不存在", dye_lot_no))
            })
    }

    /// 生成下一个匹号（含唯一性校验）
    async fn generate_next_piece_no(
        &self,
        dye_lot_id: i32,
        dye_lot_no: &str,
    ) -> Result<(String, i32), AppError> {
        let max_seq_piece = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::DyeLotId.eq(dye_lot_id))
            .filter(inventory_piece::Column::PieceSeq.is_not_null())
            .order_by_desc(inventory_piece::Column::PieceSeq)
            .one(&*self.db)
            .await?;
        let next_seq = max_seq_piece
            .as_ref()
            .and_then(|p| p.piece_seq)
            .map(|s| s + 1)
            .unwrap_or(1);
        let piece_no = format!("{}-{:03}", dye_lot_no, next_seq);
        let existing = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::DyeLotId.eq(dye_lot_id))
            .filter(inventory_piece::Column::PieceNo.eq(piece_no.clone()))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::business(format!(
                "匹号 {} 已存在，同一缸号下匹号不能重复",
                piece_no
            )));
        }
        Ok((piece_no, next_seq))
    }

    /// 构建 inventory_piece ActiveModel
    fn build_piece_active_model(
        req: &RollFabricRequest,
        dye_lot_id: i32,
        dye_lot_no: &str,
        model: &InspectionModel,
        piece_no: &str,
        next_seq: i32,
        inspection_id: i32,
    ) -> PieceActiveModel {
        let now_piece = chrono::Utc::now();
        PieceActiveModel {
            id: Default::default(),
            dye_lot_id: Set(dye_lot_id),
            batch_no: Set(dye_lot_no.to_string()),
            product_id: Set(model.product_id.unwrap_or(0)),
            warehouse_id: Set(req.warehouse_id),
            location_id: Set(None),
            piece_no: Set(piece_no.to_string()),
            barcode: Set(Some(piece_no.to_string())),
            parent_piece_id: Set(None),
            length: Set(req.roll_length),
            weight: Set(req.roll_weight),
            width: Set(req.roll_width),
            gram_weight: Set(req.roll_gram_weight),
            status: Set(piece_status::AVAILABLE.to_string()),
            remarks: Set(Some(format!("验布打卷生成，验布单号 {}", model.inspection_no))),
            scan_type: Set(None),
            created_at: Set(now_piece),
            updated_at: Set(now_piece),
            supplier_piece_no: Default::default(),
            position_no: Default::default(),
            package_no: Default::default(),
            production_date: Default::default(),
            shelf_life: Default::default(),
            quality_status: Set(Some("passed".to_string())),
            inventory_status: Set(Some("available".to_string())),
            created_by: Default::default(),
            updated_by: Default::default(),
            color_no: Set(model.color_no.clone()),
            dye_lot_no: Set(model.dye_lot_no.clone()),
            inspection_id: Set(Some(inspection_id)),
            piece_seq: Set(Some(next_seq)),
        }
    }

    /// 更新验布记录的打卷汇总并流转状态
    async fn apply_roll_summary(
        &self,
        model: InspectionModel,
        req: &RollFabricRequest,
    ) -> Result<InspectionModel, AppError> {
        let new_total_rolls = model.total_rolls + 1;
        let new_total_length = model.total_roll_length + req.roll_length;
        let new_total_weight = model.total_roll_weight + req.roll_weight.unwrap_or(Decimal::ZERO);
        let mut active: InspectionActiveModel = model.into();
        active.total_rolls = Set(new_total_rolls);
        active.total_roll_length = Set(new_total_length);
        active.total_roll_weight = Set(new_total_weight);
        active.status = Set(inspection_status::ROLLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 关闭归档（rolled → closed）
    pub async fn close_inspection(&self, id: i32) -> Result<InspectionModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != inspection_status::ROLLED {
            return Err(AppError::business(format!(
                "仅已打卷(rolled)状态可关闭归档，当前状态: {}",
                model.status
            )));
        }
        let mut active: InspectionActiveModel = model.into();
        active.status = Set(inspection_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

// ============================================================================
// 疵点明细 Service
// ============================================================================

/// 创建疵点明细请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDefectRequest {
    pub inspection_id: i32,
    pub defect_type: String,
    pub position_yards: Decimal,
    pub defect_length_inches: Decimal,
    pub direction: Option<String>,
    pub is_hole: Option<bool>,
    pub is_continuous: Option<bool>,
    pub is_half_width: Option<bool>,
    pub description: Option<String>,
}

/// 疵点明细 Service
pub struct FabricDefectService {
    db: Arc<DatabaseConnection>,
}

impl FabricDefectService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 合法疵点类型校验
    fn validate_defect_type(defect_type: &str) -> Result<(), AppError> {
        let valid_types = [
            "broken_end", "oil_stain", "color_spot", "hole", "skew_lane",
            "streak", "color_diff", "narrow_width", "crease", "uneven_dye",
            "lint", "other",
        ];
        if !valid_types.contains(&defect_type) {
            return Err(AppError::business(format!(
                "疵点类型必须是 {:?} 之一",
                valid_types
            )));
        }
        Ok(())
    }

    /// 添加疵点明细（自动计算扣分）
    ///
    /// 根据验布记录的评分制式（four_point/ten_point）自动计算扣分
    pub async fn create(&self, req: CreateDefectRequest) -> Result<DefectModel, AppError> {
        // 业务校验：疵点类型合法
        Self::validate_defect_type(&req.defect_type)?;

        // 业务校验：验布记录存在且处于验布中(inspecting)状态
        let inspection = InspectionEntity::find_by_id(req.inspection_id)
            .filter(fabric_inspection_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("验布记录 {} 不存在", req.inspection_id)))?;

        if inspection.status != inspection_status::INSPECTING {
            return Err(AppError::business(format!(
                "仅验布中(inspecting)状态可添加疵点，当前状态: {}",
                inspection.status
            )));
        }

        // 业务校验：疵点长度不能为负
        if req.defect_length_inches < Decimal::ZERO {
            return Err(AppError::business("疵点长度不能为负"));
        }

        // 业务校验：疵点位置不能为负
        if req.position_yards < Decimal::ZERO {
            return Err(AppError::business("疵点位置不能为负"));
        }

        let direction = req.direction.unwrap_or_else(|| "other".to_string());
        let is_hole = req.is_hole.unwrap_or(false);
        let is_continuous = req.is_continuous.unwrap_or(false);
        let is_half_width = req.is_half_width.unwrap_or(false);

        // 根据评分制式自动计算扣分
        let points = if inspection.scoring_system == fabric_scoring::FOUR_POINT {
            calculate_four_point_points(req.defect_length_inches, is_hole, is_continuous)
        } else {
            calculate_ten_point_points(req.defect_length_inches, &direction, is_hole, is_half_width)
        };

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = DefectActiveModel {
            id: Default::default(),
            inspection_id: Set(req.inspection_id),
            defect_type: Set(req.defect_type),
            position_yards: Set(req.position_yards),
            defect_length_inches: Set(req.defect_length_inches),
            direction: Set(direction),
            is_hole: Set(is_hole),
            is_continuous: Set(is_continuous),
            is_half_width: Set(is_half_width),
            points: Set(points),
            description: Set(req.description),
            created_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("疵点明细创建失败: {}", e)))?;
        Ok(result)
    }

    /// 按验布记录查询疵点明细
    pub async fn list_by_inspection(&self, inspection_id: i32) -> Result<Vec<DefectModel>, AppError> {
        let list = DefectEntity::find()
            .filter(fabric_defect_record::Column::InspectionId.eq(inspection_id))
            .order_by_asc(fabric_defect_record::Column::PositionYards)
            .all(&*self.db)
            .await?;
        Ok(list)
    }

    /// 按 ID 查询疵点明细
    pub async fn get_by_id(&self, id: i32) -> Result<DefectModel, AppError> {
        let model = DefectEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("疵点明细 {} 不存在", id)))?;
        Ok(model)
    }

    /// 删除疵点明细（仅验布中状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let defect = self.get_by_id(id).await?;
        let inspection = InspectionEntity::find_by_id(defect.inspection_id)
            .filter(fabric_inspection_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("关联的验布记录不存在"))?;

        if inspection.status != inspection_status::INSPECTING {
            return Err(AppError::business(format!(
                "仅验布中(inspecting)状态可删除疵点，当前状态: {}",
                inspection.status
            )));
        }

        DefectEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("疵点明细删除失败: {}", e)))?;
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

    // ===== 四分制扣分计算测试 =====

    #[test]
    fn test_calculate_four_point_points_normal() {
        // ≤3寸 = 1分
        assert_eq!(calculate_four_point_points(Decimal::new(3, 0), false, false), 1);
        assert_eq!(calculate_four_point_points(Decimal::new(0, 0), false, false), 1);
        assert_eq!(calculate_four_point_points(Decimal::new(2, 0), false, false), 1);

        // 3-6寸 = 2分
        assert_eq!(calculate_four_point_points(Decimal::new(4, 0), false, false), 2);
        assert_eq!(calculate_four_point_points(Decimal::new(6, 0), false, false), 2);

        // 6-9寸 = 3分
        assert_eq!(calculate_four_point_points(Decimal::new(7, 0), false, false), 3);
        assert_eq!(calculate_four_point_points(Decimal::new(9, 0), false, false), 3);

        // >9寸 = 4分
        assert_eq!(calculate_four_point_points(Decimal::new(10, 0), false, false), 4);
        assert_eq!(calculate_four_point_points(Decimal::new(36, 0), false, false), 4);
    }

    #[test]
    fn test_calculate_four_point_points_hole_and_continuous() {
        // 破洞不论大小一律4分
        assert_eq!(calculate_four_point_points(Decimal::new(0, 0), true, false), 4);
        assert_eq!(calculate_four_point_points(Decimal::new(1, 0), true, false), 4);

        // 连续性疵点不论大小一律4分
        assert_eq!(calculate_four_point_points(Decimal::new(0, 0), false, true), 4);
        assert_eq!(calculate_four_point_points(Decimal::new(1, 0), false, true), 4);
    }

    // ===== 十分制扣分计算测试 =====

    #[test]
    fn test_calculate_ten_point_points_warp() {
        // 破洞 = 10分
        assert_eq!(calculate_ten_point_points(Decimal::new(1, 0), "warp", true, false), 10);

        // 经向：1寸下=1
        assert_eq!(calculate_ten_point_points(Decimal::new(0, 0), "warp", false, false), 1);

        // 经向：1-5寸=3
        assert_eq!(calculate_ten_point_points(Decimal::new(1, 0), "warp", false, false), 3);
        assert_eq!(calculate_ten_point_points(Decimal::new(5, 0), "warp", false, false), 3);

        // 经向：5-10寸=5
        assert_eq!(calculate_ten_point_points(Decimal::new(6, 0), "warp", false, false), 5);
        assert_eq!(calculate_ten_point_points(Decimal::new(10, 0), "warp", false, false), 5);

        // 经向：10-36寸=10
        assert_eq!(calculate_ten_point_points(Decimal::new(11, 0), "warp", false, false), 10);
        assert_eq!(calculate_ten_point_points(Decimal::new(36, 0), "warp", false, false), 10);
    }

    #[test]
    fn test_calculate_ten_point_points_weft() {
        // 纬向：1寸下=1
        assert_eq!(calculate_ten_point_points(Decimal::new(0, 0), "weft", false, false), 1);

        // 纬向：1-5寸=3
        assert_eq!(calculate_ten_point_points(Decimal::new(3, 0), "weft", false, false), 3);

        // 纬向：5寸-半门幅=5
        assert_eq!(calculate_ten_point_points(Decimal::new(6, 0), "weft", false, false), 5);

        // 纬向：半门幅以上=10
        assert_eq!(calculate_ten_point_points(Decimal::new(6, 0), "weft", false, true), 10);
    }

    // ===== 每百平方码分数计算测试 =====

    #[test]
    fn test_calculate_points_per_100_sq_yards() {
        // (655 × 36 × 100) / (2500 × 55) = 17.1
        let result = calculate_points_per_100_sq_yards(
            655,
            Decimal::new(2500, 0),
            Decimal::new(55, 0),
        ).unwrap();
        let expected = Decimal::new(171, 1); // 17.1
        assert_eq!(result.round_dp(1), expected);
    }

    #[test]
    fn test_calculate_points_per_100_sq_yards_invalid_input() {
        // 受检码数为0
        let result = calculate_points_per_100_sq_yards(10, Decimal::ZERO, Decimal::new(55, 0));
        assert!(result.is_err());

        // 幅宽为0
        let result = calculate_points_per_100_sq_yards(10, Decimal::new(100, 0), Decimal::ZERO);
        assert!(result.is_err());
    }

    // ===== 等级判定测试 =====

    #[test]
    fn test_determine_grade_by_four_point() {
        // ≤40 = 首级
        assert_eq!(determine_grade_by_four_point(Decimal::new(40, 0)), fabric_grade::FIRST);
        assert_eq!(determine_grade_by_four_point(Decimal::new(0, 0)), fabric_grade::FIRST);
        assert_eq!(determine_grade_by_four_point(Decimal::new(16, 0)), fabric_grade::FIRST);

        // >40 = 次级
        assert_eq!(determine_grade_by_four_point(Decimal::new(41, 0)), fabric_grade::SECOND);
        assert_eq!(determine_grade_by_four_point(Decimal::new(100, 0)), fabric_grade::SECOND);
    }

    #[test]
    fn test_determine_grade_by_ten_point() {
        // 总扣分 < 总码数 = 首级
        assert_eq!(determine_grade_by_ten_point(50, Decimal::new(100, 0)), fabric_grade::FIRST);
        assert_eq!(determine_grade_by_ten_point(0, Decimal::new(100, 0)), fabric_grade::FIRST);

        // 总扣分 ≥ 总码数 = 次级
        assert_eq!(determine_grade_by_ten_point(100, Decimal::new(100, 0)), fabric_grade::SECOND);
        assert_eq!(determine_grade_by_ten_point(150, Decimal::new(100, 0)), fabric_grade::SECOND);
    }

    // ===== 疵点类型校验测试 =====

    #[test]
    fn test_validate_defect_type_valid() {
        assert!(FabricDefectService::validate_defect_type("broken_end").is_ok());
        assert!(FabricDefectService::validate_defect_type("hole").is_ok());
        assert!(FabricDefectService::validate_defect_type("other").is_ok());
    }

    #[test]
    fn test_validate_defect_type_invalid() {
        assert!(FabricDefectService::validate_defect_type("invalid_type").is_err());
        assert!(FabricDefectService::validate_defect_type("").is_err());
    }
}

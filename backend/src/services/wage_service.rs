//! 产量工资 Service
//!
//! v14 批次 427：产量工资核算贯通
//! 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
//! 真实业务流程：
//!   工序流转扫码 → process_step_record 自动记录工人 IDs + 实际产量 + 合格产量（批次 425 已建）
//!   工价方案定义 → 每道工序的计件/计时单价 + A/B/C 等级系数
//!   工资计算 → 按工序记录 + 工价方案 + 等级系数自动计算每个工人的应得工资
//!   班组汇总 → 按车间/周期汇总工资，自动进入财务工资核算模块
//!
//! 核心能力：
//! - 工序工价 CRUD + 状态机流转（draft→active→disabled）
//! - 工资记录 CRUD + 状态机流转（draft→confirmed→paid/cancelled）
//! - 工资计算（按工价+工序记录+等级系数自动计算每个工人的应得工资）
//! - 三维度产量统计（工序产量 + 设备产量 + 工人产量工资）
//!
//! 复用现有功能（§10.0.1）：
//! - process_step_record 表：作为产量数据源（批次 425 已建）
//! - process_route 表：作为工序定义（批次 425 已建）
//! - determine_quality_grade 函数：A/B/C 等级判定（批次 421 已建）

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

use crate::models::process_route::Entity as RouteEntity;
use crate::models::process_step_record::{self, Entity as StepEntity, Model as StepModel};
use crate::models::process_wage_rate::{self, ActiveModel as RateActiveModel, Entity as RateEntity, Model as RateModel};
use crate::models::status::wage_rate_status;
use crate::models::status::wage_record_status;
use crate::models::status::wage_type;
use crate::models::wage_record::{self, ActiveModel as RecordActiveModel, Entity as RecordEntity, Model as RecordModel};
use crate::models::wage_record_detail::{self, ActiveModel as DetailActiveModel};
use crate::utils::error::AppError;

// 复用批次 421 的质量分级函数和常量
use crate::services::quality_inspection_service::{
    determine_quality_grade, QUALITY_GRADE_A, QUALITY_GRADE_B, QUALITY_GRADE_C,
};

/// 将 NaiveDate 转换为带时区的 DateTime（当天 00:00:00 UTC）
///
/// 用于工序记录的 start_at 字段比较
fn naive_date_to_date_time_tz(date: chrono::NaiveDate) -> chrono::DateTime<chrono::FixedOffset> {
    let naive_time = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let naive_date_time = chrono::NaiveDateTime::new(date, naive_time);
    chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(
        naive_date_time,
        chrono::FixedOffset::east_opt(0).unwrap(),
    )
}

/// 将 NaiveDate 转换为带时区的当天 23:59:59（用于区间右边界）
fn naive_date_to_end_of_day_tz(date: chrono::NaiveDate) -> chrono::DateTime<chrono::FixedOffset> {
    let naive_time = chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap();
    let naive_date_time = chrono::NaiveDateTime::new(date, naive_time);
    chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(
        naive_date_time,
        chrono::FixedOffset::east_opt(0).unwrap(),
    )
}

// ============================================================================
// 评分计算纯函数
// ============================================================================

/// 计算合格率（百分比，0-100）
///
/// 业务规则：
/// - 若实际产量为 0 或 None，合格率为 0
/// - 若合格产量为 None，按 0 处理
/// - 公式：qualified_quantity / actual_quantity × 100
pub fn compute_qualification_rate(
    actual_quantity: Option<Decimal>,
    qualified_quantity: Option<Decimal>,
) -> Decimal {
    let actual = actual_quantity.unwrap_or(Decimal::ZERO);
    if actual <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let qualified = qualified_quantity.unwrap_or(Decimal::ZERO);
    // qualified / actual × 100
    qualified * Decimal::new(100, 0) / actual
}

/// 依据合格率判定质检等级（A/B/C）
///
/// 业务规则（复用批次 421 determine_quality_grade）：
/// - 合格率 ≥ 95% → A 级（合格）
/// - 80% ≤ 合格率 < 95% → B 级（让步接收）
/// - 合格率 < 80% → C 级（不合格）
pub fn determine_grade_by_qualification_rate(rate: Decimal) -> String {
    determine_quality_grade(Some(rate))
}

/// 依据质检等级返回工价等级系数
///
/// 业务规则：
/// - A 级：grade_a_ratio（默认全额 1.0）
/// - B 级：grade_b_ratio（默认 8 折 0.8）
/// - C 级：grade_c_ratio（默认不计 0.0）
pub fn determine_grade_ratio(grade: &str, rate_model: &RateModel) -> Decimal {
    match grade {
        QUALITY_GRADE_A => rate_model.grade_a_ratio,
        QUALITY_GRADE_B => rate_model.grade_b_ratio,
        QUALITY_GRADE_C => rate_model.grade_c_ratio,
        _ => Decimal::ZERO,
    }
}

/// 计算单条工序记录的工资明细
///
/// 业务规则：
/// - 计件工资 = 合格产量 × 计件单价 × 等级系数
/// - 计时工资 = 工时（分钟） × 计时单价 × 等级系数
/// - 应得工资 = 计件工资 + 计时工资（根据 wage_type 选择）
///
/// 参数：
/// - rate: 工价方案
/// - actual_quantity: 实际产量
/// - qualified_quantity: 合格产量
/// - duration_minutes: 工时（分钟）
///
/// 返回：(grade, grade_ratio, piece_wage, time_wage, wage_amount)
pub fn calculate_wage_for_step(
    rate: &RateModel,
    actual_quantity: Option<Decimal>,
    qualified_quantity: Option<Decimal>,
    duration_minutes: Option<i32>,
) -> (String, Decimal, Decimal, Decimal, Decimal) {
    // 1. 计算合格率
    let rate_value = compute_qualification_rate(actual_quantity, qualified_quantity);
    // 2. 判定等级
    let grade = determine_grade_by_qualification_rate(rate_value);
    // 3. 获取等级系数
    let grade_ratio = determine_grade_ratio(&grade, rate);
    // 4. 按工价类型计算工资
    let qualified = qualified_quantity.unwrap_or(Decimal::ZERO);
    let minutes = Decimal::from(duration_minutes.unwrap_or(0));

    let mut piece_wage = Decimal::ZERO;
    let mut time_wage = Decimal::ZERO;

    match rate.wage_type.as_str() {
        wage_type::PIECE => {
            // 计件：合格产量 × 计件单价 × 等级系数
            piece_wage = qualified * rate.piece_price * grade_ratio;
        }
        wage_type::TIME => {
            // 计时：工时 × 计时单价 × 等级系数
            time_wage = minutes * rate.time_price * grade_ratio;
        }
        wage_type::MIXED => {
            // 混合：计件 + 计时
            piece_wage = qualified * rate.piece_price * grade_ratio;
            time_wage = minutes * rate.time_price * grade_ratio;
        }
        _ => {
            // 未知类型按计件处理
            piece_wage = qualified * rate.piece_price * grade_ratio;
        }
    }

    let wage_amount = piece_wage + time_wage;
    (grade, grade_ratio, piece_wage, time_wage, wage_amount)
}

/// 解析工序记录的工人 IDs（逗号分隔字符串 → HashSet）
///
/// 真实业务：扫码登记工人时，可能多个工人共同完成一道工序
/// 工资按人均分配（简化方案，实际业务可按工时比例分配）
pub fn parse_worker_ids(worker_ids_str: &Option<String>) -> Vec<i32> {
    let s = match worker_ids_str {
        Some(s) if !s.trim().is_empty() => s,
        _ => return Vec::new(),
    };
    s.split(',')
        .filter_map(|id_str| {
            let trimmed = id_str.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<i32>().ok()
            }
        })
        .collect()
}

/// 解析工人姓名（逗号分隔字符串 → Vec）
pub fn parse_worker_names(worker_names_str: &Option<String>) -> Vec<String> {
    let s = match worker_names_str {
        Some(s) if !s.trim().is_empty() => s,
        _ => return Vec::new(),
    };
    s.split(',').map(|n| n.trim().to_string()).collect()
}

/// 按人均分配工资（多人共同完成一道工序时）
///
/// 公式：单人工资 = 总工资 / 工人数量
pub fn split_wage_among_workers(wage: Decimal, worker_count: usize) -> Decimal {
    if worker_count == 0 {
        return Decimal::ZERO;
    }
    wage / Decimal::from(worker_count)
}

// ============================================================================
// 工序工价 Service
// ============================================================================

/// 创建工价请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWageRateRequest {
    pub process_route_id: i32,
    pub wage_type: Option<String>,
    pub piece_price: Option<Decimal>,
    pub time_price: Option<Decimal>,
    pub grade_a_ratio: Option<Decimal>,
    pub grade_b_ratio: Option<Decimal>,
    pub grade_c_ratio: Option<Decimal>,
    pub effective_date: chrono::NaiveDate,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub workshop: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工价请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWageRateRequest {
    pub wage_type: Option<String>,
    pub piece_price: Option<Decimal>,
    pub time_price: Option<Decimal>,
    pub grade_a_ratio: Option<Decimal>,
    pub grade_b_ratio: Option<Decimal>,
    pub grade_c_ratio: Option<Decimal>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub workshop: Option<String>,
    pub remarks: Option<String>,
}

/// 工价查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct WageRateQuery {
    pub route_code: Option<String>,
    pub process_route_id: Option<i32>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 工序工价 Service
pub struct WageRateService {
    db: Arc<DatabaseConnection>,
}

impl WageRateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成工价单号：PWR-YYYYMMDDHHMMSS-NNN
    fn generate_rate_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PWR-{}-{:03}", timestamp, random)
    }

    /// 创建工价
    pub async fn create(&self, req: CreateWageRateRequest) -> Result<RateModel, AppError> {
        // 业务校验：工序路线存在
        let route = RouteEntity::find_by_id(req.process_route_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::business(format!("工序路线 {} 不存在", req.process_route_id))
            })?;

        // 业务校验：工价类型合法
        let wage_type_value = req
            .wage_type
            .unwrap_or_else(|| wage_type::PIECE.to_string());
        if wage_type_value != wage_type::PIECE
            && wage_type_value != wage_type::TIME
            && wage_type_value != wage_type::MIXED
        {
            return Err(AppError::business(format!(
                "工价类型必须是 {} / {} / {}",
                wage_type::PIECE,
                wage_type::TIME,
                wage_type::MIXED
            )));
        }

        // 业务校验：单价非负
        let piece_price = req.piece_price.unwrap_or(Decimal::ZERO);
        let time_price = req.time_price.unwrap_or(Decimal::ZERO);
        if piece_price < Decimal::ZERO {
            return Err(AppError::business("计件单价不能为负"));
        }
        if time_price < Decimal::ZERO {
            return Err(AppError::business("计时单价不能为负"));
        }

        // 业务校验：计件类型必须有计件单价，计时类型必须有计时单价
        if wage_type_value == wage_type::PIECE && piece_price <= Decimal::ZERO {
            return Err(AppError::business("计件工价必须设置计件单价 > 0"));
        }
        if wage_type_value == wage_type::TIME && time_price <= Decimal::ZERO {
            return Err(AppError::business("计时工价必须设置计时单价 > 0"));
        }
        if wage_type_value == wage_type::MIXED
            && piece_price <= Decimal::ZERO
            && time_price <= Decimal::ZERO
        {
            return Err(AppError::business("混合工价必须设置计件或计时单价 > 0"));
        }

        // 业务校验：等级系数范围 [0, 1]
        let grade_a_ratio = req.grade_a_ratio.unwrap_or_else(|| Decimal::new(10, 1)); // 1.0
        let grade_b_ratio = req.grade_b_ratio.unwrap_or_else(|| Decimal::new(8, 1)); // 0.8
        let grade_c_ratio = req.grade_c_ratio.unwrap_or(Decimal::ZERO);
        for (name, value) in [
            ("A 级", grade_a_ratio),
            ("B 级", grade_b_ratio),
            ("C 级", grade_c_ratio),
        ] {
            if value < Decimal::ZERO || value > Decimal::new(10, 1) {
                return Err(AppError::business(format!(
                    "{} 等级系数必须在 [0, 1] 范围内",
                    name
                )));
            }
        }

        // 业务校验：失效日期必须晚于生效日期
        if let Some(expiry) = req.expiry_date {
            if expiry <= req.effective_date {
                return Err(AppError::business("失效日期必须晚于生效日期"));
            }
        }

        let rate_no = Self::generate_rate_no();
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RateActiveModel {
            id: Default::default(),
            rate_no: Set(rate_no),
            process_route_id: Set(req.process_route_id),
            route_code: Set(route.route_code.clone()),
            route_name: Set(route.route_name.clone()),
            wage_type: Set(wage_type_value),
            piece_price: Set(piece_price),
            time_price: Set(time_price),
            grade_a_ratio: Set(grade_a_ratio),
            grade_b_ratio: Set(grade_b_ratio),
            grade_c_ratio: Set(grade_c_ratio),
            effective_date: Set(req.effective_date),
            expiry_date: Set(req.expiry_date),
            workshop: Set(req.workshop),
            status: Set(wage_rate_status::DRAFT.to_string()),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工价创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新工价（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateWageRateRequest,
    ) -> Result<RateModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_rate_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        // 记录原 effective_date（在 model.into() 之前取出，避免 ActiveValue 取值复杂）
        let original_effective_date = model.effective_date;

        let mut active: RateActiveModel = model.into();

        if let Some(v) = req.wage_type {
            if v != wage_type::PIECE && v != wage_type::TIME && v != wage_type::MIXED {
                return Err(AppError::business("工价类型不合法"));
            }
            active.wage_type = Set(v);
        }
        if let Some(v) = req.piece_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("计件单价不能为负"));
            }
            active.piece_price = Set(v);
        }
        if let Some(v) = req.time_price {
            if v < Decimal::ZERO {
                return Err(AppError::business("计时单价不能为负"));
            }
            active.time_price = Set(v);
        }
        if let Some(v) = req.grade_a_ratio {
            if v < Decimal::ZERO || v > Decimal::new(10, 1) {
                return Err(AppError::business("A 级等级系数必须在 [0, 1] 范围内"));
            }
            active.grade_a_ratio = Set(v);
        }
        if let Some(v) = req.grade_b_ratio {
            if v < Decimal::ZERO || v > Decimal::new(10, 1) {
                return Err(AppError::business("B 级等级系数必须在 [0, 1] 范围内"));
            }
            active.grade_b_ratio = Set(v);
        }
        if let Some(v) = req.grade_c_ratio {
            if v < Decimal::ZERO || v > Decimal::new(10, 1) {
                return Err(AppError::business("C 级等级系数必须在 [0, 1] 范围内"));
            }
            active.grade_c_ratio = Set(v);
        }
        if let Some(v) = req.effective_date {
            active.effective_date = Set(v);
        }
        if let Some(v) = req.expiry_date {
            // 失效日期必须晚于生效日期（用原始 effective_date 或请求中的新 effective_date 比较）
            let effective = req.effective_date.unwrap_or(original_effective_date);
            if v <= effective {
                return Err(AppError::business("失效日期必须晚于生效日期"));
            }
            active.expiry_date = Set(Some(v));
        }
        if let Some(v) = req.workshop {
            active.workshop = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除工价（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_rate_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RateActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 启用工价（draft → active）
    pub async fn activate(&self, id: i32) -> Result<RateModel, AppError> {
        self.transition_status(id, wage_rate_status::DRAFT, wage_rate_status::ACTIVE)
            .await
    }

    /// 停用工价（active → disabled）
    pub async fn disable(&self, id: i32) -> Result<RateModel, AppError> {
        self.transition_status(id, wage_rate_status::ACTIVE, wage_rate_status::DISABLED)
            .await
    }

    /// 状态流转通用方法
    async fn transition_status(
        &self,
        id: i32,
        from: &str,
        to: &str,
    ) -> Result<RateModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != from {
            return Err(AppError::business(format!(
                "状态流转非法：当前 {}，期望 {}",
                model.status, from
            )));
        }
        let mut active: RateActiveModel = model.into();
        active.status = Set(to.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RateModel, AppError> {
        let model = RateEntity::find_by_id(id)
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工价 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按单号查询
    pub async fn get_by_no(&self, rate_no: &str) -> Result<RateModel, AppError> {
        let model = RateEntity::find()
            .filter(process_wage_rate::Column::RateNo.eq(rate_no))
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工价单号 {} 不存在", rate_no)))?;
        Ok(model)
    }

    /// 查询工序当前生效的工价
    pub async fn get_effective_by_route(
        &self,
        process_route_id: i32,
        on_date: chrono::NaiveDate,
    ) -> Result<Option<RateModel>, AppError> {
        let model = RateEntity::find()
            .filter(process_wage_rate::Column::ProcessRouteId.eq(process_route_id))
            .filter(process_wage_rate::Column::Status.eq(wage_rate_status::ACTIVE))
            .filter(process_wage_rate::Column::IsDeleted.eq(false))
            .filter(process_wage_rate::Column::EffectiveDate.lte(on_date))
            .filter(
                sea_orm::Condition::any()
                    .add(process_wage_rate::Column::ExpiryDate.is_null())
                    .add(process_wage_rate::Column::ExpiryDate.gt(on_date)),
            )
            .order_by_desc(process_wage_rate::Column::EffectiveDate)
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: WageRateQuery,
    ) -> Result<(Vec<RateModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RateEntity::find()
            .filter(process_wage_rate::Column::IsDeleted.eq(false));

        if let Some(v) = query.route_code {
            q = q.filter(process_wage_rate::Column::RouteCode.eq(v));
        }
        if let Some(v) = query.process_route_id {
            q = q.filter(process_wage_rate::Column::ProcessRouteId.eq(v));
        }
        if let Some(v) = query.workshop {
            q = q.filter(process_wage_rate::Column::Workshop.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(process_wage_rate::Column::Status.eq(v));
        }

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(process_wage_rate::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

// ============================================================================
// 工资记录 + 工资计算 Service
// ============================================================================

/// 创建工资记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWageRecordRequest {
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub workshop: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工资记录请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWageRecordRequest {
    pub workshop: Option<String>,
    pub remarks: Option<String>,
}

/// 工资计算请求（触发计算）
#[derive(Debug, Clone, Deserialize)]
pub struct CalculateWageRequest {
    /// 重新计算（删除已有明细重新生成）
    pub recalculate: Option<bool>,
}

/// 工资记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct WageRecordQuery {
    pub record_no: Option<String>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub period_start: Option<chrono::NaiveDate>,
    pub period_end: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 工资记录 Service
pub struct WageRecordService {
    db: Arc<DatabaseConnection>,
}

impl WageRecordService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成工资单号：WR-YYYYMM-NNN
    fn generate_record_no(period: chrono::NaiveDate) -> String {
        let ym = period.format("%Y%m");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("WR-{}-{:03}", ym, random)
    }

    /// 创建工资记录（仅创建空记录，需调用 calculate 触发计算）
    pub async fn create(&self, req: CreateWageRecordRequest) -> Result<RecordModel, AppError> {
        // 业务校验：周期结束必须 ≥ 周期开始
        if req.period_end < req.period_start {
            return Err(AppError::business("周期结束日期必须 ≥ 周期开始日期"));
        }

        let record_no = Self::generate_record_no(req.period_start);
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RecordActiveModel {
            id: Default::default(),
            record_no: Set(record_no),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            workshop: Set(req.workshop),
            total_workers: Set(0),
            total_step_records: Set(0),
            total_qualified_quantity: Set(Decimal::ZERO),
            total_duration_minutes: Set(0),
            total_amount: Set(Decimal::ZERO),
            status: Set(wage_record_status::DRAFT.to_string()),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            paid_by: Set(None),
            paid_at: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工资记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新工资记录（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateWageRecordRequest,
    ) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: RecordActiveModel = model.into();

        if let Some(v) = req.workshop {
            active.workshop = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除工资记录（仅 draft 状态可删除，连带删除明细）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RecordActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 确认工资（draft → confirmed）
    pub async fn confirm(&self, id: i32, confirmed_by: i32) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可确认，当前状态: {}",
                model.status
            )));
        }
        // 业务校验：必须有明细才能确认
        let detail_count = wage_record_detail::Entity::find()
            .filter(wage_record_detail::Column::WageRecordId.eq(id))
            .filter(wage_record_detail::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await?;
        if detail_count == 0 {
            return Err(AppError::business("工资记录无明细，请先调用 calculate 触发计算"));
        }

        let mut active: RecordActiveModel = model.into();
        active.status = Set(wage_record_status::CONFIRMED.to_string());
        active.confirmed_by = Set(Some(confirmed_by));
        active.confirmed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 发放工资（confirmed → paid）
    pub async fn pay(&self, id: i32, paid_by: i32) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::CONFIRMED {
            return Err(AppError::business(format!(
                "仅已确认(confirmed)状态可发放，当前状态: {}",
                model.status
            )));
        }
        let mut active: RecordActiveModel = model.into();
        active.status = Set(wage_record_status::PAID.to_string());
        active.paid_by = Set(Some(paid_by));
        active.paid_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消工资（draft/confirmed → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT && model.status != wage_record_status::CONFIRMED
        {
            return Err(AppError::business(format!(
                "仅草稿(draft)或已确认(confirmed)状态可取消，当前状态: {}",
                model.status
            )));
        }
        let mut active: RecordActiveModel = model.into();
        active.status = Set(wage_record_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RecordModel, AppError> {
        let model = RecordEntity::find_by_id(id)
            .filter(wage_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按单号查询
    pub async fn get_by_no(&self, record_no: &str) -> Result<RecordModel, AppError> {
        let model = RecordEntity::find()
            .filter(wage_record::Column::RecordNo.eq(record_no))
            .filter(wage_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资单号 {} 不存在", record_no)))?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: WageRecordQuery,
    ) -> Result<(Vec<RecordModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RecordEntity::find().filter(wage_record::Column::IsDeleted.eq(false));

        if let Some(v) = query.record_no {
            q = q.filter(wage_record::Column::RecordNo.like(format!("%{}%", v)));
        }
        if let Some(v) = query.workshop {
            q = q.filter(wage_record::Column::Workshop.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(wage_record::Column::Status.eq(v));
        }
        if let Some(v) = query.period_start {
            q = q.filter(wage_record::Column::PeriodStart.gte(v));
        }
        if let Some(v) = query.period_end {
            q = q.filter(wage_record::Column::PeriodEnd.lte(v));
        }

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(wage_record::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}

// ============================================================================
// 工资计算 Service
// ============================================================================

/// 工资计算 Service
///
/// 真实业务：按周期 + 车间查询工序记录 → 按工序匹配生效工价 → 计算每个工人的应得工资
pub struct WageCalculationService {
    db: Arc<DatabaseConnection>,
}

impl WageCalculationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 触发工资计算
    ///
    /// 业务流程：
    /// 1. 查询周期内 completed 状态的工序记录
    /// 2. 按工序路线匹配生效工价
    /// 3. 计算每个工人每道工序的工资
    /// 4. 生成工资明细 + 汇总工资记录
    pub async fn calculate(
        &self,
        wage_record_id: i32,
        req: CalculateWageRequest,
    ) -> Result<RecordModel, AppError> {
        let record = RecordEntity::find_by_id(wage_record_id)
            .filter(wage_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资记录 {} 不存在", wage_record_id)))?;

        // 业务校验：仅 draft 状态可计算
        if record.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可触发计算，当前状态: {}",
                record.status
            )));
        }

        // 重新计算时先删除旧明细
        if req.recalculate.unwrap_or(false) {
            wage_record_detail::Entity::delete_many()
                .filter(wage_record_detail::Column::WageRecordId.eq(wage_record_id))
                .exec(&*self.db)
                .await?;
        } else {
            // 不重新计算时，检查是否已有明细
            let existing = wage_record_detail::Entity::find()
                .filter(wage_record_detail::Column::WageRecordId.eq(wage_record_id))
                .filter(wage_record_detail::Column::IsDeleted.eq(false))
                .count(&*self.db)
                .await?;
            if existing > 0 {
                return Err(AppError::business(format!(
                    "工资记录已有 {} 条明细，如需重新计算请设置 recalculate=true",
                    existing
                )));
            }
        }

        // 1. 查询周期内 completed 状态的工序记录
        // 工序记录无 workshop 字段，车间维度在工价匹配阶段通过 process_wage_rate.workshop 间接关联
        let period_start_tz = naive_date_to_date_time_tz(record.period_start);
        let period_end_tz = naive_date_to_end_of_day_tz(record.period_end);
        let step_query = StepEntity::find()
            .filter(process_step_record::Column::IsDeleted.eq(false))
            .filter(process_step_record::Column::Status.eq("completed"))
            // 周期内：start_at 在 [period_start 00:00, period_end 23:59]
            .filter(process_step_record::Column::StartAt.gte(period_start_tz))
            .filter(process_step_record::Column::StartAt.lte(period_end_tz));

        let step_records: Vec<StepModel> = step_query.all(&*self.db).await?;

        if step_records.is_empty() {
            return Err(AppError::business(format!(
                "周期 {} ~ {} 内无已完成的工序记录，无法计算工资",
                record.period_start, record.period_end
            )));
        }

        // 2. 按工序路线匹配生效工价，生成工资明细
        let now = crate::utils::date_utils::utc_now_fixed();
        let mut total_amount = Decimal::ZERO;
        let mut total_qualified = Decimal::ZERO;
        let mut total_minutes: i64 = 0;
        let mut worker_set: HashSet<i32> = HashSet::new();
        let mut step_set: HashSet<i32> = HashSet::new();
        let mut detail_count: u64 = 0;

        for step in &step_records {
            // 查找工价（按工序路线匹配）
            let route_id = match step.process_route_id {
                Some(id) => id,
                None => continue, // 无工序路线的记录跳过
            };

            // 工价匹配条件：工序路线 + 当前生效 + 车间过滤
            // effective_date 和 expiry_date 是 NaiveDate 类型，直接用 NaiveDate 比较
            let mut rate_query = RateEntity::find()
                .filter(process_wage_rate::Column::ProcessRouteId.eq(route_id))
                .filter(process_wage_rate::Column::Status.eq(wage_rate_status::ACTIVE))
                .filter(process_wage_rate::Column::IsDeleted.eq(false))
                .filter(process_wage_rate::Column::EffectiveDate.lte(record.period_end))
                .filter(
                    sea_orm::Condition::any()
                        .add(process_wage_rate::Column::ExpiryDate.is_null())
                        .add(process_wage_rate::Column::ExpiryDate.gt(record.period_start)),
                );

            if let Some(ref workshop) = record.workshop {
                rate_query = rate_query.filter(
                    sea_orm::Condition::any()
                        .add(process_wage_rate::Column::Workshop.is_null())
                        .add(process_wage_rate::Column::Workshop.eq(workshop)),
                );
            }

            let rate = match rate_query
                .order_by_desc(process_wage_rate::Column::EffectiveDate)
                .one(&*self.db)
                .await?
            {
                Some(r) => r,
                None => continue, // 无生效工价的工序跳过
            };

            // 3. 计算工资
            let (grade, grade_ratio, piece_wage, time_wage, wage_amount) = calculate_wage_for_step(
                &rate,
                step.actual_quantity,
                step.qualified_quantity,
                step.duration_minutes,
            );

            // 4. 按工人 IDs 分配工资（多人共同完成时按人均分配）
            let worker_ids = parse_worker_ids(&step.worker_ids);
            let worker_names = parse_worker_names(&step.worker_names);
            if worker_ids.is_empty() {
                continue; // 无工人的记录跳过
            }

            let worker_count = worker_ids.len();
            let per_worker_piece = split_wage_among_workers(piece_wage, worker_count);
            let per_worker_time = split_wage_among_workers(time_wage, worker_count);
            let per_worker_amount = split_wage_among_workers(wage_amount, worker_count);

            for (idx, &worker_id) in worker_ids.iter().enumerate() {
                let worker_name = worker_names.get(idx).cloned();

                let detail = DetailActiveModel {
                    id: Default::default(),
                    wage_record_id: Set(wage_record_id),
                    step_record_id: Set(step.id),
                    flow_card_id: Set(Some(step.flow_card_id)),
                    dye_lot_no: Set(None), // dye_lot_no 在 production_flow_card 上，这里留空
                    process_route_id: Set(step.process_route_id),
                    route_code: Set(Some(step.route_code.clone())),
                    route_name: Set(Some(step.route_name.clone())),
                    process_type: Set(Some(step.process_type.clone())),
                    worker_id: Set(worker_id),
                    worker_name: Set(worker_name),
                    equipment_id: Set(step.equipment_id),
                    equipment_name: Set(step.equipment_name.clone()),
                    wage_type: Set(rate.wage_type.clone()),
                    grade: Set(grade.clone()),
                    actual_quantity: Set(step.actual_quantity.unwrap_or(Decimal::ZERO)
                        / Decimal::from(worker_count)),
                    qualified_quantity: Set(step.qualified_quantity.unwrap_or(Decimal::ZERO)
                        / Decimal::from(worker_count)),
                    qualification_rate: Set(compute_qualification_rate(
                        step.actual_quantity,
                        step.qualified_quantity,
                    )),
                    piece_price: Set(rate.piece_price),
                    time_price: Set(rate.time_price),
                    grade_ratio: Set(grade_ratio),
                    duration_minutes: Set(step.duration_minutes.unwrap_or(0) / worker_count as i32),
                    piece_wage: Set(per_worker_piece),
                    time_wage: Set(per_worker_time),
                    wage_amount: Set(per_worker_amount),
                    remarks: Set(None),
                    is_deleted: Set(false),
                    created_at: Set(now),
                    updated_at: Set(now),
                };
                detail.insert(&*self.db).await?;
                detail_count += 1;
                total_amount += per_worker_amount;
                total_qualified += step.qualified_quantity.unwrap_or(Decimal::ZERO)
                    / Decimal::from(worker_count);
                total_minutes += (step.duration_minutes.unwrap_or(0) as i64) / worker_count as i64;
                worker_set.insert(worker_id);
                step_set.insert(step.id);
            }
        }

        if detail_count == 0 {
            return Err(AppError::business(
                "未生成任何工资明细，请检查工价方案配置或工序记录状态",
            ));
        }

        // 5. 更新工资记录汇总
        let mut active: RecordActiveModel = record.into();
        active.total_workers = Set(worker_set.len() as i32);
        active.total_step_records = Set(step_set.len() as i32);
        active.total_qualified_quantity = Set(total_qualified);
        active.total_duration_minutes = Set(total_minutes as i32);
        active.total_amount = Set(total_amount);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::ToPrimitive;

    // ===== compute_qualification_rate 合格率计算 =====

    /// 测试_合格率计算_正常情况
    ///
    /// 验证 actual=100, qualified=95 时合格率为 95%。
    #[test]
    fn 测试_合格率计算_正常情况() {
        let rate = compute_qualification_rate(
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(95, 0)),
        );
        assert_eq!(rate, Decimal::new(95, 0));
    }

    /// 测试_合格率计算_全合格
    ///
    /// 验证 actual=100, qualified=100 时合格率为 100%。
    #[test]
    fn 测试_合格率计算_全合格() {
        let rate = compute_qualification_rate(
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(100, 0)),
        );
        assert_eq!(rate, Decimal::new(100, 0));
    }

    /// 测试_合格率计算_零产量
    ///
    /// 验证 actual=0 时合格率为 0（避免除零错误）。
    #[test]
    fn 测试_合格率计算_零产量() {
        let rate = compute_qualification_rate(Some(Decimal::ZERO), Some(Decimal::ZERO));
        assert_eq!(rate, Decimal::ZERO);
    }

    /// 测试_合格率计算_None按零处理
    ///
    /// 验证 None 时按 0 处理。
    #[test]
    fn 测试_合格率计算_None按零处理() {
        let rate = compute_qualification_rate(None, None);
        assert_eq!(rate, Decimal::ZERO);
    }

    // ===== determine_grade_by_qualification_rate 等级判定 =====

    /// 测试_等级判定_A级_95以上
    ///
    /// 验证合格率 ≥ 95% 判定为 A 级。
    #[test]
    fn 测试_等级判定_A级_95以上() {
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(95, 0)),
            QUALITY_GRADE_A
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(100, 0)),
            QUALITY_GRADE_A
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(995, 1)), // 99.5
            QUALITY_GRADE_A
        );
    }

    /// 测试_等级判定_B级_80到95区间
    ///
    /// 验证合格率 80-95% 判定为 B 级。
    #[test]
    fn 测试_等级判定_B级_80到95区间() {
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(80, 0)),
            QUALITY_GRADE_B
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(85, 0)),
            QUALITY_GRADE_B
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(9499, 2)), // 94.99
            QUALITY_GRADE_B
        );
    }

    /// 测试_等级判定_C级_80以下
    ///
    /// 验证合格率 < 80% 判定为 C 级。
    #[test]
    fn 测试_等级判定_C级_80以下() {
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(79, 0)),
            QUALITY_GRADE_C
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(50, 0)),
            QUALITY_GRADE_C
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::ZERO),
            QUALITY_GRADE_C
        );
    }

    // ===== determine_grade_ratio 等级系数获取 =====

    /// 测试_等级系数获取_各级别
    ///
    /// 验证 A/B/C 级返回对应的工价等级系数。
    #[test]
    fn 测试_等级系数获取_各级别() {
        // 构造一个 Mock 工价模型
        let rate = RateModel {
            id: 1,
            rate_no: "PWR-TEST-001".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0), // 5 元/kg
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1), // 1.0
            grade_b_ratio: Decimal::new(8, 1),  // 0.8
            grade_c_ratio: Decimal::ZERO,
            effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            expiry_date: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(
            determine_grade_ratio(QUALITY_GRADE_A, &rate),
            Decimal::new(10, 1)
        );
        assert_eq!(
            determine_grade_ratio(QUALITY_GRADE_B, &rate),
            Decimal::new(8, 1)
        );
        assert_eq!(
            determine_grade_ratio(QUALITY_GRADE_C, &rate),
            Decimal::ZERO
        );
        // 未知等级返回 0
        assert_eq!(determine_grade_ratio("X", &rate), Decimal::ZERO);
    }

    // ===== calculate_wage_for_step 工资计算 =====

    /// 测试_工资计算_计件_A级全额
    ///
    /// 验证计件工价 + A 级（100%合格率）= 合格产量 × 计件单价 × 1.0。
    #[test]
    fn 测试_工资计算_计件_A级全额() {
        let rate = RateModel {
            id: 1,
            rate_no: "PWR-TEST-002".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0), // 5 元/kg
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            expiry_date: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // actual=100kg, qualified=100kg, 100% 合格率 → A 级
        let (grade, ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(100, 0)),
            Some(120),
        );

        assert_eq!(grade, QUALITY_GRADE_A);
        assert_eq!(ratio, Decimal::new(10, 1));
        assert_eq!(piece_wage, Decimal::new(500, 0)); // 100 × 5 × 1.0 = 500
        assert_eq!(time_wage, Decimal::ZERO); // 计件类型，计时为 0
        assert_eq!(total, Decimal::new(500, 0));
    }

    /// 测试_工资计算_计件_B级8折
    ///
    /// 验证计件工价 + B 级（85%合格率）= 合格产量 × 计件单价 × 0.8。
    #[test]
    fn 测试_工资计算_计件_B级8折() {
        let rate = RateModel {
            id: 2,
            rate_no: "PWR-TEST-003".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0),
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            expiry_date: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // actual=100kg, qualified=85kg, 85% 合格率 → B 级
        let (grade, ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(85, 0)),
            Some(120),
        );

        assert_eq!(grade, QUALITY_GRADE_B);
        assert_eq!(ratio, Decimal::new(8, 1));
        // 85 × 5 × 0.8 = 340
        assert_eq!(piece_wage, Decimal::new(340, 0));
        assert_eq!(time_wage, Decimal::ZERO);
        assert_eq!(total, Decimal::new(340, 0));
    }

    /// 测试_工资计算_计件_C级不计
    ///
    /// 验证计件工价 + C 级（50%合格率）= 工资为 0。
    #[test]
    fn 测试_工资计算_计件_C级不计() {
        let rate = RateModel {
            id: 3,
            rate_no: "PWR-TEST-004".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0),
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            expiry_date: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // actual=100kg, qualified=50kg, 50% 合格率 → C 级
        let (grade, ratio, piece_wage, _time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(50, 0)),
            Some(120),
        );

        assert_eq!(grade, QUALITY_GRADE_C);
        assert_eq!(ratio, Decimal::ZERO);
        assert_eq!(piece_wage, Decimal::ZERO);
        assert_eq!(total, Decimal::ZERO);
    }

    /// 测试_工资计算_计时_按工时
    ///
    /// 验证计时工价 = 工时 × 计时单价 × 等级系数。
    #[test]
    fn 测试_工资计算_计时_按工时() {
        let rate = RateModel {
            id: 4,
            rate_no: "PWR-TEST-005".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::TIME.to_string(),
            piece_price: Decimal::ZERO,
            time_price: Decimal::new(2, 0), // 2 元/分钟
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            expiry_date: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // 100% 合格率 → A 级，120 分钟
        let (_grade, _ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(100, 0)),
            Some(120),
        );

        // 120 × 2 × 1.0 = 240
        assert_eq!(piece_wage, Decimal::ZERO); // 计时类型，计件为 0
        assert_eq!(time_wage, Decimal::new(240, 0));
        assert_eq!(total, Decimal::new(240, 0));
    }

    /// 测试_工资计算_混合_计件加计时
    ///
    /// 验证混合工价 = 计件 + 计时。
    #[test]
    fn 测试_工资计算_混合_计件加计时() {
        let rate = RateModel {
            id: 5,
            rate_no: "PWR-TEST-006".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::MIXED.to_string(),
            piece_price: Decimal::new(5, 0),
            time_price: Decimal::new(2, 0),
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            expiry_date: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // 85% 合格率 → B 级，100kg 合格产量，120 分钟
        let (_grade, _ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(85, 0)),
            Some(120),
        );

        // piece_wage = 85 × 5 × 0.8 = 340
        // time_wage = 120 × 2 × 0.8 = 192
        assert_eq!(piece_wage, Decimal::new(340, 0));
        assert_eq!(time_wage, Decimal::new(192, 0));
        assert_eq!(total, Decimal::new(532, 0)); // 340 + 192
    }

    // ===== parse_worker_ids 工人IDs解析 =====

    /// 测试_工人IDs解析_正常情况
    #[test]
    fn 测试_工人IDs解析_正常情况() {
        let ids = parse_worker_ids(&Some("1,2,3".to_string()));
        assert_eq!(ids, vec![1, 2, 3]);
    }

    /// 测试_工人IDs解析_带空格
    #[test]
    fn 测试_工人IDs解析_带空格() {
        let ids = parse_worker_ids(&Some("1, 2, 3".to_string()));
        assert_eq!(ids, vec![1, 2, 3]);
    }

    /// 测试_工人IDs解析_空值
    #[test]
    fn 测试_工人IDs解析_空值() {
        assert!(parse_worker_ids(&None).is_empty());
        assert!(parse_worker_ids(&Some(String::new())).is_empty());
        assert!(parse_worker_ids(&Some("  ".to_string())).is_empty());
    }

    /// 测试_工人IDs解析_非法值过滤
    #[test]
    fn 测试_工人IDs解析_非法值过滤() {
        let ids = parse_worker_ids(&Some("1,abc,3,".to_string()));
        assert_eq!(ids, vec![1, 3]);
    }

    // ===== split_wage_among_workers 工资按人均分配 =====

    /// 测试_工资按人均分配_单人
    #[test]
    fn 测试_工资按人均分配_单人() {
        let wage = Decimal::new(500, 0);
        assert_eq!(split_wage_among_workers(wage, 1), Decimal::new(500, 0));
    }

    /// 测试_工资按人均分配_多人整除
    #[test]
    fn 测试_工资按人均分配_多人整除() {
        let wage = Decimal::new(500, 0);
        assert_eq!(split_wage_among_workers(wage, 5), Decimal::new(100, 0));
    }

    /// 测试_工资按人均分配_零人
    #[test]
    fn 测试_工资按人均分配_零人() {
        let wage = Decimal::new(500, 0);
        assert_eq!(split_wage_among_workers(wage, 0), Decimal::ZERO);
    }

    /// 测试_工资按人均分配_非整除取小数
    #[test]
    fn 测试_工资按人均分配_非整除取小数() {
        let wage = Decimal::new(100, 0);
        // 100 / 3 = 33.33...
        let result = split_wage_among_workers(wage, 3);
        let f = result.to_f64().unwrap();
        assert!((f - 33.3333).abs() < 0.01);
    }
}

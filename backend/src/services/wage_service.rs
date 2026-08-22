//! 产量工资 Service（facade）
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
//!
//! 批次 490 D10-4a 拆分：本文件作为 facade，保留 9 个工资计算纯函数 + 3 个 Service struct
//! + new 构造函数 + 7 个 DTOs + 单元测试。3 个 Service 的 impl 块迁移至 `wage_ops` 子模块
//!（rate / record / calculation），通过 db 字段 pub(crate) 让 ops 访问，外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::process_wage_rate::Model as RateModel;
use crate::models::status::wage_type;
use crate::services::quality_inspection_service::{
    QUALITY_GRADE_A, QUALITY_GRADE_B, QUALITY_GRADE_C, determine_quality_grade,
};

// 工资 DTO 已迁移至 models/dto/wage_dto.rs，此处 re-export 保持外部引用路径不变
pub use crate::models::dto::wage_dto::*;

// ============================================================================
// 工资计算纯函数
// ============================================================================

/// 将 NaiveDate 转换为带时区的 DateTime（当天 00:00:00 UTC）（用于工序记录的 start_at 字段比较）
pub(crate) fn naive_date_to_date_time_tz(
    date: chrono::NaiveDate,
) -> chrono::DateTime<chrono::FixedOffset> {
    let naive_time = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let naive_date_time = chrono::NaiveDateTime::new(date, naive_time);
    chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(
        naive_date_time,
        chrono::FixedOffset::east_opt(0).unwrap(),
    )
}

/// 将 NaiveDate 转换为带时区的当天 23:59:59（用于区间右边界）
pub(crate) fn naive_date_to_end_of_day_tz(
    date: chrono::NaiveDate,
) -> chrono::DateTime<chrono::FixedOffset> {
    let naive_time = chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap();
    let naive_date_time = chrono::NaiveDateTime::new(date, naive_time);
    chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(
        naive_date_time,
        chrono::FixedOffset::east_opt(0).unwrap(),
    )
}

/// 计算合格率（百分比，0-100）（业务规则：若实际产量为 0 或 None，合格率为 0；若合格产量为 None，按 0 处理；公式：qualified_quantity / actual_quantity × 100）
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
/// 业务规则（复用批次 421 determine_quality_grade）：合格率 ≥ 95% → A 级（合格）；80% ≤ 合格率 < 95% → B 级（让步接收）；合格率 < 80% → C 级（不合格）
pub fn determine_grade_by_qualification_rate(rate: Decimal) -> String {
    determine_quality_grade(Some(rate))
}

/// 依据质检等级返回工价等级系数（业务规则：A 级：grade_a_ratio（默认全额 1.0）；B 级：grade_b_ratio（默认 8 折 0.8）；C 级：grade_c_ratio（默认不计 0.0））
pub fn determine_grade_ratio(grade: &str, rate_model: &RateModel) -> Decimal {
    match grade {
        QUALITY_GRADE_A => rate_model.grade_a_ratio,
        QUALITY_GRADE_B => rate_model.grade_b_ratio,
        QUALITY_GRADE_C => rate_model.grade_c_ratio,
        _ => Decimal::ZERO,
    }
}

/// 计算单条工序记录的工资明细
/// 业务规则：计件工资 = 合格产量 × 计件单价 × 等级系数；计时工资 = 工时（分钟） × 计时单价 × 等级系数；应得工资 = 计件工资 + 计时工资 + 加班费（根据 wage_type 选择）；参数：rate: 工价方案；actual_quantity: 实际产量；qualified_quantity: 合格产量；duration_minutes: 工时（分钟）；返回：(grade, grade_ratio, piece_wage, time_wage, wage_amount)
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

/// V15 P1-08-22 加班费计算（《劳动法》第 44 条）
/// 业务规则：工作日加班：按计时单价的 1.5 倍计算；休息日加班：按计时单价的 2 倍计算（不能安排补休时）；法定节假日加班：按计时单价的 3 倍计算；加班费 = (weekday_ot × 1.5 + weekend_ot × 2 + holiday_ot × 3) × time_price × grade_ratio / 60；（time_price 单位为元/分钟，故除以 60 转换为小时费率更直观，但此处直接用分钟费率）；参数：rate: 工价方案（取 time_price 与 grade_ratio）；grade_ratio: 等级系数（与计件/计时工资一致）；weekday_overtime_minutes: 工作日加班工时（分钟）；weekend_overtime_minutes: 休息日加班工时（分钟）；holiday_overtime_minutes: 法定节假日加班工时（分钟）；返回：加班费总额（Decimal）
pub fn calculate_overtime_pay(
    rate: &RateModel,
    grade_ratio: Decimal,
    weekday_overtime_minutes: i32,
    weekend_overtime_minutes: i32,
    holiday_overtime_minutes: i32,
) -> Decimal {
    // 《劳动法》第 44 条加班倍率
    let weekday_ot_multiplier = Decimal::new(15, 1); // 1.5
    let weekend_ot_multiplier = Decimal::new(2, 0); // 2.0
    let holiday_ot_multiplier = Decimal::new(3, 0); // 3.0

    let weekday_ot = Decimal::from(weekday_overtime_minutes) * weekday_ot_multiplier;
    let weekend_ot = Decimal::from(weekend_overtime_minutes) * weekend_ot_multiplier;
    let holiday_ot = Decimal::from(holiday_overtime_minutes) * holiday_ot_multiplier;

    let total_ot_minutes = weekday_ot + weekend_ot + holiday_ot;
    total_ot_minutes * rate.time_price * grade_ratio
}

/// 解析工序记录的工人 IDs（逗号分隔字符串 → HashSet）（真实业务：扫码登记工人时，可能多个工人共同完成一道工序；工资按人均分配（简化方案，实际业务可按工时比例分配））
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

/// 按人均分配工资（多人共同完成一道工序时）（公式：单人工资 = 总工资 / 工人数量）
pub fn split_wage_among_workers(wage: Decimal, worker_count: usize) -> Decimal {
    if worker_count == 0 {
        return Decimal::ZERO;
    }
    wage / Decimal::from(worker_count)
}

// ============================================================================
// 工序工价 Service struct 定义（impl 块在 wage_ops/rate 子模块）
// ============================================================================

/// 工序工价 Service
pub struct WageRateService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl WageRateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 工资记录 Service struct 定义（impl 块在 wage_ops/record 子模块）
// ============================================================================

/// 工资记录 Service
pub struct WageRecordService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl WageRecordService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 工资计算 Service struct 定义（impl 块在 wage_ops/calculation 子模块）
// ============================================================================

/// 工资计算 Service（真实业务：按周期 + 车间查询工序记录 → 按工序匹配生效工价 → 计算每个工人的应得工资）
pub struct WageCalculationService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl WageCalculationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

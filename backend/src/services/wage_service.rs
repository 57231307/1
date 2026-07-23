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
use serde::Deserialize;
use std::sync::Arc;

use crate::models::process_wage_rate::Model as RateModel;
use crate::models::status::wage_type;
use crate::services::quality_inspection_service::{
    determine_quality_grade, QUALITY_GRADE_A, QUALITY_GRADE_B, QUALITY_GRADE_C,
};

// ============================================================================
// 工资计算纯函数
// ============================================================================

/// 将 NaiveDate 转换为带时区的 DateTime（当天 00:00:00 UTC）
///
/// 用于工序记录的 start_at 字段比较
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
// 工序工价 Service struct 定义（impl 块在 wage_ops/rate 子模块）
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

/// 工资计算 Service
///
/// 真实业务：按周期 + 车间查询工序记录 → 按工序匹配生效工价 → 计算每个工人的应得工资
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::status::wage_rate_status;
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

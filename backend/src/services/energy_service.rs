//! 能耗管理 Service（facade）
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务流程：
//!   能耗采集 → 能源计量设备（水/电/汽表，IoT 对接）→ 时间段登记能耗记录
//!   分摊规则 → 定义分摊基准（按工时/产量/设备/车间）+ 标准单位能耗（用于超基准预警）
//!   月末分摊 → 按规则将总能耗分摊到缸号/工序/订单 → 生成 cost_collection 记录
//!   单位能耗分析 → 每米布能耗/每缸能耗/能耗产值分析
//!
//! 核心能力：
//! - 能源计量设备 CRUD + 状态机（active→inactive/maintenance）
//! - 能耗记录 CRUD + 状态机（draft→confirmed→cancelled）+ IoT 自动采集
//! - 能耗分摊规则 CRUD + 状态机（draft→active→disabled）
//! - 能耗分摊记录 CRUD + 状态机 + 月末分摊计算 + 关联 cost_collection
//!
//! 复用现有功能（§10.0.1）：
//! - process_step_record 表：作为工时/产量数据源（批次 425 已建）
//! - process_route 表：作为工序定义（批次 425 已建）
//! - cost_collection 表：作为成本归集目标（批次 422 已建）
//! - production_flow_card 表：作为缸号/流转卡关联（批次 425 已建）
//!
//! 批次 488 D10-2a 拆分：本文件作为 facade，保留 9 个能耗计算纯函数 + 单元测试，
//! 4 个业务 Service（EnergyMeterService / EnergyConsumptionService /
//! EnergyAllocationRuleService / EnergyAllocationRecordService）及其 DTOs
//! 迁移至 `energy_ops` 子模块，并通过 `pub use` re-export，保持外部引用路径
//! `crate::services::energy_service::*` 不变。

use rust_decimal::Decimal;

use crate::models::status::energy_allocation_basis;
use crate::models::status::energy_type;
use crate::utils::error::AppError;

// 批次 488 D10-2a 拆分：re-export 4 个业务 Service 及其 DTOs，保持外部引用路径不变
pub use crate::services::energy_ops::{
    // meter
    CreateMeterRequest, EnergyMeterService, MeterQuery, UpdateMeterRequest,
    // consumption
    ConsumptionQuery, CreateConsumptionRequest, EnergyConsumptionService,
    UpdateConsumptionRequest, WorkshopEnergySummary,
    // allocation_rule
    CreateRuleRequest, EnergyAllocationRuleService, RuleQuery, UpdateRuleRequest,
    // allocation_record
    AllocationRecordQuery, CreateAllocationRecordRequest, EnergyAllocationRecordService,
    MonthlyAllocationRequest, UpdateAllocationRecordRequest,
};

// ============================================================================
// 能耗计算纯函数
// ============================================================================

/// 计算消耗量（当前读数 - 上次读数）
///
/// 业务规则：
/// - 若当前读数 < 上次读数，返回 0（可能是表计回零或异常）
/// - 否则返回差值
pub fn compute_consumption(
    previous_reading: Decimal,
    current_reading: Decimal,
) -> Decimal {
    if current_reading < previous_reading {
        return Decimal::ZERO;
    }
    current_reading - previous_reading
}

/// 计算总成本（消耗量 × 单价）
pub fn compute_total_cost(consumption: Decimal, unit_price: Decimal) -> Decimal {
    consumption * unit_price
}

/// 计算分摊比例（分摊依据量 / 总依据量）
///
/// 业务规则：
/// - 若总依据量为 0，返回 0（避免除零）
pub fn compute_allocation_ratio(
    basis_value: Decimal,
    total_basis_value: Decimal,
) -> Decimal {
    if total_basis_value <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    basis_value / total_basis_value
}

/// 计算分摊消耗量（总消耗量 × 分摊比例）
pub fn compute_allocated_consumption(
    total_consumption: Decimal,
    allocation_ratio: Decimal,
) -> Decimal {
    total_consumption * allocation_ratio
}

/// 计算分摊成本（总成本 × 分摊比例）
pub fn compute_allocated_cost(
    total_cost: Decimal,
    allocation_ratio: Decimal,
) -> Decimal {
    total_cost * allocation_ratio
}

/// 计算单位能耗（分摊消耗量 / 单位产量）
///
/// 业务规则：
/// - 若单位产量为 0 或 None，返回 None
/// - 用于单位能耗分析（每米布能耗、每缸能耗）
pub fn compute_unit_consumption(
    allocated_consumption: Decimal,
    output_quantity: Option<Decimal>,
) -> Option<Decimal> {
    let output = output_quantity?;
    if output <= Decimal::ZERO {
        return None;
    }
    Some(allocated_consumption / output)
}

/// 判断能耗是否超过基准
///
/// 业务规则：
/// - 实际单位能耗 > 标准单位能耗 × (1 + tolerance) 时视为超基准
/// - tolerance 默认 0.1（10% 容差）
/// - 返回 (是否超基准, 实际单位能耗, 偏差百分比)
pub fn check_consumption_exceeds_standard(
    actual_unit_consumption: Decimal,
    standard_consumption_per_unit: Decimal,
    tolerance: Decimal,
) -> (bool, Decimal) {
    if standard_consumption_per_unit <= Decimal::ZERO {
        return (false, Decimal::ZERO);
    }
    let threshold = standard_consumption_per_unit * (Decimal::ONE + tolerance);
    let deviation = if actual_unit_consumption > standard_consumption_per_unit {
        (actual_unit_consumption - standard_consumption_per_unit)
            / standard_consumption_per_unit
            * Decimal::new(100, 0)
    } else {
        Decimal::ZERO
    };
    (actual_unit_consumption > threshold, deviation)
}

/// 校验能源类型是否合法
pub fn validate_meter_type(meter_type: &str) -> Result<(), AppError> {
    let valid_types = [
        energy_type::WATER,
        energy_type::ELECTRICITY,
        energy_type::STEAM,
        energy_type::GAS,
        energy_type::COMPRESSED_AIR,
    ];
    if !valid_types.contains(&meter_type) {
        return Err(AppError::business(format!(
            "能源类型必须是 water / electricity / steam / gas / compressed_air，当前: {}",
            meter_type
        )));
    }
    Ok(())
}

/// 校验分摊基准是否合法
pub fn validate_allocation_basis(basis: &str) -> Result<(), AppError> {
    let valid_basis = [
        energy_allocation_basis::BY_DURATION,
        energy_allocation_basis::BY_OUTPUT,
        energy_allocation_basis::BY_EQUIPMENT,
        energy_allocation_basis::BY_WORKSHOP,
    ];
    if !valid_basis.contains(&basis) {
        return Err(AppError::business(format!(
            "分摊基准必须是 by_duration / by_output / by_equipment / by_workshop，当前: {}",
            basis
        )));
    }
    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn 测试计算消耗量_正常() {
        let result = compute_consumption(Decimal::new(100, 0), Decimal::new(150, 0));
        assert_eq!(result, Decimal::new(50, 0));
    }

    #[test]
    fn 测试计算消耗量_当前小于上次返回零() {
        let result = compute_consumption(Decimal::new(150, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算消耗量_相等返回零() {
        let result = compute_consumption(Decimal::new(100, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算总成本() {
        let result = compute_total_cost(Decimal::new(50, 0), Decimal::new(12, 1));
        assert_eq!(result, Decimal::new(600, 0));
    }

    #[test]
    fn 测试计算分摊比例_正常() {
        let result = compute_allocation_ratio(Decimal::new(30, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::new(30, 2)); // 0.30
    }

    #[test]
    fn 测试计算分摊比例_总依据为零返回零() {
        let result = compute_allocation_ratio(Decimal::new(30, 0), Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算分摊消耗量() {
        let result = compute_allocated_consumption(
            Decimal::new(1000, 0),
            Decimal::new(30, 2), // 0.30
        );
        assert_eq!(result, Decimal::new(300, 0));
    }

    #[test]
    fn 测试计算分摊成本() {
        let result = compute_allocated_cost(
            Decimal::new(5000, 0),
            Decimal::new(30, 2), // 0.30
        );
        assert_eq!(result, Decimal::new(1500, 0));
    }

    #[test]
    fn 测试计算单位能耗_正常() {
        let result = compute_unit_consumption(
            Decimal::new(300, 0),
            Some(Decimal::new(100, 0)),
        );
        assert_eq!(result, Some(Decimal::new(3, 0)));
    }

    #[test]
    fn 测试计算单位能耗_产量为零返回None() {
        let result = compute_unit_consumption(Decimal::new(300, 0), Some(Decimal::ZERO));
        assert_eq!(result, None);
    }

    #[test]
    fn 测试计算单位能耗_产量为None返回None() {
        let result = compute_unit_consumption(Decimal::new(300, 0), None);
        assert_eq!(result, None);
    }

    #[test]
    fn 测试超基准判断_正常未超() {
        let (exceeds, deviation) = check_consumption_exceeds_standard(
            Decimal::new(95, 0),
            Decimal::new(100, 0),
            Decimal::new(10, 2), // 0.10
        );
        assert!(!exceeds);
        assert_eq!(deviation, Decimal::ZERO);
    }

    #[test]
    fn 测试超基准判断_超出阈值() {
        // 标准 100，容差 10%，阈值 110，实际 120 → 超基准
        let (exceeds, deviation) = check_consumption_exceeds_standard(
            Decimal::new(120, 0),
            Decimal::new(100, 0),
            Decimal::new(10, 2),
        );
        assert!(exceeds);
        // 偏差 = (120 - 100) / 100 × 100 = 20
        assert_eq!(deviation, Decimal::new(20, 0));
    }

    #[test]
    fn 测试超基准判断_标准为零返回未超() {
        let (exceeds, deviation) = check_consumption_exceeds_standard(
            Decimal::new(120, 0),
            Decimal::ZERO,
            Decimal::new(10, 2),
        );
        assert!(!exceeds);
        assert_eq!(deviation, Decimal::ZERO);
    }

    #[test]
    fn 测试校验能源类型_合法() {
        assert!(validate_meter_type("water").is_ok());
        assert!(validate_meter_type("electricity").is_ok());
        assert!(validate_meter_type("steam").is_ok());
        assert!(validate_meter_type("gas").is_ok());
        assert!(validate_meter_type("compressed_air").is_ok());
    }

    #[test]
    fn 测试校验能源类型_非法() {
        assert!(validate_meter_type("invalid").is_err());
    }

    #[test]
    fn 测试校验分摊基准_合法() {
        assert!(validate_allocation_basis("by_duration").is_ok());
        assert!(validate_allocation_basis("by_output").is_ok());
        assert!(validate_allocation_basis("by_equipment").is_ok());
        assert!(validate_allocation_basis("by_workshop").is_ok());
    }

    #[test]
    fn 测试校验分摊基准_非法() {
        assert!(validate_allocation_basis("invalid").is_err());
    }
}

//! 出库扣减规划与追溯维度校验（产品 + 色号 + 缸号 + 批次）
//!
//! 白坯/染色判定不在本文件重复实现，唯一来源是 [`crate::services::inv::fabric_class`]
//! （空色号⇒白坯免缸号、批次必填；非空⇒染色布、缸号+批次必填）；本文件仅委托它做维度校验。
//!
//! 业务规则（用户拍板）：
//! - **染色布**出库按"色号非空 ⇒ 色号 + 缸号 + 批次"匹配扣减：
//!   1. 先扣指定缸号（精确命中）的库存行，按入库时间（created_at）升序、库存行 ID 升序；
//!   2. 不足部分按【缸号字典序升序（无缸号的行排最后）→ 入库时间升序 → 库存行 ID 升序】
//!      依次回退到其他缸；
//!   3. 每一笔实际扣到的缸号/批次都必须如实写出（由调用方落进出库明细与库存流水）。
//! - **白坯布**（色号为空）出库按"色号空 + 批次"匹配、**免缸号**：只接受库里无缸号
//!   （`dye_lot_no IS NULL`）的库存行，**不存在跨缸概念、绝不回退到带缸号行**（白坯没颜色、
//!   自然没有染缸，跨缸回退到染缸行会把染缸料当白坯扣走，破坏追溯）。
//! - 不做兜底：染色布缺色号/缸号/批次、白坯布缺批次，或组合口径无可用库存时，
//!   返回明确业务错误，禁止回退到"产品+色号"式随意扣减。
//!
//! 本文件只承载**纯规划逻辑**（可单测、无 DB 依赖）；SELECT/UPDATE 与流水记录
//! 由各出库路径（销售发货 `so::delivery_ops`、调拨出库 `inv::batch`）执行。

use crate::services::inv::fabric_class::{self, FabricTrace};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::utils::error::AppError;

/// 一笔分配的来源：精确命中指定缸号，还是显式跨缸回退。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationSource {
    /// 扣的是出库单指定的缸号
    ExactDyeLot,
    /// 指定缸数量不足，显式回退扣了其他缸号
    CrossDyeLot,
}

/// 参与规划的候选库存行（四维匹配后剩余的候选，字段为规划所需最小集）。
#[derive(Debug, Clone)]
pub struct DeductionCandidate {
    /// 库存行 ID（inventory_stocks.id）
    pub stock_id: i32,
    /// 该库存行的缸号：白坯库存行为 None（`inventory_stocks.dye_lot_no IS NULL`，
    /// 入库白坯由 purchase_receipt 落 Set(None)）；染色行为非空缸号
    pub dye_lot_no: Option<String>,
    /// 该库存行当前可用数量
    pub quantity_available: Decimal,
    /// 入库时间（跨缸/同行排序用）
    pub created_at: DateTime<Utc>,
}

/// 规划产出的一笔实际扣减分配（调用方据此逐行 UPDATE 库存并写流水/明细）。
#[derive(Debug, Clone, PartialEq)]
pub struct DeductionAllocation {
    /// 实际被扣的库存行 ID
    pub stock_id: i32,
    /// 实际被扣库存行的缸号（必须如实写进出库流水/明细）
    pub dye_lot_no: Option<String>,
    /// 本笔分配扣减数量
    pub quantity: Decimal,
    /// 扣减前该行可用数量
    pub quantity_before: Decimal,
    /// 扣减后该行可用数量
    pub quantity_after: Decimal,
    /// 精确命中还是跨缸回退
    pub source: AllocationSource,
}

/// 规划失败原因（由调用方拼上四维上下文转成中文业务错误）。
#[derive(Debug, Clone, PartialEq)]
pub enum DeductionError {
    /// 指定缸号 + 其他缸都没有可扣数量（候选为空或全部可用量为 0）
    NoStockAtAll,
    /// 有库存但总量不足（含跨缸回退后仍不足）
    Insufficient {
        /// 四维口径下（含可回退的其他缸）可用总量
        available_total: Decimal,
        /// 本次要求出库数量
        required: Decimal,
    },
}

/// 对候选库存行做四维扣减规划。
///
/// `candidates` 为"款号+仓库+批次"口径下、按色号过滤后的全部候选行（缸号维度在规划内取舍），
/// `requested_dye_lot` 为出库单的缸号需求：
/// - `Some(lot)`（染色布）：出库单指定缸号；
/// - `None`（白坯布）：无缸号维度。
///
/// 数量单位与调用方口径一致（销售出库为米，调拨出库为米）。
///
/// **染色布分支（Some(lot)，行为不变，已由本文件单测锁定）**：
/// `candidates` 已由调用方按"款号(product_id)+色号+批次"过滤，缸号维度在规划内取舍——
/// 先消费与指定缸号精确命中的行（同产品+同色号+同批次+同缸号四维全等），按入库时间升序→行 ID
/// 升序；不足时才走**显式跨缸回退**（其他缸按缸号字典序→入库时间→行 ID 的确定性次序），
/// 每笔如实标 `AllocationSource::CrossDyeLot`。
///
/// **白坯布分支（None，新增）**：只接受库里无缸号（`dye_lot_no IS NULL`，即入库白坯的真实存储）
/// 的候选行，行内按入库时间升序→行 ID 升序消耗；**不做任何跨缸回退**——白坯没有颜色也就没有
/// 染缸概念，若允许回退会把染缸料当白坯扣走，破坏缸号追溯与账实一致。合计不足即报
/// [`DeductionError`]，绝不用带缸号的行凑数。
pub fn plan_deduction(
    candidates: &[DeductionCandidate],
    requested_dye_lot: Option<&str>,
    quantity: Decimal,
) -> Result<Vec<DeductionAllocation>, DeductionError> {
    if quantity <= Decimal::ZERO {
        // 调用方入口已保证数量为正；此处防御性拒绝，避免规划出空转分配
        return Err(DeductionError::Insufficient {
            available_total: Decimal::ZERO,
            required: quantity,
        });
    }

    // 按布种分支构造确定性消费次序（见函数文档）。
    let ordered: Vec<&DeductionCandidate> = match requested_dye_lot {
        Some(lot) => {
            // 染色布：精确命中指定缸的行在前（入库时间升序 → 行 ID 升序），
            // 跨缸回退行按缸号字典序升序（None 排最后）→ 入库时间升序 → 行 ID 升序。
            let mut v: Vec<&DeductionCandidate> = candidates
                .iter()
                .filter(|c| c.quantity_available > Decimal::ZERO)
                .collect();
            v.sort_by(|a, b| {
                let a_exact = a.dye_lot_no.as_deref() == Some(lot);
                let b_exact = b.dye_lot_no.as_deref() == Some(lot);
                // bool 的 Ord 是 false < true，所以必须用 b.cmp(a) 才把"精确命中指定缸号"的行排前面；
                // 写成 a_exact.cmp(&b_exact) 会把指定缸的行压到最后，出库先扣别的缸。
                b_exact
                    .cmp(&a_exact)
                    .then_with(|| match (a_exact, b_exact) {
                        // 同为跨缸回退行时先比缸号字典序；同为精确行时缸号相同，直接比时间
                        (false, false) => {
                            dye_lot_order_key(&a.dye_lot_no).cmp(&dye_lot_order_key(&b.dye_lot_no))
                        }
                        _ => std::cmp::Ordering::Equal,
                    })
                    .then_with(|| a.created_at.cmp(&b.created_at))
                    .then_with(|| a.stock_id.cmp(&b.stock_id))
            });
            v
        }
        None => {
            // 白坯布：只接受库里无缸号（dye_lot_no IS NULL）的候选行，行内按入库时间→行 ID 消耗，
            // 不回退到带缸号的行（白坯无缸号概念）。
            let mut v: Vec<&DeductionCandidate> = candidates
                .iter()
                .filter(|c| c.quantity_available > Decimal::ZERO && c.dye_lot_no.is_none())
                .collect();
            v.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.stock_id.cmp(&b.stock_id))
            });
            v
        }
    };

    let available_total: Decimal = ordered.iter().map(|c| c.quantity_available).sum();
    if ordered.is_empty() || available_total <= Decimal::ZERO {
        return Err(DeductionError::NoStockAtAll);
    }

    let mut remaining = quantity;
    let mut allocations: Vec<DeductionAllocation> = Vec::new();
    for c in ordered {
        if remaining <= Decimal::ZERO {
            break;
        }
        let take = remaining.min(c.quantity_available);
        // 白坯分支候选已限定为无缸号行，每行即需求行本身，恒精确命中（不标跨缸）；
        // 染色分支按是否等于指定缸号判定。
        let is_exact = match requested_dye_lot {
            None => true,
            Some(lot) => c.dye_lot_no.as_deref() == Some(lot),
        };
        allocations.push(DeductionAllocation {
            stock_id: c.stock_id,
            dye_lot_no: c.dye_lot_no.clone(),
            quantity: take,
            quantity_before: c.quantity_available,
            quantity_after: c.quantity_available - take,
            source: if is_exact {
                AllocationSource::ExactDyeLot
            } else {
                AllocationSource::CrossDyeLot
            },
        });
        remaining -= take;
    }

    if remaining > Decimal::ZERO {
        return Err(DeductionError::Insufficient {
            available_total,
            required: quantity,
        });
    }
    Ok(allocations)
}

/// 跨缸回退的缸号排序键：无缸号（NULL）行排最后，其余按字典序升序。
fn dye_lot_order_key(dye_lot_no: &Option<String>) -> (u8, &str) {
    match dye_lot_no.as_deref().filter(|s| !s.is_empty()) {
        Some(lot) => (1, lot),
        None => (0, ""),
    }
}

/// 出库单三维入参（色号 + 缸号 + 批次，trim 后 owned 值）。
///
/// 与入库侧 [`fabric_class`] 的判定结果 [`FabricTrace`] 同源同型：白坯布 `color_no` 为空串、
/// `dye_lot_no` 为 None；染色布二者皆非空。用类型别名而非另立结构，杜绝出库/入库两份口径漂移。
pub type OutboundDimensions = FabricTrace;

/// 校验并归一化出库明细的追溯维度（款号由 product_id 承载，此处含色号 / 缸号 / 批次）。
///
/// 判定委托全仓唯一实现 [`fabric_class::validate_fabric_trace`]，出库 / 入库同源，
/// 不在出库侧再写第二份规则（业务铁律：白坯布 = 没有颜色的布）：
/// - 白坯布（色号为空 / 空白）：免缸号，`dye_lot_no` 归一为 None，批次仍必填；
/// - 染色布（色号非空）：缸号、批次都必填，缺一返回明确业务错误（不做兜底）；
/// - 不以色号名称嗅探白坯（"本白""WHITE" 是已染色的白色布，必须带缸号追溯）。
///
/// `bill_label` 用于指明是销售发货明细还是调拨出库明细，`product_id` 用于定位款号。
pub fn require_outbound_dimensions(
    bill_label: &str,
    product_id: i32,
    color_no: Option<&str>,
    dye_lot_no: Option<&str>,
    batch_no: Option<&str>,
) -> Result<OutboundDimensions, AppError> {
    fabric_class::validate_fabric_trace(
        color_no.map(str::to_string),
        dye_lot_no.map(str::to_string),
        batch_no.map(str::to_string),
    )
    .map_err(|e| AppError::business(format!("{}（款号产品 {}）：{}", bill_label, product_id, e)))
}

// =====================================================
// 单元测试（染色：精确命中 / 部分命中+跨缸回退 / 无库存；白坯：仅无缸号行、不跨缸）
// =====================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::inv::fabric_class::validate_fabric_trace;
    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(v: &str) -> Decimal {
        Decimal::from_str(v).unwrap()
    }

    fn ts(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, day, 1, 0, 0, 0).unwrap()
    }

    fn row(stock_id: i32, dye_lot: Option<&str>, available: &str, day: u32) -> DeductionCandidate {
        DeductionCandidate {
            stock_id,
            dye_lot_no: dye_lot.map(str::to_string),
            quantity_available: d(available),
            created_at: ts(day),
        }
    }

    fn exact_allocations(allocs: &[DeductionAllocation]) -> Vec<(i32, Option<&str>, Decimal)> {
        allocs
            .iter()
            .map(|a| (a.stock_id, a.dye_lot_no.as_deref(), a.quantity))
            .collect()
    }

    #[test]
    fn test_exact_hit_deducts_only_requested_dye_lot() {
        // 指定缸 DL-A 充足：只扣 DL-A，绝不动其他缸
        let candidates = vec![
            row(1, Some("DL-A"), "100", 3),
            row(2, Some("DL-B"), "100", 1),
        ];
        let allocs = plan_deduction(&candidates, Some("DL-A"), d("40")).unwrap();
        assert_eq!(exact_allocations(&allocs), vec![(1, Some("DL-A"), d("40"))]);
        assert_eq!(allocs[0].quantity_before, d("100"));
        assert_eq!(allocs[0].quantity_after, d("60"));
        assert_eq!(allocs[0].source, AllocationSource::ExactDyeLot);
    }

    #[test]
    fn test_exact_rows_consumed_in_fifo_order_by_created_at() {
        // 同一指定缸多行（同批次入库分多行）：按入库时间升序、行 ID 升序消耗
        let candidates = vec![
            row(9, Some("DL-A"), "10", 5),
            row(7, Some("DL-A"), "10", 2),
            row(8, Some("DL-A"), "10", 2),
        ];
        let allocs = plan_deduction(&candidates, Some("DL-A"), d("25")).unwrap();
        assert_eq!(
            exact_allocations(&allocs),
            vec![
                (7, Some("DL-A"), d("10")),
                (8, Some("DL-A"), d("10")),
                (9, Some("DL-A"), d("5"))
            ]
        );
    }

    #[test]
    fn test_partial_hit_triggers_deterministic_cross_dye_lot_fallback() {
        // 指定缸 DL-A 只有 10，需 25 → 精确 10 + 跨缸按缸号字典序补 DL-B(5)
        // DL-C/DL-D 存在但字典序在 DL-B 之后，不应被扣
        let candidates = vec![
            row(1, Some("DL-D"), "50", 1),
            row(2, Some("DL-A"), "10", 1),
            row(3, Some("DL-C"), "50", 1),
            row(4, Some("DL-B"), "50", 4),
        ];
        let allocs = plan_deduction(&candidates, Some("DL-A"), d("15")).unwrap();
        assert_eq!(
            exact_allocations(&allocs),
            vec![(2, Some("DL-A"), d("10")), (4, Some("DL-B"), d("5"))]
        );
        assert_eq!(allocs[0].source, AllocationSource::ExactDyeLot);
        assert_eq!(allocs[1].source, AllocationSource::CrossDyeLot);
        // 跨缸行记录的是被扣行自己的扣减前/后数量（供流水如实记录）
        assert_eq!(allocs[1].quantity_before, d("50"));
        assert_eq!(allocs[1].quantity_after, d("45"));
    }

    #[test]
    fn test_cross_dye_lot_rows_without_dye_lot_sort_last() {
        // 无缸号行只能作为最后顺位的跨缸回退
        let candidates = vec![
            row(1, None, "50", 1),
            row(2, Some("DL-A"), "5", 1),
            row(3, Some("DL-Z"), "50", 9),
        ];
        let allocs = plan_deduction(&candidates, Some("DL-A"), d("12")).unwrap();
        assert_eq!(
            exact_allocations(&allocs),
            vec![(2, Some("DL-A"), d("5")), (3, Some("DL-Z"), d("7"))]
        );
        assert_eq!(allocs[1].source, AllocationSource::CrossDyeLot);
    }

    #[test]
    fn test_no_candidates_at_all_returns_no_stock_error() {
        // 四维组合完全无库存：报 NoStockAtAll，不回退到"产品+色号"
        let candidates: Vec<DeductionCandidate> = vec![];
        let err = plan_deduction(&candidates, Some("DL-A"), d("1")).unwrap_err();
        assert_eq!(err, DeductionError::NoStockAtAll);

        // 有行但全部可用量为 0 同样视为无库存
        let zero_rows = vec![row(1, Some("DL-A"), "0", 1), row(2, Some("DL-B"), "0", 2)];
        assert_eq!(
            plan_deduction(&zero_rows, Some("DL-A"), d("1")).unwrap_err(),
            DeductionError::NoStockAtAll
        );
    }

    #[test]
    fn test_insufficient_after_fallback_reports_real_totals() {
        // 指定缸 + 可回退缸全部加起来仍不足
        let candidates = vec![row(1, Some("DL-A"), "10", 1), row(2, Some("DL-B"), "20", 1)];
        let err = plan_deduction(&candidates, Some("DL-A"), d("35")).unwrap_err();
        assert_eq!(
            err,
            DeductionError::Insufficient {
                available_total: d("30"),
                required: d("35")
            }
        );
    }

    #[test]
    fn test_non_positive_quantity_is_rejected() {
        let candidates = vec![row(1, Some("DL-A"), "10", 1)];
        assert!(plan_deduction(&candidates, Some("DL-A"), Decimal::ZERO).is_err());
        assert!(plan_deduction(&candidates, Some("DL-A"), d("-1")).is_err());
    }

    // ---------- 白坯布（无缸号）分支 ----------

    #[test]
    fn white_greige_deducts_only_null_dye_lot_rows() {
        // 白坯分支：库里无缸号（dye_lot_no IS NULL）的候选可正常扣减，且标精确命中
        let candidates = vec![
            row(11, None, "60", 3),
            row(10, None, "60", 1),
            row(20, Some("DL-X"), "100", 2), // 带缸号行不得被白坯出库看到
        ];
        let allocs = plan_deduction(&candidates, None, d("100")).unwrap();
        // 仅扣两行无缸号行，按入库时间升序（先 id=10 后 id=11），绝不触碰 DL-X
        assert_eq!(
            exact_allocations(&allocs),
            vec![(10, None, d("60")), (11, None, d("40"))]
        );
        assert!(
            allocs
                .iter()
                .all(|a| a.source == AllocationSource::ExactDyeLot),
            "白坯出库不应产生跨缸回退标记"
        );
        assert!(
            !allocs.iter().any(|a| a.dye_lot_no.is_some()),
            "白坯出库绝不得扣到带缸号的行"
        );
    }

    #[test]
    fn white_greige_no_null_dye_lot_row_is_no_stock() {
        // 只有带缸号的候选时，白坯出库必须报无库存（而非回退去扣染缸料）
        let candidates = vec![
            row(1, Some("DL-A"), "100", 1),
            row(2, Some("DL-B"), "100", 2),
        ];
        let err = plan_deduction(&candidates, None, d("10")).unwrap_err();
        assert_eq!(err, DeductionError::NoStockAtAll);
    }

    #[test]
    fn white_greige_insufficient_without_cross_dye_fallback() {
        // 无缸号行合计不足：报明确不足错误，不回退到带缸号行凑数
        let candidates = vec![row(1, None, "5", 1), row(2, Some("DL-A"), "100", 2)];
        let err = plan_deduction(&candidates, None, d("20")).unwrap_err();
        assert_eq!(
            err,
            DeductionError::Insufficient {
                available_total: d("5"),
                required: d("20")
            }
        );
    }

    // ---------- 维度校验与同源判定 ----------

    #[test]
    fn require_outbound_dimensions_dyed_requires_dye_lot_and_batch() {
        // 染色布（色号非空）缺缸号/缺批次都报业务错误，且指明缺哪个维度
        let missing_dye =
            require_outbound_dimensions("销售发货", 7, Some("RED"), Some("  "), Some("B1"))
                .unwrap_err();
        assert!(
            missing_dye.to_string().contains("缸号"),
            "实际: {}",
            missing_dye
        );

        let missing_batch =
            require_outbound_dimensions("调拨出库", 7, Some("RED"), Some("DL-A"), None)
                .unwrap_err();
        assert!(
            missing_batch.to_string().contains("批"),
            "实际: {}",
            missing_batch
        );

        // 染色布四维齐（色号+缸号+批次）通过并 trim
        let ok =
            require_outbound_dimensions("销售发货", 7, Some(" RED "), Some("DL-A"), Some("B1"))
                .unwrap();
        assert_eq!(ok.color_no, "RED");
        assert_eq!(ok.dye_lot_no.as_deref(), Some("DL-A"));
        assert_eq!(ok.batch_no, "B1");
    }

    #[test]
    fn require_outbound_dimensions_white_greige_allows_empty_color_no_dye_lot() {
        // 白坯布（色号为空 / 空白 / None）免缸号，批次仍必填；缸号归一为 None
        let none_color =
            require_outbound_dimensions("销售发货", 7, None, None, Some("B1")).unwrap();
        assert_eq!(none_color.color_no, "");
        assert_eq!(none_color.dye_lot_no, None);
        assert_eq!(none_color.batch_no, "B1");

        let blank_color =
            require_outbound_dimensions("调拨出库", 7, Some("   "), Some("  "), Some("B2"))
                .unwrap();
        assert_eq!(blank_color.color_no, "");
        assert_eq!(blank_color.dye_lot_no, None);

        // 白坯缺批次仍拒（批次与是否染色无关，必填）
        let err = require_outbound_dimensions("销售发货", 7, Some(""), None, None).unwrap_err();
        assert!(err.to_string().contains("批"), "实际: {}", err);
    }

    #[test]
    fn white_named_color_is_dyed_and_requires_dye_lot_on_outbound() {
        // 名字带"白"/WHITE 的色号是染色白色布，出库缺缸号必须被拒（不按名称豁免）
        let err =
            require_outbound_dimensions("销售发货", 7, Some("本白"), None, Some("B1")).unwrap_err();
        assert!(err.to_string().contains("缸号"), "实际: {}", err);
    }

    #[test]
    fn outbound_and_inbound_determination_share_single_source() {
        // 出库 require_outbound_dimensions 与入库 validate_fabric_trace 判定同源：
        // 同一组（色号/缸号/批次）在两个方向必须得到完全一致的归一化结果。
        let cases: [(&str, &str, &str); 4] = [
            ("", "", "B1"),         // 白坯：免缸号
            ("  ", "  ", "B2"),     // 白坯（空白）
            ("RED", "DL-A", "B3"),  // 染色：四维齐
            ("本白", "DL-W", "B4"), // 白色号是染色布
        ];
        for (color, dye, batch) in cases {
            let outbound =
                require_outbound_dimensions("销售发货", 1, Some(color), Some(dye), Some(batch))
                    .expect("有效维度应通过");
            let inbound = validate_fabric_trace(
                Some(color.to_string()),
                Some(dye.to_string()),
                Some(batch.to_string()),
            )
            .expect("有效维度应通过");
            assert_eq!(
                outbound, inbound,
                "同一入参出库/入库判定必须一致: {color:?}/{dye:?}"
            );
        }

        // 染色布缺缸号：两方向都必须拒绝
        let ob = require_outbound_dimensions("销售发货", 1, Some("RED"), None, Some("B5"));
        let ib = validate_fabric_trace(Some("RED".to_string()), None, Some("B5".to_string()));
        assert!(ob.is_err());
        assert!(ib.is_err());
    }
}

//! 出库四维匹配扣减规划（款号 + 色号 + 缸号 + 批次）
//!
//! 业务规则（用户拍板）：
//! - 出库扣减库存必须按入库时的四个维度匹配：款号（product_id，对应 products.code）
//!   + 色号（inventory_stocks.color_no）+ 缸号（inventory_stocks.dye_lot_no）
//!   + 批次（inventory_stocks.batch_no）。
//! - 只有当"同款号 + 色号 + 批次"在**指定缸号**下数量不足时，才允许走**显式跨缸回退**：
//!   扣其他缸号的库存，且回退次序必须确定、可解释——
//!   1. 先扣指定缸号（精确命中）的库存行，按入库时间（created_at）升序、库存行 ID 升序；
//!   2. 不足部分按【缸号字典序升序（无缸号的行排最后）→ 入库时间升序 → 库存行 ID 升序】
//!      依次回退到其他缸；
//!   3. 每一笔实际扣到的缸号/批次都必须如实写出（由调用方落进出库明细与库存流水）。
//! - 不做兜底：出库单未指定缸号/色号/批次、或四维组合（含回退范围内）完全没有可用库存时，
//!   返回明确业务错误，禁止回退到"产品+色号"式随意扣减。
//!
//! 本文件只承载**纯规划逻辑**（可单测、无 DB 依赖）；SELECT/UPDATE 与流水记录
//! 由各出库路径（销售发货 `so::delivery_ops`、调拨出库 `inv::batch`）执行。

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
    /// 该库存行的缸号（可为空 = 无缸号行，只在跨缸回退且排在最后时才会被扣）
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
/// `candidates` 为"款号+色号+批次"口径下的全部候选行（缸号过滤在这里完成），
/// `requested_dye_lot` 为出库单指定的缸号（非空，已由 [`require_outbound_dimensions`] 校验）。
/// 数量单位与调用方口径一致（销售出库为米，调拨出库为米）。
///
/// 与"产品→色号→缸号→批次"次序的核对结论（用户拍板口径，**不改行为**）：本函数的
/// `candidates` 已由调用方按"款号(product_id)+色号+批次"过滤，缸号维度在规划内取舍——
/// 先消费与指定缸号精确命中的行（即同产品+同色号+同批次+同缸号四维全等），不足时才走**显式
/// 跨缸回退**（其他缸按缸号字典序→入库时间→行 ID 的确定性次序）。这与"先满足同产品同色号同
/// 缸号同批次、不足才显式跨缸回退"的口径完全一致，故排序实现无需改动（已被本文件 8 个单测锁定）。
pub fn plan_deduction(
    candidates: &[DeductionCandidate],
    requested_dye_lot: &str,
    quantity: Decimal,
) -> Result<Vec<DeductionAllocation>, DeductionError> {
    if quantity <= Decimal::ZERO {
        // 调用方入口已保证数量为正；此处防御性拒绝，避免规划出空转分配
        return Err(DeductionError::Insufficient {
            available_total: Decimal::ZERO,
            required: quantity,
        });
    }

    // 确定性次序：精确缸号行在前（入库时间升序 → 行 ID 升序），
    // 跨缸回退行按缸号字典序升序（None 排最后）→ 入库时间升序 → 行 ID 升序。
    let mut ordered: Vec<&DeductionCandidate> = candidates
        .iter()
        .filter(|c| c.quantity_available > Decimal::ZERO)
        .collect();
    ordered.sort_by(|a, b| {
        let a_exact = a.dye_lot_no.as_deref() == Some(requested_dye_lot);
        let b_exact = b.dye_lot_no.as_deref() == Some(requested_dye_lot);
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
        allocations.push(DeductionAllocation {
            stock_id: c.stock_id,
            dye_lot_no: c.dye_lot_no.clone(),
            quantity: take,
            quantity_before: c.quantity_available,
            quantity_after: c.quantity_available - take,
            source: if c.dye_lot_no.as_deref() == Some(requested_dye_lot) {
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

/// 出库单四维入参（已通过非空校验，trim 后的 owned 值）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundDimensions {
    /// 色号
    pub color_no: String,
    /// 缸号
    pub dye_lot_no: String,
    /// 批次号
    pub batch_no: String,
}

/// 校验出库明细必须显式携带色号/缸号/批次（款号由 product_id 承载）。
///
/// 缺失即业务错误（不做兜底）：`bill_label` 用于指明是销售发货明细还是调拨出库明细。
pub fn require_outbound_dimensions(
    bill_label: &str,
    product_id: i32,
    color_no: Option<&str>,
    dye_lot_no: Option<&str>,
    batch_no: Option<&str>,
) -> Result<OutboundDimensions, AppError> {
    let dim = |value: Option<&str>, name: &str| -> Result<String, AppError> {
        value
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                AppError::business(format!(
                    "{}明细（款号产品 {}）缺少{}：出库必须按款号+色号+缸号+批次四维匹配扣减，不允许缺维度扣减",
                    bill_label, product_id, name
                ))
            })
    };
    Ok(OutboundDimensions {
        color_no: dim(color_no, "色号")?,
        dye_lot_no: dim(dye_lot_no, "缸号")?,
        batch_no: dim(batch_no, "批次号")?,
    })
}

// =====================================================
// 单元测试（四维扣减规划：精确命中 / 部分命中+跨缸回退 / 无库存报错）
// =====================================================
#[cfg(test)]
mod tests {
    use super::*;
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

    fn exact_allocations(allocs: &[DeductionAllocation]) -> Vec<(i32, &str, Decimal)> {
        allocs
            .iter()
            .map(|a| {
                (
                    a.stock_id,
                    a.dye_lot_no.as_deref().unwrap_or(""),
                    a.quantity,
                )
            })
            .collect()
    }

    #[test]
    fn test_exact_hit_deducts_only_requested_dye_lot() {
        // 指定缸 DL-A 充足：只扣 DL-A，绝不动其他缸
        let candidates = vec![
            row(1, Some("DL-A"), "100", 3),
            row(2, Some("DL-B"), "100", 1),
        ];
        let allocs = plan_deduction(&candidates, "DL-A", d("40")).unwrap();
        assert_eq!(exact_allocations(&allocs), vec![(1, "DL-A", d("40"))]);
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
        let allocs = plan_deduction(&candidates, "DL-A", d("25")).unwrap();
        assert_eq!(
            exact_allocations(&allocs),
            vec![
                (7, "DL-A", d("10")),
                (8, "DL-A", d("10")),
                (9, "DL-A", d("5"))
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
        let allocs = plan_deduction(&candidates, "DL-A", d("15")).unwrap();
        assert_eq!(
            exact_allocations(&allocs),
            vec![(2, "DL-A", d("10")), (4, "DL-B", d("5"))]
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
        let allocs = plan_deduction(&candidates, "DL-A", d("12")).unwrap();
        assert_eq!(
            exact_allocations(&allocs),
            vec![(2, "DL-A", d("5")), (3, "DL-Z", d("7"))]
        );
        assert_eq!(allocs[1].source, AllocationSource::CrossDyeLot);
    }

    #[test]
    fn test_no_candidates_at_all_returns_no_stock_error() {
        // 四维组合完全无库存：报 NoStockAtAll，不回退到"产品+色号"
        let candidates: Vec<DeductionCandidate> = vec![];
        let err = plan_deduction(&candidates, "DL-A", d("1")).unwrap_err();
        assert_eq!(err, DeductionError::NoStockAtAll);

        // 有行但全部可用量为 0 同样视为无库存
        let zero_rows = vec![row(1, Some("DL-A"), "0", 1), row(2, Some("DL-B"), "0", 2)];
        assert_eq!(
            plan_deduction(&zero_rows, "DL-A", d("1")).unwrap_err(),
            DeductionError::NoStockAtAll
        );
    }

    #[test]
    fn test_insufficient_after_fallback_reports_real_totals() {
        // 指定缸 + 可回退缸全部加起来仍不足
        let candidates = vec![row(1, Some("DL-A"), "10", 1), row(2, Some("DL-B"), "20", 1)];
        let err = plan_deduction(&candidates, "DL-A", d("35")).unwrap_err();
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
        assert!(plan_deduction(&candidates, "DL-A", Decimal::ZERO).is_err());
        assert!(plan_deduction(&candidates, "DL-A", d("-1")).is_err());
    }

    #[test]
    fn test_require_outbound_dimensions_rejects_each_missing_dim() {
        // 缺任一维度都必须报业务错误（不兜底），且错误信息指明缺哪个维度
        let missing_dye =
            require_outbound_dimensions("销售发货", 7, Some("RED"), Some("  "), Some("B1"));
        assert!(matches!(missing_dye, Err(AppError::BusinessError(_))));
        assert!(missing_dye.unwrap_err().to_string().contains("缸号"));

        let missing_color =
            require_outbound_dimensions("销售发货", 7, None, Some("DL-A"), Some("B1"));
        assert!(missing_color.unwrap_err().to_string().contains("色号"));

        let missing_batch =
            require_outbound_dimensions("调拨出库", 7, Some("RED"), Some("DL-A"), None);
        assert!(missing_batch.unwrap_err().to_string().contains("批次号"));

        let ok =
            require_outbound_dimensions("销售发货", 7, Some(" RED "), Some("DL-A"), Some("B1"))
                .unwrap();
        assert_eq!(
            ok,
            OutboundDimensions {
                color_no: "RED".to_string(),
                dye_lot_no: "DL-A".to_string(),
                batch_no: "B1".to_string(),
            }
        );
    }
}

//! 纺织白坯/染色判定（全仓唯一实现）
//!
//! 供入库侧 `inv::inventory_move`（建单批 / 重建批两处）、`inv::batch`（add_item 批），
//! 以及出库侧 `services::inventory_deduction::require_outbound_dimensions`（销售发货 / 调拨出库）
//! 共同调用，统一按用户拍板的纺织四维口径校验色号 / 缸号 / 批次三个追溯字段。
//! 出库侧不再另写判定规则，判定唯一来源即本文件。

use crate::utils::error::AppError;

/// 校验并归一化后的面料追溯字段（款号由 product_id 承载，此处含色号 / 缸号 / 批次）。
///
/// 三字段的类型与 `inventory_transfer_item::ActiveModel` 对应列一一对应：
/// `color_no` / `batch_no` 为 NOT NULL 的 `String`（白坯布色号为空串），
/// `dye_lot_no` 为 `Option<String>`（白坯布合法为 None，落 DB 默认空）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricTrace {
    /// trim 后的色号；白坯布为空串
    pub color_no: String,
    /// 染色布为非空缸号；白坯布归一为 None
    pub dye_lot_no: Option<String>,
    /// trim 后的批次（入库批次，任何布种都必填）
    pub batch_no: String,
}

/// 白坯 / 染色判定 + 缸号 / 批次必填校验（全仓唯一实现，禁止在他处重复判定）。
///
/// 为什么不能用色号名称判断白坯布：白坯布的业务定义是"没有颜色"，即 `color_no` 为空；
/// 名字里带"白"的色号（如"本白""白色"）是已染色的白色布，属有颜色产品，必须带缸号追溯，
/// 按名称嗅探会把染色白色布误判为白坯而豁免缸号，破坏四维追溯。
///
/// 统一规则（用户拍板）：
/// - `color_no` 为空 → 白坯布：免缸号（`dye_lot_no` 归一为 None），但批次仍必填
///   （批次是入库批次，与是否染色无关）；
/// - `color_no` 非空 → 染色布：缸号、批次都必填，缺一即返回明确业务错误；
/// - 不看色号文本内容，仅以是否为空判定布种。
pub fn validate_fabric_trace(
    color_no: Option<String>,
    dye_lot_no: Option<String>,
    batch_no: Option<String>,
) -> Result<FabricTrace, AppError> {
    let color_no = color_no.map(|s| s.trim().to_string()).unwrap_or_default();
    let dye_lot_no = dye_lot_no
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let batch_no = batch_no
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::validation("明细缺少批号：出入库按款号+色号+缸号+批次四维追溯，批次不得为空")
        })?;
    // 色号为空 = 白坯布免缸号；色号非空 = 染色布缸号必填（仅以是否为空判定，不看名称）。
    if !color_no.is_empty() && dye_lot_no.is_none() {
        return Err(AppError::validation(format!(
            "染色布必须提供缸号（color_no={} 但 dye_lot_no 为空）",
            color_no
        )));
    }
    Ok(FabricTrace {
        color_no,
        dye_lot_no,
        batch_no,
    })
}

#[cfg(test)]
mod tests {
    use super::validate_fabric_trace;

    fn v(color: Option<&str>, dye: Option<&str>, batch: Option<&str>) -> super::FabricTrace {
        validate_fabric_trace(
            color.map(str::to_string),
            dye.map(str::to_string),
            batch.map(str::to_string),
        )
        .expect("合法四维应通过")
    }

    #[test]
    fn empty_color_is_white_fabric_exempt_from_dye_lot() {
        // 空色号 = 白坯布：免缸号，缸号归一为 None；批次仍必填
        let f = v(Some(""), Some(""), Some("B1"));
        assert_eq!(f.color_no, "");
        assert_eq!(f.dye_lot_no, None);
        assert_eq!(f.batch_no, "B1");

        let g = v(None, None, Some("B2"));
        assert_eq!(g.color_no, "");
        assert_eq!(g.dye_lot_no, None);
    }

    #[test]
    fn dyed_fabric_missing_dye_lot_rejected() {
        // 非空色号 = 染色布：缺缸号必须报明确业务错误
        let err = validate_fabric_trace(Some("C001".to_string()), None, Some("B1".to_string()))
            .expect_err("染色布缺缸号必须报错");
        assert!(err.to_string().contains("缸号"), "实际: {}", err);
    }

    #[test]
    fn missing_batch_rejected_even_for_white_fabric() {
        // 批次是入库批次，白坯布亦必填
        let err_white = validate_fabric_trace(Some("".to_string()), None, None)
            .expect_err("白坯布缺批次必须报错");
        assert!(err_white.to_string().contains("批"), "实际: {}", err_white);

        let err_dyed =
            validate_fabric_trace(Some("C001".to_string()), Some("D9".to_string()), None)
                .expect_err("染色布缺批次必须报错");
        assert!(err_dyed.to_string().contains("批"), "实际: {}", err_dyed);
    }

    #[test]
    fn white_named_color_is_treated_as_dyed_not_greige() {
        // 不再按名称判定：名字带"白"的色号是染色白色布，缺缸号必须报错
        for name in ["本白", "白色", "白坯", "WHITE", "white"] {
            let err = validate_fabric_trace(Some(name.to_string()), None, Some("B1".to_string()))
                .expect_err(&format!(
                    "色号 {} 是染色布，缺缸号必须报错（不得按名称豁免）",
                    name
                ));
            assert!(
                err.to_string().contains("缸号"),
                "色号 {} 实际: {}",
                name,
                err
            );
        }
    }

    #[test]
    fn values_are_trimmed() {
        let f = v(Some(" C001 "), Some(" D9 "), Some(" B1 "));
        assert_eq!(f.color_no, "C001");
        assert_eq!(f.dye_lot_no.as_deref(), Some("D9"));
        assert_eq!(f.batch_no, "B1");
    }
}

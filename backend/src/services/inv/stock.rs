//! 库存服务 - 库存检查辅助（inv/stock）
//!
//! 提供调拨单创建时的源仓库库存预检逻辑。
//! 由 `move::create_transfer` 调用以确保调出仓库有充足库存。
//!
//! 拆分自原 `inventory_transfer_service.rs` 的 `check_from_warehouse_inventory` 私有方法。

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::inventory_stock::{self, Column as StockColumn, Entity as InventoryStockEntity};
use crate::models::status::purchase_inventory::{
    inventory_stock_quality_status as quality_status, inventory_stock_status,
};
use crate::services::inventory_deduction::{OutboundDimensions, require_outbound_dimensions};
use crate::utils::error::AppError;

use super::{InventoryTransferItemRequest, InventoryTransferService};

/// 库存行的可出库性判定（全仓唯一实现）。
///
/// 功能：把「这行库存现在能不能出库」收成一条判定——台账状态必须是「正常」（报废与软删除行
/// 不是可出库的在库库存），质量状态必须是「合格」（待检与不合格行未通过质检，出了就是错发）。
/// 调用方：本文件 [`match_single_item_against_stocks`]（调拨建单预检）、
/// `inv::batch::apply_ship_item_deduction`（调拨发运实扣），以及出库候选行的查询侧
/// （`handlers::inventory_stock_handler::validate_quality_status_param` 与
/// `StockListFilter` 按同一词表下推筛选）。
/// 入参：`stock`，读出的库存行。
/// 传给谁：返回 false 的行不进入可出量合计，调用方据此给出点名拒绝。
/// 存什么·存哪里：只读判定，不落数据；两列取值域见
/// `models::status::purchase_inventory::{inventory_stock_status, inventory_stock_quality_status}`。
///
/// 此前调拨建单与发运实扣都只按四维与数量筛候选行，报废行与不合格行照样能被扣减，
/// 而词表注释宣称"可出库筛选按 PASS 过滤"却没有对应实现，属假文档加真放行。
pub fn is_outbound_eligible(stock: &inventory_stock::Model) -> bool {
    stock.stock_status == inventory_stock_status::NORMAL
        && stock.quality_status == quality_status::PASS
}

/// 库存行不可出库的点名拒绝文案（列出每条被挡住行的状态，供建单与发运两处共用）。
///
/// 入参：`blocked` 是四维命中但可出库性不通过的行；`label` 是链路说法（调拨出库/调拨发运）。
/// 传给谁：调用方原样透传，`business_displayable`（400，可外显）。
/// 存什么·存哪里：只产文案，不落数据。
///
/// 与"无匹配库存"分开报，是因为两种情形的处置动作完全不同：前者是换批次或补捡验，
/// 后者是根本没这批货。合成一条"库存不足"会把质检未过的货说成没货，用户只会去补单。
pub fn non_outbound_rejection(
    label: &str,
    product_id: i32,
    dims: &OutboundDimensions,
    blocked: &[&inventory_stock::Model],
) -> AppError {
    let detail = blocked
        .iter()
        .take(5)
        .map(|s| {
            format!(
                "库存 {}（台账状态 {}，质量状态 {}，可用 {} 米）",
                s.id, s.stock_status, s.quality_status, s.quantity_available
            )
        })
        .collect::<Vec<_>>()
        .join("；");
    AppError::business_displayable(format!(
        "{label}被拒，产品 {}（色号 {}，批次 {}）命中的库存行不可出库：{}；可出库口径为台账状态「{}」且质量状态「{}」，报废与不合格行请走报废处置或重新质检",
        product_id,
        if dims.color_no.is_empty() {
            "白坯（无颜色）"
        } else {
            dims.color_no.as_str()
        },
        dims.batch_no,
        detail,
        inventory_stock_status::NORMAL,
        quality_status::PASS,
    ))
}

impl InventoryTransferService {
    /// 检查调出仓库库存是否充足（在调拨单创建事务中调用，确保所有调拨明细在源仓库有足够库存。；采用批量查询优化 N+1：先一次性查出所有相关 product 的库存记录，；再用内存匹配按追溯维度定位命中行。）
    ///
    /// 维度口径与出库真实扣减（`inv::batch::apply_ship_item_deduction`）对齐，判定唯一来源为
    /// 全仓实现 [`fabric_class`]（经 [`require_outbound_dimensions`] 包装）：
    /// 染色布（色号非空）缺缸号/匹号在此即返回明确校验错误（VALIDATION 族，4xx，禁裸 500）；白坯（色号为空）维持宽松、
    /// 仅按款号 + 批次匹配、不强制缸号/匹号，保持既有放行行为避免回归。
    /// 匹号在此只做存在性预检（只读、不占用，`piece_domain_service::validate_dyed_piece_for_outbound`）：
    /// 指定匹必须真实存在于调出仓该缸该批且为可用态，"假匹号"在建单期即拒；
    /// 真实消耗（CAS AVAILABLE→SHIPPED）发生在发运事务内，消除 TOCTOU。
    pub(crate) async fn check_from_warehouse_inventory(
        &self,
        from_warehouse_id: &i32,
        items: &[InventoryTransferItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 批量获取库存记录（优化N+1查询）
        // 跳过 product_id 为 None 的项，避免脏 product_id=0 污染查询
        let product_ids: Vec<i32> = items.iter().filter_map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(StockColumn::WarehouseId.eq(*from_warehouse_id))
            .filter(StockColumn::ProductId.is_in(product_ids))
            .all(txn)
            .await?;

        for item in items {
            // 上游已校验 product_id 必填；None 项直接跳过
            let Some(product_id) = item.product_id else {
                continue;
            };
            // 调拨量必填且严格大于零：此前缺键走 unwrap_or(ZERO) 静默兜底，
            // 于是零量明细既能建出来、又在"可用量 >= 0"的比较里恒通过，
            // 出库单出现无痕空行；负数则等价于凭空加库存并绕过出库门控。
            let quantity = item.quantity.ok_or_else(|| {
                AppError::validation_displayable(format!(
                    "调拨出库明细缺少数量（产品 {product_id}，批次 {:?}），数量必填且必须大于 0，拒绝建单",
                    item.batch_no
                ))
            })?;
            if quantity <= Decimal::ZERO {
                return Err(AppError::validation_displayable(format!(
                    "调拨出库明细数量必须大于 0（产品 {product_id}，实为 {quantity}），零与负数不构成出库，拒绝建单"
                )));
            }

            // 复用 fabric_class 唯一判定（经出库侧包装为带单据措辞的业务错误）：
            // 白坯布归一为空色号 + 无缸号；染色布缺缸号/匹号在此返回与 fabric_class 同口径的明确 4xx 错误。
            let dims = require_outbound_dimensions(
                "调拨出库",
                product_id,
                item.color_no.as_deref(),
                item.dye_lot_no.as_deref(),
                item.batch_no.as_deref(),
                item.piece_no.as_deref(),
            )?;

            // 染色布匹号存在性预检（与扣减同 tuple 口径；白坯 dims.piece_no 为 None 跳过）
            if let Some(piece_no) = dims.piece_no.as_deref() {
                let ctx = crate::services::piece_domain_service::OutboundPieceContext {
                    product_id,
                    warehouse_id: *from_warehouse_id,
                    dye_lot_no: dims.dye_lot_no.as_deref().unwrap_or_default(),
                    batch_no: &dims.batch_no,
                    piece_no,
                };
                crate::services::piece_domain_service::validate_dyed_piece_for_outbound(txn, &ctx)
                    .await?;
            }

            // 按归一维度在调出仓库存行中精确匹配并校验数量
            match_single_item_against_stocks(product_id, &dims, quantity, &stocks)?;
        }
        Ok(())
    }
}

/// 按追溯维度匹配调出仓库存并校验数量充足性（纯内存、无 DB，便于单测直接覆盖业务逻辑）。
///
/// 维度口径与出库真实扣减（`inv::batch::apply_ship_item_deduction`）对齐：
/// - 白坯布（`dims.color_no` 为空）：按款号 + 批次匹配，**不强制缸号**（维持既有宽松放行）；
/// - 染色布（`dims.color_no` 非空）：款号 + 色号 + 批次 + 缸号四维齐全才命中并放行。
///
/// 布种判定的唯一来源是 `dims`（由 `fabric_class::validate_fabric_trace` 归一），
/// 此处不再另写一套白坯 / 染色嗅探（尤其不得按色号名称含“白”猜白坯）。
fn match_single_item_against_stocks(
    product_id: i32,
    dims: &OutboundDimensions,
    quantity: Decimal,
    stocks: &[inventory_stock::Model],
) -> Result<(), AppError> {
    let is_dyed = !dims.color_no.is_empty();
    let matched: Vec<&inventory_stock::Model> = stocks
        .iter()
        .filter(|s| {
            s.product_id == product_id
                && s.color_no == dims.color_no
                && s.batch_no == dims.batch_no
                // 白坯不强制缸号（is_dyed=false 时短路放行任意缸行）；染色布要求缸号四维齐全命中
                && (!is_dyed
                    || s.dye_lot_no.as_deref().map(str::trim) == dims.dye_lot_no.as_deref())
        })
        .collect();

    if matched.is_empty() {
        let color_disp = if dims.color_no.is_empty() {
            "白坯（无颜色）".to_string()
        } else {
            dims.color_no.clone()
        };
        let dye_disp = match dims.dye_lot_no.as_deref() {
            Some(lot) => format!("，缸号：{}", lot),
            None => String::new(),
        };
        return Err(AppError::business(format!(
            "调出仓库无匹配库存记录，产品 {}（色号：{}{}，批次：{}）",
            product_id, color_disp, dye_disp, dims.batch_no
        )));
    }

    // 可出库门：四维命中不等于能出。报废行与待检/不合格行在此挡下并点名，
    // 不计入可出量合计（见 is_outbound_eligible）。
    let eligible: Vec<&inventory_stock::Model> = matched
        .iter()
        .copied()
        .filter(|s| is_outbound_eligible(s))
        .collect();
    if eligible.is_empty() {
        return Err(non_outbound_rejection(
            "调拨出库",
            product_id,
            dims,
            &matched,
        ));
    }

    let available: Decimal = eligible.iter().map(|s| s.quantity_available).sum();
    if available < quantity {
        return Err(AppError::business(format!(
            "调出仓库库存不足，产品 {}，当前可出库：{}，需要调拨：{}",
            product_id, available, quantity
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::inv::InventoryTransferItemRequest;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn item(
        product_id: i32,
        quantity: &str,
        color: Option<&str>,
        dye: Option<&str>,
        batch: Option<&str>,
    ) -> InventoryTransferItemRequest {
        InventoryTransferItemRequest {
            product_id: Some(product_id),
            quantity: Some(d(quantity)),
            notes: None,
            color_no: color.map(str::to_string),
            dye_lot_no: dye.map(str::to_string),
            batch_no: batch.map(str::to_string),
            // 测试默认补一个匹号：染色布出库四维（缸/色/批/匹）中匹号必填，
            // 缺匹号行为由 dyed_fabric_missing_piece_no_rejected 单独锁定
            piece_no: color.map(|_| "P-1".to_string()),
            unit_cost: None,
        }
    }

    fn stock(
        product_id: i32,
        color_no: &str,
        dye_lot_no: Option<&str>,
        batch_no: &str,
        available: &str,
    ) -> inventory_stock::Model {
        inventory_stock::Model {
            id: 1,
            warehouse_id: 1,
            product_id,
            quantity_on_hand: d(available),
            quantity_available: d(available),
            quantity_reserved: Decimal::ZERO,
            quantity_shipped: Decimal::ZERO,
            quantity_incoming: Decimal::ZERO,
            reorder_point: Decimal::ZERO,
            max_stock_point: Decimal::ZERO,
            reorder_quantity: Decimal::ZERO,
            bin_location: None,
            last_count_date: None,
            last_movement_date: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            batch_no: batch_no.to_string(),
            color_no: color_no.to_string(),
            dye_lot_no: dye_lot_no.map(str::to_string),
            grade: "一等品".to_string(),
            production_date: None,
            expiry_date: None,
            quantity_meters: d(available),
            quantity_kg: Decimal::ZERO,
            gram_weight: None,
            width: None,
            location_id: None,
            shelf_no: None,
            layer_no: None,
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
            version: 0,
            replenishment_strategy: "reorder_point".to_string(),
        }
    }

    /// 归一维度 + 内存匹配：复现 check 的核心判定（require_outbound_dimensions + match_single_item_against_stocks）
    fn run_check(
        it: &InventoryTransferItemRequest,
        stocks: &[inventory_stock::Model],
    ) -> Result<(), AppError> {
        let product_id = it.product_id.unwrap();
        let dims = require_outbound_dimensions(
            "调拨出库",
            product_id,
            it.color_no.as_deref(),
            it.dye_lot_no.as_deref(),
            it.batch_no.as_deref(),
            it.piece_no.as_deref(),
        )?;
        match_single_item_against_stocks(
            product_id,
            &dims,
            it.quantity.unwrap_or(Decimal::ZERO),
            stocks,
        )
    }

    #[test]
    fn white_fabric_remains_loose_without_dye_lot() {
        // 白坯布（色号为空）：库存行无缸号也应放行，维持既有宽松行为
        let it = item(1, "50", None, None, Some("B1"));
        let stocks = vec![stock(1, "", None, "B1", "100")];
        run_check(&it, &stocks).expect("白坯无缸号应维持宽松放行");
    }

    #[test]
    fn white_fabric_matches_by_product_and_batch() {
        // 白坯按款号 + 批次匹配：批次不命中则无库存记录被拒
        let stocks = vec![stock(1, "", None, "B1", "100")];
        let ok = item(1, "50", Some(""), None, Some("B1"));
        run_check(&ok, &stocks).expect("白坯款号+批次匹配应放行");

        let wrong_batch = item(1, "50", Some(""), None, Some("B9"));
        let err = run_check(&wrong_batch, &stocks).expect_err("白坯批次不匹配应无库存记录被拒");
        assert!(err.to_string().contains("无匹配库存"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_missing_dye_lot_rejected() {
        // 染色布（色号非空）缺缸号：判定复用 fabric_class 唯一口径，返回明确业务错误（非裸 500）
        let it = item(1, "50", Some("RED"), None, Some("B1"));
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        let err = run_check(&it, &stocks).expect_err("染色布缺缸号必须被拒");
        assert!(err.to_string().contains("缸号"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_requires_all_four_dimensions() {
        // 染色布四维齐全才命中放行
        let ok = item(1, "50", Some("RED"), Some("DL-A"), Some("B1"));
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        run_check(&ok, &stocks).expect("染色布四维齐全应放行");

        // 缸号不同则四维不齐 → 不命中，被拒（证明缸号已纳入染色布匹配）
        let wrong_dye = item(1, "50", Some("RED"), Some("DL-Z"), Some("B1"));
        let err = run_check(&wrong_dye, &stocks).expect_err("染色布缸号不匹配应被拒");
        assert!(err.to_string().contains("无匹配库存"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_four_dim_hit_but_insufficient_quantity_rejected() {
        // 染色布四维命中但数量不足 → 库存不足业务错误
        let it = item(1, "150", Some("RED"), Some("DL-A"), Some("B1"));
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        let err = run_check(&it, &stocks).expect_err("数量不足应被拒");
        assert!(err.to_string().contains("库存不足"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_missing_piece_no_rejected() {
        // 用户 2026-10-02 口径：出库对染色布强制四维=缸/色/批/匹，缺匹号建单期即拒
        let mut it = item(1, "50", Some("RED"), Some("DL-A"), Some("B1"));
        it.piece_no = None;
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        let err = run_check(&it, &stocks).expect_err("染色布缺匹号必须被拒");
        assert!(err.to_string().contains("匹号"), "实际：{}", err);

        // 白坯布免匹号：色号为空时匹号缺失不报错（既有免缸号口径同步豁免匹号）
        let mut white = item(1, "50", None, None, Some("B1"));
        white.piece_no = None;
        run_check(&white, &[stock(1, "", None, "B1", "100")]).expect("白坯缺匹号应放行（不强制）");
    }
}

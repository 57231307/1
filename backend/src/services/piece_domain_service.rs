//! 匹号领域服务（设计见 docs/piece-number-domain-design.md）
//!
//! 领域规则（用户确认，2026-09-05）：
//! - 生产报工逐匹登记生产匹号 + 机台号 + 开机人（胚布无缸号，机台号仅存在于生产环节）
//! - 染色完成后生成染色匹号 + 缸号；染色匹号贯穿入库/外发/销售/出库/对账
//! - 仓库类型约束：胚布仓（greige）只能存放未染色/未做工艺的胚布；
//!   成品仓（finished）只能存放染色/工艺后的成品

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;

use crate::models::inventory_piece;
use crate::utils::error::AppError;

/// 匹类型常量
pub const PIECE_TYPE_GREIGE: &str = "greige";
pub const PIECE_TYPE_DYED: &str = "dyed";

/// 仓库类型与匹类型的兼容校验
/// - 胚布仓（greige）：只能存放未染色/未做工艺的胚布（greige 匹）
/// - 成品仓（finished）：只能存放染色/工艺后的成品（dyed 匹）
/// - 仓库未设置类型（NULL）或非标准类型：不校验（兼容存量仓库）
pub async fn validate_warehouse_for_piece_type<C: ConnectionTrait>(
    db: &C,
    warehouse_id: i32,
    piece_type: &str,
    // 净布工艺豁免：净布工艺完成的胚布匹（无缸号）允许入成品仓
    allow_greige_in_finished: bool,
) -> Result<(), AppError> {
    use sea_orm::EntityTrait;
    let warehouse = crate::models::warehouse::Entity::find_by_id(warehouse_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", warehouse_id)))?;
    let reject = |msg: String| Err(AppError::business(msg));
    match (warehouse.warehouse_type.as_deref(), piece_type) {
        (Some("greige"), PIECE_TYPE_GREIGE) | (Some("finished"), PIECE_TYPE_DYED) => Ok(()),
        // 净布工艺：工艺完成的胚布匹（无缸号）允许入成品仓
        (Some("finished"), PIECE_TYPE_GREIGE) if allow_greige_in_finished => Ok(()),
        (Some("greige"), _) => reject(
            "胚布仓只能存放未染色、未做工艺的胚布，染色后/工艺后的成品请入成品仓".to_string(),
        ),
        (Some("finished"), _) => reject(
            "成品仓只能存放染色后或做工艺后的成品，未染色的生产匹请入胚布仓".to_string(),
        ),
        _ => Ok(()),
    }
}

/// 生产报工逐匹登记输入（生产匹号生成时机 = 生产报工）
#[derive(Debug, Clone, Deserialize)]
pub struct ReportPieceInput {
    /// 生产匹号
    pub piece_no: String,
    /// 机台号（胚布织造机台）
    pub machine_no: Option<String>,
    /// 开机人（什么人开的机器）
    pub machine_operator: Option<String>,
    /// 长度（米，必填）
    pub length: rust_decimal::Decimal,
    /// 重量（千克）
    pub weight: Option<rust_decimal::Decimal>,
    /// 幅宽（cm）
    pub width: Option<rust_decimal::Decimal>,
    /// 克重（g/m²）
    pub gram_weight: Option<rust_decimal::Decimal>,
    /// 入库的胚布仓库（必须为胚布仓或未分类仓库）
    pub warehouse_id: i32,
    /// 生产日期
    pub production_date: Option<chrono::NaiveDate>,
}

/// 生产报工逐匹登记：为工艺单的胚布产出创建生产匹（piece_type=greige）
///
/// - production_order_no：生产单号；生产匹 = 生产单号下产品生产出来的第 * 匹
/// - batch_no 记为生产单号
/// - warehouse_in_at 记录入库胚布仓库的时间（= 登记时刻）
/// - 生产匹无缸号：dye_lot_id/dye_lot_no 为 NULL
#[allow(clippy::too_many_arguments)]
pub async fn create_greige_pieces_from_report<C: ConnectionTrait>(
    db: &C,
    production_order_no: &str,
    product_id: i32,
    operator_id: Option<i32>,
    pieces: &[ReportPieceInput],
) -> Result<Vec<inventory_piece::Model>, AppError> {
    let mut created = Vec::with_capacity(pieces.len());
    for piece in pieces {
        validate_warehouse_for_piece_type(db, piece.warehouse_id, PIECE_TYPE_GREIGE, false).await?;
        let now_utc = crate::utils::date_utils::utc_now_fixed().with_timezone(&chrono::Utc);
        let active = inventory_piece::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            piece_no: Set(piece.piece_no.clone()),
            piece_type: Set(PIECE_TYPE_GREIGE.to_string()),
            machine_no: Set(piece.machine_no.clone()),
            machine_operator: Set(piece.machine_operator.clone()),
            warehouse_in_at: Set(Some(now_utc)),
            // 生产匹无缸号；batch_no = 生产单号（生产匹 = 生产单号下产品的第 * 匹）
            dye_lot_id: Set(None),
            dye_lot_no: Set(None),
            batch_no: Set(production_order_no.to_string()),
            product_id: Set(product_id),
            warehouse_id: Set(piece.warehouse_id),
            length: Set(piece.length),
            weight: Set(piece.weight),
            width: Set(piece.width),
            gram_weight: Set(piece.gram_weight),
            production_date: Set(piece.production_date),
            quality_status: Set(None),
            inventory_status: Set(Some("available".to_string())),
            supplier_piece_no: Set(None),
            position_no: Set(None),
            package_no: Set(None),
            shelf_life: Set(None),
            barcode: Set(Some(piece.piece_no.clone())),
            parent_piece_id: Set(None),
            inspection_id: Set(None),
            piece_seq: Set(None),
            location_id: Set(None),
            scan_type: Set(None),
            status: Set("available".to_string()),
            remarks: Set(Some(format!(
                "生产报工逐匹登记（生产单 {}）",
                production_order_no
            ))),
            created_at: Set(now_utc),
            updated_at: Set(now_utc),
            created_by: Set(operator_id),
            updated_by: Set(None),
            color_no: Set(None),
            original_length: Set(None),
            original_weight: Set(None),
        };
        created.push(active.insert(db).await?);
    }
    Ok(created)
}

/// 委外回仓入库生成匹记录（染色匹 + 缸号；净布工艺为无缸号的胚布匹）
///
/// - 染色外发（订单有 dye_lot_no/dye_batch_id）：回仓必须携带缸号，生成染色匹
/// - 净布外发（订单无缸号信息）：生成无缸号胚布匹，允许入成品仓（净布豁免）
#[allow(clippy::too_many_arguments)]
pub async fn create_piece_from_outsourcing_receipt<C: ConnectionTrait>(
    db: &C,
    receipt_no: &str,
    receipt_dye_lot_no: Option<&str>,
    order_dye_lot_no: Option<&str>,
    product_id: i32,
    warehouse_id: Option<i32>,
    length_m: rust_decimal::Decimal,
    grade: Option<&str>,
    remarks: &str,
) -> Result<Option<inventory_piece::Model>, AppError> {
    use sea_orm::EntityTrait;

    let Some(warehouse_id) = warehouse_id else {
        return Err(AppError::business(
            "委外回仓单未指定入库仓库，无法生成匹记录",
        ));
    };
    // 染色外发：回仓缸号必填（用户规则：染色后必须有缸号）
    let dye_lot_no = match (order_dye_lot_no, receipt_dye_lot_no) {
        (Some(_), Some(lot)) => Some(lot.to_string()),
        (Some(_), None) => {
            return Err(AppError::business(
                "染色外发的回仓单必须填写缸号（dye_lot_no）",
            ))
        }
        // 净布外发：无缸号
        (None, _) => None,
    };
    let is_dyed = dye_lot_no.is_some();
    let piece_type = if is_dyed {
        PIECE_TYPE_DYED
    } else {
        PIECE_TYPE_GREIGE
    };
    // 净布工艺完成的匹允许入成品仓
    validate_warehouse_for_piece_type(db, warehouse_id, piece_type, !is_dyed).await?;

    let dye_lot_id: Option<i32> = if is_dyed {
        let lot_no = dye_lot_no.as_deref().unwrap_or_default();
        // 缸号档案 find_or_create：染色回仓是缸号的产生时机，档案缺失时自动补齐
        // （历史实现直接报错"未建档"，导致染色链路在无手工建档入口时完全走不通）
        let existing = crate::models::batch_dye_lot::Entity::find()
            .filter(
                crate::models::batch_dye_lot::Column::DyeLotNo.eq(lot_no),
            )
            .one(db)
            .await?;
        let lot = match existing {
            Some(lot) => lot,
            None => {
                let now = crate::utils::date_utils::utc_now_fixed().date_naive();
                let active = crate::models::batch_dye_lot::ActiveModel {
                    // batch_no 有单字段 UNIQUE（m0013 DDL），用回仓单号保证唯一；
                    // 缸号维度本身由 dye_lot_no 表达
                    batch_no: Set(format!("{}-RECEIPT", receipt_no)),
                    product_id,
                    color_id: Set(None),
                    dye_lot_no: Set(lot_no.to_string()),
                    dye_date: Set(now),
                    quantity: Set(length_m),
                    color_code: Set(None),
                    status: Set("ACTIVE".to_string()),
                    remarks: Set(Some("委外回仓自动建档（缸号产生时机）".to_string())),
                    ..Default::default()
                };
                active.insert(db).await?
            }
        };
        Some(lot.id)
    } else {
        None
    };

    // 染色匹编号语义：染色匹 = 缸号/染色批次号染色后的第 * 匹。
    // 染色匹 piece_no = {dye_lot_no}-{seq:03}，batch_no = 缸号，piece_seq 同缸递增
    // （与验布打卷链路 generate_next_piece_no 同语义）；净布外发的无缸号胚布匹
    // 用回仓单号维度编号（无缸号可挂）。
    let now_utc = crate::utils::date_utils::utc_now_fixed().with_timezone(&chrono::Utc);
    let (piece_no, piece_seq) = if is_dyed {
        let lot_no = dye_lot_no.as_deref().unwrap_or_default();
        let max_seq_piece = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::DyeLotId.eq(dye_lot_id))
            .filter(inventory_piece::Column::PieceSeq.is_not_null())
            .order_by_desc(inventory_piece::Column::PieceSeq)
            .one(db)
            .await?;
        let next_seq = max_seq_piece
            .as_ref()
            .and_then(|p| p.piece_seq)
            .map(|s| s + 1)
            .unwrap_or(1);
        (
            format!("{}-{:03}", lot_no, next_seq),
            Some(next_seq),
        )
    } else {
        (format!("{}-P01", receipt_no), Some(1))
    };
    let batch_no_str = if is_dyed {
        dye_lot_no.clone().unwrap_or_default()
    } else {
        receipt_no.to_string()
    };
    let active = inventory_piece::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        piece_no: Set(piece_no.clone()),
        piece_type: Set(piece_type.to_string()),
        machine_no: Set(None),
        machine_operator: Set(None),
        warehouse_in_at: Set(Some(now_utc)),
        dye_lot_id: Set(dye_lot_id),
        dye_lot_no: Set(dye_lot_no),
        color_no: Set(None),
        batch_no: Set(batch_no_str),
        product_id: Set(product_id),
        warehouse_id: Set(warehouse_id),
        length: Set(length_m),
        weight: Set(None),
        width: Set(None),
        gram_weight: Set(None),
        production_date: Set(None),
        quality_status: Set(grade.map(|g| g.to_string())),
        inventory_status: Set(Some("available".to_string())),
        supplier_piece_no: Set(None),
        position_no: Set(None),
        package_no: Set(None),
        shelf_life: Set(None),
        barcode: Set(Some(piece_no)),
        parent_piece_id: Set(None),
        inspection_id: Set(None),
        piece_seq: Set(piece_seq),
        location_id: Set(None),
        scan_type: Set(None),
        status: Set("available".to_string()),
        remarks: Set(Some(remarks.to_string())),
        created_at: Set(now_utc),
        updated_at: Set(now_utc),
        created_by: Set(None),
        updated_by: Set(None),
        original_length: Set(None),
        original_weight: Set(None),
    };
    Ok(Some(active.insert(db).await?))
}

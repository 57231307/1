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
use crate::models::status::purchase_inventory::inventory_piece as piece_status;
use crate::models::status::quality_dyeing::batch_dye_lot_status;
use crate::utils::error::AppError;

/// 匹类型常量
pub const PIECE_TYPE_GREIGE: &str = "greige";
pub const PIECE_TYPE_DYED: &str = "dyed";

// =====================================================
// 匹行三列打卷实测值的取值域门（weight / width / gram_weight）
// =====================================================

/// 单列取值域门：`inventory_piece` 的实测值列**非空时必须是正值**。
///
/// 为什么要有这道门（不是重复劳动）：这三列是成品布入库标签的直读源
/// （`services/print_service.rs:4991-5011` 逐列判空后原样印出），标签只判 NULL 不判取值，
/// 所以 0 与负数在这里等同"已实测"会被直接印上实物标签 —— 那是伪造实测档案。
///
/// 同一取值域在本仓有四处彼此指认的实现，取值口径必须逐字符一致（`IS NULL OR > 0`）：
/// - DB CHECK（并发/旁路写入的兜底）：`weight/width/gram_weight` 三条
///   `chk_inventory_piece_*_positive`（migration `m0076`）、收回单同名列
///   `chk_outsourcing_receipt_*_positive`（migration `m0075`）；
/// - 打卷入库存口：`services/fabric_inspection_service.rs:601-617`
///   （文案「打卷入库{列}必须大于 0」）；
/// - 委外收回口：`services/outsourcing_ops/receipt.rs:65-87`；
/// - 本函数：匹行落库前（生产报工建匹 / 收回透传建匹 / 拆匹剪裁 / 剪大货样建匹）。
///
/// 拒绝归校验族（值域越界是"我填错了"），与上面两处的既有口径同族同码；
/// 留空（None）不在此门内 —— NULL 是"未补录"的合法形态，由标签 fail-closed 点名。
pub fn validate_piece_measured_value(
    subject: &str,
    label: &str,
    piece_no: Option<&str>,
    value: Option<rust_decimal::Decimal>,
) -> Result<(), AppError> {
    if let Some(v) = value {
        if v <= rust_decimal::Decimal::ZERO {
            let who = piece_no.map(|p| format!("匹 {p} 的")).unwrap_or_default();
            return Err(AppError::validation_displayable(format!(
                "{subject}{who}{label}必须大于 0（填 0 或负数属伪造实测值，会被直接印上成品布入库标签；暂无实测数据请留空，标签会在补录前按缺值拒绝打印）"
            )));
        }
    }
    Ok(())
}

/// 三列成组校验（列名与标签 fail-closed 点名的列名逐字符一致：重量(weight) /
/// 幅宽(width) / 克重(gram_weight)，让用户在表单、错误文案与标签之间对得上号）。
pub fn validate_piece_measured_triple(
    subject: &str,
    piece_no: Option<&str>,
    weight: Option<rust_decimal::Decimal>,
    width: Option<rust_decimal::Decimal>,
    gram_weight: Option<rust_decimal::Decimal>,
) -> Result<(), AppError> {
    for (label, value) in [
        ("重量(weight)", weight),
        ("幅宽(width)", width),
        ("克重(gram_weight)", gram_weight),
    ] {
        validate_piece_measured_value(subject, label, piece_no, value)?;
    }
    Ok(())
}

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
        (Some("finished"), _) => {
            reject("成品仓只能存放染色后或做工艺后的成品，未染色的生产匹请入胚布仓".to_string())
        }
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
    // 实测值取值域门在任何 DB 访问前逐匹预检（含 DB 读）：三列任一为 0/负数即整批拒，
    // 不给"前几匹已落库、第三匹才失败"的部分写入留窗口（同口径门见
    // `validate_piece_measured_value` 的四处指认；DB 兜底 = m0076 CHECK）。
    for piece in pieces {
        validate_piece_measured_triple(
            "生产报工",
            Some(piece.piece_no.as_str()),
            piece.weight,
            piece.width,
            piece.gram_weight,
        )?;
    }
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
            dye_lot_no: Set(String::new()),
            batch_no: Set(production_order_no.to_string()),
            product_id: Set(product_id),
            warehouse_id: Set(piece.warehouse_id),
            length: Set(piece.length),
            weight: Set(piece.weight),
            width: Set(piece.width),
            gram_weight: Set(piece.gram_weight),
            production_date: Set(piece.production_date),
            quality_status: Set(None),
            inventory_status: Set(Some(piece_status::AVAILABLE.to_string())),
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
            status: Set(piece_status::AVAILABLE.to_string()),
            remarks: Set(Some(format!(
                "生产报工逐匹登记（生产单 {}）",
                production_order_no
            ))),
            created_at: Set(now_utc),
            updated_at: Set(now_utc),
            created_by: Set(operator_id),
            updated_by: Set(None),
            color_no: Set(String::new()),
            original_length: Set(None),
            original_weight: Set(None),
        };
        created.push(active.insert(db).await?);
    }
    Ok(created)
}

/// 匹号领域二期：外发发料前匹号校验
///
/// 染色/印花外发（piece_no 语义单据）发料时，明细必须引用真实存在且可用的生产匹：
/// - 明细集合为空 → 拒绝（逐条校验对空集合 = 0 次校验，整单放行即"空单也能推进
///   issued 并生成发料凭证"的静默数据完整性漏洞，必须在入口显式拒绝）
/// - piece_no 为空 → 拒绝（外发发料必须精确到匹）
/// - 匹不存在 → 拒绝（防止引用虚构匹号导致回仓对不上账）
/// - 匹状态非可用（已预留/已发货/缺陷/不可用）→ 拒绝
pub async fn validate_pieces_for_issue<C: ConnectionTrait>(
    db: &C,
    items: &[crate::models::outsourcing_order_item::Model],
) -> Result<(), AppError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use std::collections::HashMap;

    // 门控是逐条明细校验，空集合会完成 0 次校验直接 Ok——调用方（委外发料）
    // 随后就会推进状态并落发料凭证。域规则"发料精确到匹"蕴含"至少存在一条
    // 明细"，故空明细是业务规则违反，显式拒绝（公开规则文案，无内部标识，
    // 可外显；不含记录 ID）。
    if items.is_empty() {
        return Err(AppError::business_displayable(
            "委外订单没有发料明细，无法发料；发料必须精确到匹，请先登记发料明细",
        ));
    }

    let piece_nos: Vec<String> = items
        .iter()
        .filter_map(|it| it.piece_no.clone())
        .filter(|s| !s.is_empty())
        .collect();

    // 引用的匹批量查询
    let pieces: HashMap<String, inventory_piece::Model> = if piece_nos.is_empty() {
        HashMap::new()
    } else {
        inventory_piece::Entity::find()
            .filter(inventory_piece::Column::PieceNo.is_in(piece_nos.clone()))
            .all(db)
            .await?
            .into_iter()
            .map(|p| (p.piece_no.clone(), p))
            .collect()
    };

    for it in items {
        let piece_no = it.piece_no.as_deref().unwrap_or_default();
        if piece_no.is_empty() {
            return Err(AppError::business(format!(
                "外发明细（缸号 {}）必须填写生产匹号，发料需精确到匹",
                it.dye_lot_no.as_deref().unwrap_or_default()
            )));
        }
        let Some(piece) = pieces.get(piece_no) else {
            return Err(AppError::business(format!(
                "外发明细引用的生产匹 {piece_no} 不存在，请核对匹号"
            )));
        };
        if piece.status != crate::models::status::purchase_inventory::inventory_piece::AVAILABLE {
            return Err(AppError::business(format!(
                "生产匹 {piece_no} 当前状态为 {}（非可用），不可外发发料",
                piece.status
            )));
        }
    }
    Ok(())
}

/// 提取明细引用到的去重匹号（保持首次出现顺序；NULL/空串不入列——
/// 二者的拒绝归因由 `validate_pieces_for_issue` 逐条负责，本函数只做占用面收敛）
fn distinct_referenced_piece_nos(
    items: &[crate::models::outsourcing_order_item::Model],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for it in items {
        if let Some(pn) = it.piece_no.as_deref().filter(|s| !s.is_empty()) {
            if !out.iter().any(|s| s == pn) {
                out.push(pn.to_string());
            }
        }
    }
    out
}

/// 对单个匹号执行「当前值 == from」前提下的比较并交换（CAS）条件更新。
/// 返回 rows_affected（0 = CAS 未命中，由调用方归因，绝不静默）。
///
/// `operator_id` 为审计主体（真实操作人 user_id）：与 status/updated_at 落在同一条
/// update_many 内——CAS 的原子性与零 N+1 是本闭环的立身点，补一次 UPDATE 会引入
/// 「状态已改、审计主体未写」的中间态与额外往返。None 时写 NULL，与迁移回填
/// 「系统路径无操作主体、不伪造 updated_by」同口径。
async fn cas_piece_status<C: ConnectionTrait>(
    conn: &C,
    piece_no: &str,
    from: &str,
    to: &str,
    operator_id: Option<i32>,
) -> Result<u64, AppError> {
    use sea_orm::EntityTrait;
    let result = inventory_piece::Entity::update_many()
        .filter(inventory_piece::Column::PieceNo.eq(piece_no))
        .filter(inventory_piece::Column::Status.eq(from))
        .set(inventory_piece::ActiveModel {
            status: Set(to.to_string()),
            updated_by: Set(operator_id),
            // inventory_piece.updated_at 列型为 DateTime<Utc>（models/inventory_piece.rs），
            // 与本域既有写入点（create_greige_pieces_from_report）同一时钟口径
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        })
        .exec(conn)
        .await?;
    Ok(result.rows_affected)
}

/// CAS 未命中后的归因文案：重读该匹（不存在 vs 当前状态非预期）。
/// 文案携带查询所得匹号/状态，按 `utils/error.rs` 保密分层走脱敏 `business`，
/// 与本文件 `validate_pieces_for_issue` 既有逐条拒绝分支同族。
async fn cas_miss_attribution<C: ConnectionTrait>(
    conn: &C,
    piece_no: &str,
    expected_from: &str,
) -> Result<AppError, AppError> {
    use sea_orm::EntityTrait;
    let current = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq(piece_no))
        .one(conn)
        .await?;
    Ok(match current {
        None => AppError::business(format!(
            "外发明细引用的生产匹 {piece_no} 不存在，请核对匹号"
        )),
        Some(p) => AppError::business(format!(
            "生产匹 {piece_no} 当前状态为 {}（非 {}），占用/释放 CAS 未命中",
            p.status, expected_from
        )),
    })
}

/// CAS 未命中且**不阻断主流程**（取消释放/收回转出）时的归因留痕：
/// 回读该匹现状写入 WARN（只进日志、不出 HTTP）。回读本身的 DB 错误原样上抛——
/// 跳过未命中是业务决定，连接/查询失败是另一回事，不混为一谈也不吞。
async fn warn_cas_miss<C: ConnectionTrait>(
    conn: &C,
    piece_no: &str,
    transition: &str,
) -> Result<(), AppError> {
    use sea_orm::EntityTrait;
    let current = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq(piece_no))
        .one(conn)
        .await?;
    match current {
        Some(p) => tracing::warn!(
            piece_no = %piece_no,
            current_status = %p.status,
            "委外{transition}：生产匹 CAS 未命中（当前状态非 RESERVED），跳过该匹；历史数据或未走占用闭环"
        ),
        None => tracing::warn!(
            piece_no = %piece_no,
            "委外{transition}：生产匹 CAS 未命中且回读无此行，跳过该匹；明细引用与库存存在完整性异常需人工核查"
        ),
    }
    Ok(())
}

/// 委外发料占用：把明细引用的生产匹以 CAS（AVAILABLE→RESERVED）逐匹置为已预留。
///
/// 词表依据：`inventory_piece::RESERVED` 词表语义即「已为订单预留」
/// （models/status/purchase_inventory.rs::inventory_piece），发料占用与之吻合，
/// 不新增词表值。
///
/// 原子性契约：**必须在发料事务内调用**（caller: outsourcing_ops/order.rs::issue_order）。
/// CAS 用「UPDATE ... WHERE status = AVAILABLE」把校验与写入压进同一条语句：
/// - 消除「事务外只读校验 → 事务内提交」的 TOCTOU 窗口；
/// - 不依赖行锁（本仓 sqlite 夹具不支持 lock_exclusive，PG 生产真跑同一套 SQL——
///   条件更新在两种方言下都真实生效，可测性与正确性同源）；
/// - 同一事务内任一行 CAS 未命中即整单拒绝，已占用的行随调用方事务回滚，
///   不允许出现部分匹被占用。
pub async fn reserve_pieces_for_issue<C: ConnectionTrait>(
    conn: &C,
    items: &[crate::models::outsourcing_order_item::Model],
    operator_id: Option<i32>,
) -> Result<(), AppError> {
    // 纯读校验复用既有门控（空明细拒绝/缺匹号/匹不存在/非可用），归因文案不变
    validate_pieces_for_issue(conn, items).await?;

    // 同单内匹号重复必须显式拒绝：validate 的批量 map 会让两条引用同一 AVAILABLE
    // 匹的明细双双放行；若不拦，CAS 循环会把第二条误判成「被前一条自己占用」，
    // 归因失真。文案含匹号（查询所得实体值）→ 脱敏 business。
    let piece_nos = distinct_referenced_piece_nos(items);
    if piece_nos.len() != items.len() {
        let mut seen: Vec<&str> = Vec::new();
        for it in items {
            if let Some(pn) = it.piece_no.as_deref().filter(|s| !s.is_empty()) {
                if seen.contains(&pn) {
                    return Err(AppError::business(format!(
                        "同一委外订单的发料明细重复引用生产匹 {pn}，发料必须一匹一条明细"
                    )));
                }
                seen.push(pn);
            }
        }
    }

    for pn in &piece_nos {
        let rows = cas_piece_status(
            conn,
            pn,
            piece_status::AVAILABLE,
            piece_status::RESERVED,
            operator_id,
        )
        .await?;
        if rows == 0 {
            // 校验刚通过而 CAS 未命中：并发事务已占用该匹（读校验与写占用之间的
            // 真实竞争在条件更新下收敛到这里），重读归因后整单拒绝
            return Err(cas_miss_attribution(conn, pn, piece_status::AVAILABLE).await?);
        }
    }
    tracing::info!(
        piece_count = piece_nos.len(),
        "委外发料匹占用完成（AVAILABLE→RESERVED，随发料事务提交）"
    );
    Ok(())
}

/// 委外取消释放：把明细引用的、当前仍 RESERVED 的匹 CAS 回 AVAILABLE。
///
/// 仅在 issued/processing 订单的取消路径调用（draft 从未占用；received/settled
/// 的匹已在收回确认时转 SHIPPED，不得由取消回退）。
/// CAS 未命中（历史数据从未被占用/状态已被其他链路改写）不阻断取消——
/// 取消本身是业务事实，但必须逐匹 tracing::warn! 留痕归因，不静默。
pub async fn release_reserved_pieces_on_cancel<C: ConnectionTrait>(
    conn: &C,
    items: &[crate::models::outsourcing_order_item::Model],
    operator_id: Option<i32>,
) -> Result<(), AppError> {
    for pn in distinct_referenced_piece_nos(items) {
        let rows = cas_piece_status(
            conn,
            &pn,
            piece_status::RESERVED,
            piece_status::AVAILABLE,
            operator_id,
        )
        .await?;
        if rows == 0 {
            warn_cas_miss(conn, &pn, "取消释放").await?;
        }
    }
    tracing::info!("委外取消释放完成（CAS RESERVED→AVAILABLE，未命中匹已逐条 WARN）");
    Ok(())
}

/// 委外收回确认转出：把明细引用的、当前仍 RESERVED 的匹 CAS 置 SHIPPED
/// （胚布已实物转出到委外商，收回产生的是新建染色匹，原匹不再在库可用）。
///
/// CAS 未命中即跳过并 tracing::warn! 留痕（历史数据在占用闭环上线前发料、
/// 匹从未被置 RESERVED），不静默、也不硬失败收回确认。
pub async fn mark_reserved_pieces_shipped_on_receipt<C: ConnectionTrait>(
    conn: &C,
    items: &[crate::models::outsourcing_order_item::Model],
    operator_id: Option<i32>,
) -> Result<(), AppError> {
    for pn in distinct_referenced_piece_nos(items) {
        let rows = cas_piece_status(
            conn,
            &pn,
            piece_status::RESERVED,
            piece_status::SHIPPED,
            operator_id,
        )
        .await?;
        if rows == 0 {
            warn_cas_miss(conn, &pn, "收回确认转出").await?;
        }
    }
    tracing::info!("委外收回确认转出完成（CAS RESERVED→SHIPPED，未命中匹已逐条 WARN）");
    Ok(())
}

/// 委外回仓生成匹记录的上下文参数（聚合 12 个业务字段，替代 13 参数函数签名
/// 以满足 clippy::too_many_arguments 上限；db 连接保持独立参数）
pub struct OutsourcingReceiptPieceContext<'a> {
    pub receipt_no: &'a str,
    pub receipt_dye_lot_no: Option<&'a str>,
    pub order_dye_lot_no: Option<&'a str>,
    pub color_no: Option<&'a str>,
    pub product_id: i32,
    pub warehouse_id: Option<i32>,
    pub length_m: rust_decimal::Decimal,
    // 收回单实测三列（m0075 建列为 DECIMAL(18,4)，migration/src/domain/production/
    // m0075_add_outsourcing_receipt_measured_values.rs:105）逐列透传：有值必落、无值落 NULL。
    // 不得改用 products.width/gram_weight 之类的标称值兜底，也不得 unwrap_or(ZERO)——成品布入库
    // 标签全取匹行实测值、缺值 fail-closed 逐列点名（print_service.rs:4982-5010），兜底会印出假档案。
    pub weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub grade: Option<&'a str>,
    pub remarks: &'a str,
}

/// 委外回仓入库生成匹记录（染色匹 + 缸号；净布工艺为无缸号的胚布匹）
///
/// - 染色外发（订单有 dye_lot_no/dye_batch_id）：回仓必须携带缸号，生成染色匹
/// - 净布外发（订单无缸号信息）：生成无缸号胚布匹，允许入成品仓（净布豁免）
pub async fn create_piece_from_outsourcing_receipt<C: ConnectionTrait>(
    db: &C,
    ctx: OutsourcingReceiptPieceContext<'_>,
) -> Result<Option<inventory_piece::Model>, AppError> {
    use sea_orm::EntityTrait;

    // 解构上下文为局部变量：与匹表列名逐列同名（weight/width/gram_weight 与
    // inventory_piece 同名列同名同型，透传可逐列对照）
    let OutsourcingReceiptPieceContext {
        receipt_no,
        receipt_dye_lot_no,
        order_dye_lot_no,
        color_no,
        product_id,
        warehouse_id,
        length_m,
        weight,
        width,
        gram_weight,
        grade,
        remarks,
    } = ctx;
    // 透传落库前的取值域门：收回单侧已有同口径门（outsourcing_ops/receipt.rs:65-87 +
    // m0075 CHECK），这里再判一次不是重复劳动而是防"收回单行被旁路改成 0 后继续产匹"——
    // 匹行是标签直读源，门必须落在标签源本身这一侧的写入点（DB 兜底 = m0076 CHECK）。
    validate_piece_measured_triple("委外收回产匹", None, weight, width, gram_weight)?;
    let Some(warehouse_id) = warehouse_id else {
        return Err(AppError::business(
            "委外回仓单未指定入库仓库，无法生成匹记录",
        ));
    };
    // 染色外发：回仓缸号必填（用户规则：染色后必须有缸号）
    // 追溯字段不可空规范：净布外发无缸号存空串
    let dye_lot_no = match (order_dye_lot_no, receipt_dye_lot_no) {
        (Some(_), Some(lot)) => lot.to_string(),
        (Some(_), None) => {
            return Err(AppError::business(
                "染色外发的回仓单必须填写缸号（dye_lot_no）",
            ));
        }
        // 净布外发：无缸号存空串
        (None, _) => String::new(),
    };
    let is_dyed = !dye_lot_no.is_empty();
    let piece_type = if is_dyed {
        PIECE_TYPE_DYED
    } else {
        PIECE_TYPE_GREIGE
    };
    // 净布工艺完成的匹允许入成品仓
    validate_warehouse_for_piece_type(db, warehouse_id, piece_type, !is_dyed).await?;

    let dye_lot_id: Option<i32> = if is_dyed {
        let lot_no = dye_lot_no.as_str();
        // 缸号档案 find_or_create：染色回仓是缸号的产生时机，档案缺失时自动补齐
        // （历史实现直接报错"未建档"，导致染色链路在无手工建档入口时完全走不通）
        let existing = crate::models::batch_dye_lot::Entity::find()
            .filter(crate::models::batch_dye_lot::Column::DyeLotNo.eq(lot_no))
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
                    product_id: Set(product_id),
                    color_id: Set(None),
                    dye_lot_no: Set(lot_no.to_string()),
                    dye_date: Set(now),
                    quantity: Set(length_m),
                    color_code: Set(None),
                    status: Set(batch_dye_lot_status::ACTIVE.to_string()),
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
        let lot_no = dye_lot_no.as_str();
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
        (format!("{}-{:03}", lot_no, next_seq), Some(next_seq))
    } else {
        (format!("{}-P01", receipt_no), Some(1))
    };
    let batch_no_str = if is_dyed {
        dye_lot_no.clone()
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
        // 匹号二期：色号从回仓单/委外订单透传（追溯链闭环）；净布外发无色号存空串
        color_no: Set(color_no.unwrap_or_default().to_string()),
        batch_no: Set(batch_no_str),
        product_id: Set(product_id),
        warehouse_id: Set(warehouse_id),
        length: Set(length_m),
        // 收回单实测三列逐列直落匹行（建列 m0075 → 收回单 DTO → confirm 收回 → 此处，调用点
        // outsourcing_ops/receipt.rs 构造 OutsourcingReceiptPieceContext）；无实测值保持 NULL，
        // 由标签侧 fail-closed 逐列点名拒绝（print_service.rs:4982-5010），此处不回落主数据/不兜底。
        weight: Set(weight),
        width: Set(width),
        gram_weight: Set(gram_weight),
        production_date: Set(None),
        quality_status: Set(grade.map(|g| g.to_string())),
        inventory_status: Set(Some(piece_status::AVAILABLE.to_string())),
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
        status: Set(piece_status::AVAILABLE.to_string()),
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

// =====================================================
// 染色布出库匹号维度（用户 2026-10-02 拍板：出库强制四维 = 缸号/色号/批次/匹号）
// =====================================================

/// 出库匹号定位上下文（一次出库明细的匹号命中口径：款号 + 出库仓 + 缸 + 批 + 匹，
/// 聚合参数以满足 clippy::too_many_arguments 上限；染色匹按 (dye_lot_id, piece_no) 唯一，
/// 匹号跨缸可重复，因此命中必须带缸号/批次/仓库/产品全tuple，不得只按匹号裸匹配）。
#[derive(Debug, Clone, Copy)]
pub struct OutboundPieceContext<'a> {
    pub product_id: i32,
    pub warehouse_id: i32,
    pub dye_lot_no: &'a str,
    pub batch_no: &'a str,
    pub piece_no: &'a str,
}

/// 按出库四维全 tuple 过滤匹记录（染色匹 + 可用态之外不预筛状态：
/// 归因需要区分「匹不存在」/「tuple 不符」/「状态非可用」三种情形）。
fn outbound_piece_filter(ctx: &OutboundPieceContext<'_>) -> sea_orm::Condition {
    use sea_orm::Condition;
    Condition::all()
        .add(inventory_piece::Column::PieceNo.eq(ctx.piece_no))
        .add(inventory_piece::Column::ProductId.eq(ctx.product_id))
        .add(inventory_piece::Column::WarehouseId.eq(ctx.warehouse_id))
        .add(inventory_piece::Column::DyeLotNo.eq(ctx.dye_lot_no))
        .add(inventory_piece::Column::BatchNo.eq(ctx.batch_no))
        .add(inventory_piece::Column::PieceType.eq(PIECE_TYPE_DYED))
}

/// 匹号未命中出库口径时的归因（脱敏 business：文案携带 DB 查询所得状态/ID，不外显）。
///
/// 边界：字段必填在 `fabric_class::normalize_outbound_piece_no` 已拒（VALIDATION 族）；
/// 走到这里说明匹号已填但未命中真实可用库存匹，属"状态门/前置未满足"= BUSINESS 族。
async fn attribute_outbound_piece_miss<C: ConnectionTrait>(
    conn: &C,
    ctx: &OutboundPieceContext<'_>,
) -> Result<AppError, AppError> {
    use sea_orm::EntityTrait;
    let any_with_no = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq(ctx.piece_no))
        .one(conn)
        .await?;
    Ok(match any_with_no {
        None => AppError::business(format!(
            "出库明细引用的匹号 {} 不存在（产品 {} / 仓 {} / 缸 {} / 批 {} 口径下未命中任何匹记录）",
            ctx.piece_no, ctx.product_id, ctx.warehouse_id, ctx.dye_lot_no, ctx.batch_no
        )),
        Some(p) if p.status != piece_status::AVAILABLE => AppError::business(format!(
            "匹号 {} 当前状态为 {}（非可用），不可出库",
            ctx.piece_no, p.status
        )),
        Some(_) => AppError::business(format!(
            "匹号 {} 不属于出库口径（产品 {}/仓 {}/缸 {}/批 {} 不匹配，或该匹非染色匹），不可出库",
            ctx.piece_no, ctx.product_id, ctx.warehouse_id, ctx.dye_lot_no, ctx.batch_no
        )),
    })
}

/// 出库建单/预检阶段：校验染色匹真实存在且可用（只读、不占用）。
///
/// 与 [`consume_dyed_piece_for_outbound`] 同一 tuple 口径，用于调拨建单预检
/// （`inv::stock::check_from_warehouse_inventory`）与销售发货充足性校验
/// （`so::delivery_ops::inventory::check_inventory`），把"假匹号"拦在建单期；
/// 真实消耗仍必须在出库事务内 CAS（消除 TOCTOU，与委外占用闭环同设计）。
pub async fn validate_dyed_piece_for_outbound<C: ConnectionTrait>(
    conn: &C,
    ctx: &OutboundPieceContext<'_>,
) -> Result<(), AppError> {
    use sea_orm::EntityTrait;
    let hit = inventory_piece::Entity::find()
        .filter(outbound_piece_filter(ctx))
        .filter(inventory_piece::Column::Status.eq(piece_status::AVAILABLE))
        .one(conn)
        .await?;
    match hit {
        Some(_) => Ok(()),
        None => Err(attribute_outbound_piece_miss(conn, ctx).await?),
    }
}

/// 出库事务内消耗染色匹：按四维全 tuple CAS（AVAILABLE→SHIPPED），未命中即整单拒绝。
///
/// 原子性契约：必须在出库事务内调用（调用方：调拨发运 `inv::batch::apply_ship_item_deduction`、
/// 销售发货 `so::delivery_ops::inventory::reduce_inventory_four_dim`）。条件更新把校验与写入
/// 压进同一条语句：同一匹被并发/重复引用时第二次 CAS 必不命中，随调用方事务整体回滚，
/// 绝不静默放行（sqlite 夹具与 PG 生产同一套 SQL 真实生效，同 `reserve_pieces_for_issue` 先例）。
pub async fn consume_dyed_piece_for_outbound<C: ConnectionTrait>(
    conn: &C,
    ctx: &OutboundPieceContext<'_>,
    operator_id: Option<i32>,
) -> Result<(), AppError> {
    use sea_orm::EntityTrait;
    let result = inventory_piece::Entity::update_many()
        .filter(outbound_piece_filter(ctx))
        .filter(inventory_piece::Column::Status.eq(piece_status::AVAILABLE))
        .set(inventory_piece::ActiveModel {
            status: Set(piece_status::SHIPPED.to_string()),
            updated_by: Set(operator_id),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        })
        .exec(conn)
        .await?;
    if result.rows_affected == 0 {
        // 建单校验通过而消耗未命中：并发出库已消耗该匹（或匹 tuple 在建单后被改写），
        // 重读归因后整单拒绝，事务回滚。
        return Err(attribute_outbound_piece_miss(conn, ctx).await?);
    }
    Ok(())
}

/// 销售发货取消：把该出库明细引用的染色匹回退（SHIPPED→AVAILABLE，与库存回加对称反向）。
///
/// 按匹号 + 产品 + 仓库 + 批次 tuple 找到当前确为 SHIPPED 的匹行后逐行按主键 CAS，
/// 防止同匹号跨缸歧义（染色匹 (dye_lot_id, piece_no) 唯一，匹号本身全局可重复）。
/// 未命中（历史数据出库时未走匹号闭环 / 已被扫码链路改写）不阻断取消——取消本身是业务事实，
/// 但必须逐匹 tracing::warn! 留痕归因，不静默（同 `release_reserved_pieces_on_cancel` 先例）。
pub async fn restore_pieces_on_delivery_cancel<C: ConnectionTrait>(
    conn: &C,
    product_id: i32,
    warehouse_id: i32,
    batch_no: &str,
    piece_no: &str,
    operator_id: Option<i32>,
) -> Result<(), AppError> {
    use sea_orm::EntityTrait;
    let candidates = inventory_piece::Entity::find()
        .filter(
            sea_orm::Condition::all()
                .add(inventory_piece::Column::PieceNo.eq(piece_no))
                .add(inventory_piece::Column::ProductId.eq(product_id))
                .add(inventory_piece::Column::WarehouseId.eq(warehouse_id))
                .add(inventory_piece::Column::BatchNo.eq(batch_no))
                .add(inventory_piece::Column::PieceType.eq(PIECE_TYPE_DYED))
                .add(inventory_piece::Column::Status.eq(piece_status::SHIPPED)),
        )
        .all(conn)
        .await?;
    if candidates.is_empty() {
        tracing::warn!(
            piece_no = %piece_no,
            product_id,
            warehouse_id,
            "发货取消回退：未找到该出库明细引用、且当前仍为 SHIPPED 的染色匹（历史数据未走匹号闭环或状态已被改写），跳过该匹回退"
        );
        return Ok(());
    }
    for p in candidates {
        let result = inventory_piece::Entity::update_many()
            .filter(inventory_piece::Column::Id.eq(p.id))
            .filter(inventory_piece::Column::Status.eq(piece_status::SHIPPED))
            .set(inventory_piece::ActiveModel {
                status: Set(piece_status::AVAILABLE.to_string()),
                updated_by: Set(operator_id),
                updated_at: Set(chrono::Utc::now()),
                ..Default::default()
            })
            .exec(conn)
            .await?;
        if result.rows_affected == 0 {
            tracing::warn!(
                piece_no = %piece_no,
                piece_id = p.id,
                "发货取消回退：匹行 CAS（SHIPPED→AVAILABLE）未命中（并发状态改写），跳过该匹，需人工核查账实一致性"
            );
        }
    }
    Ok(())
}

#![allow(dead_code)]
//! 采购库存状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的采购/库存状态常量子模块组。
//! 包含：purchase_order/purchase_receipt/inventory_reservation/inventory_transfer/inventory_count/purchase_return/purchase_inspection/inventory_adjustment/inventory_piece/purchase_receipt_inspection
//!
//! 大小写历史包袱说明：本模块下 status 字面量大小写混杂，源于历史数据库迁移遗留——
//! 单据类（purchase_order/purchase_receipt）用大写（DRAFT/APPROVED/CLOSED），
//! 库存操作类（inventory_reservation/inventory_transfer/inventory_count 等）用小写（pending/locked/consumed）。
//! 各子模块内字面量值保持与数据库现状一致，新增状态需先确认对应表的既有值大小写再定义常量。

// 采购订单状态
pub mod purchase_order {
    /// 草稿
    pub const DRAFT: &str = "DRAFT";
    /// 待审批
    pub const PENDING_APPROVAL: &str = "PENDING_APPROVAL";
    /// 已提交
    pub const SUBMITTED: &str = "SUBMITTED";
    /// 已审批
    pub const APPROVED: &str = "APPROVED";
    /// 已拒绝
    pub const REJECTED: &str = "REJECTED";
    /// 已关闭
    pub const CLOSED: &str = "CLOSED";
    /// 已取消（批次 215 真实接入：cancel_order 方法使用）
    pub const CANCELLED: &str = "CANCELLED";
    /// 已完成
    pub const COMPLETED: &str = "COMPLETED";
    /// 部分收货
    pub const PARTIAL_RECEIVED: &str = "PARTIAL_RECEIVED";
}

/// 采购收货单状态常量（purchase_receipt.receipt_status 大写，批次 214 P2-1，状态机 DRAFT→CONFIRMED→COMPLETED）
pub mod purchase_receipt {
    /// 草稿：收货单初始状态，可编辑
    pub const DRAFT: &str = "DRAFT";
    /// 已确认：收货已确认，等待入库
    pub const CONFIRMED: &str = "CONFIRMED";
    /// 已完成：收货入库流程完成（幂等键）
    pub const COMPLETED: &str = "COMPLETED";
}

// 库存预留状态（inventory_reservation.status，小写值）
// 批次 158 v11 真实接入：so/delivery.rs 中库存预留状态字符串字面量统一引用此模块（规则 0）
// 批次 341 v11 复审 P2 修复：LOCKED/RELEASED 常量已被 inventory_reservation_service.rs 和测试代码广泛使用，
// 移除过时的 #[allow(dead_code)] 抑制（lock_reservation/release_reservation 方法已真实实现）。
pub mod inventory_reservation {
    /// 待处理（已创建预留，等待发货扣减）
    pub const PENDING: &str = "pending";
    /// 已锁定（库存已锁定，等待发货扣减）
    pub const LOCKED: &str = "locked";
    /// 已消耗（发货已扣减库存，原 FULFILLED 值修正为 consumed 与业务代码一致）
    pub const CONSUMED: &str = "consumed";
    /// 已释放（订单取消或库存不足释放）
    pub const RELEASED: &str = "released";
    /// 已取消（订单取消或库存不足释放）
    pub const CANCELLED: &str = "cancelled";
}

/// 库存调拨状态（inventory_transfer.status，小写值）
/// 批次 234 v13 真实接入：inv/inventory_move.rs 中调拨状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_transfer {
    /// 待处理：调拨单初始状态，可审批
    pub const PENDING: &str = "pending";

    /// 已审批：审批通过，可发货
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";

    /// 已发货：调拨已发出，待接收
    pub const SHIPPED: &str = "shipped";

    /// 已完成：调拨流程完结
    pub const COMPLETED: &str = "completed";
}

/// 库存盘点状态（inventory_count.status，小写值）
/// 批次 234 v13 真实接入：inventory_count_service.rs 中盘点状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_count {
    /// 待处理：盘点单初始状态，可执行盘点
    pub const PENDING: &str = "pending";

    /// 复核中：盘点数已录入、等待复核确认（submit_review 写入，
    /// approve_review / revert 分别向后/向前流转）。此前它只是
    /// inventory_count_service.rs 里的三处裸字面量，词表里没有这一行，
    /// 模块注释却声称"状态字面量统一引用此模块（规则 0）"。
    pub const IN_REVIEW: &str = "in_review";

    /// 已完成：盘点流程完结
    pub const COMPLETED: &str = "completed";
}

/// 采购退货状态（purchase_return.return_status，小写值）
/// 批次 234 v13 真实接入：purchase_return_service.rs 中退货状态字符串字面量统一引用此模块（规则 0）
pub mod purchase_return {
    /// 草稿：退货单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 已提交：等待审批
    pub const SUBMITTED: &str = "submitted";

    /// 已审批：审批通过，可执行退货
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";
}

/// 采购检验状态（purchase_inspection.inspection_status，小写值）
/// 批次 234 v13 真实接入：purchase_inspection_service.rs 中检验状态字符串字面量统一引用此模块（规则 0）
pub mod purchase_inspection {
    /// 待处理：检验单初始状态，可执行检验
    pub const PENDING: &str = "pending";

    /// 已完成：检验流程完结
    pub const COMPLETED: &str = "completed";
}

/// 库存调整状态（inventory_adjustment.status，小写值）
/// 批次 234 v13 真实接入：inventory_adjustment_service.rs 中调整状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_adjustment {
    /// 待处理：调整单初始状态，可审批
    pub const PENDING: &str = "pending";

    /// 已审批：审批通过，已应用调整
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";
}

/// 库存裁片状态（inventory_piece.status，大写值）
/// 批次 236 v13 真接入：barcode_scanner_handler.rs、piece_split_handler.rs 中裁片状态字符串字面量统一引用此模块（规则 0）
pub mod inventory_piece {
    /// 已发货：裁片已发货
    pub const SHIPPED: &str = "SHIPPED";

    /// 缺陷：裁片有缺陷
    pub const DEFECT: &str = "DEFECT";

    /// 可用：裁片可用
    pub const AVAILABLE: &str = "AVAILABLE";

    /// 不可用：裁片不可用
    pub const UNAVAILABLE: &str = "UNAVAILABLE";

    /// 已预留：裁片已为订单预留
    pub const RESERVED: &str = "RESERVED";

    /// 样布：剪大货样产生的样布（V15 P0-F16 批色流程）
    pub const SAMPLE: &str = "SAMPLE";
}

/// 采购收货检验状态（purchase_receipt.inspection_status，大写值）
/// 批次 236 v13 真实接入：purchase_receipt_service.rs
///
/// 本列与 `quality_inspection_records.inspection_result`（中文：待检/合格/不合格）是
/// 两张表的两套词表：质检结论回写入库单时必须经 `from_inspection_result` 显式映射，
/// 直接复制中文值会让本列出现没有任何读取方认识的取值。
pub mod purchase_receipt_inspection {
    use crate::models::status::quality_inspection_result;

    /// 待检验
    pub const PENDING: &str = "PENDING";

    /// 质检合格：允许后续入库/结算流转
    pub const PASSED: &str = "PASSED";

    /// 质检不合格：走让步接收或退货流程
    pub const REJECTED: &str = "REJECTED";

    /// 本列全部合法取值
    pub const ALL: &[&str] = &[PENDING, PASSED, REJECTED];

    /// 质检结论 → 入库单检验状态；词表外结论返回 `None`，由调用方报错而不是默认成某个值。
    /// 用显式比较而非 match 常量模式，与本仓其余取值域校验写法保持一致（避免引用比较歧义）。
    pub fn from_inspection_result(result: &str) -> Option<&'static str> {
        if result == quality_inspection_result::PENDING {
            Some(PENDING)
        } else if result == quality_inspection_result::QUALIFIED {
            Some(PASSED)
        } else if result == quality_inspection_result::UNQUALIFIED {
            Some(REJECTED)
        } else {
            None
        }
    }
}

/// 库存台账状态（inventory_stocks.stock_status，中文值——面料行业库存主数据）
///
/// 系统当前真实写入的三个值：新建/收货入帐为 NORMAL，批色报废流程为 SCRAPPED，
/// 删除走软删除置 DELETED（行保留用于追溯）。此前这些值以字面量散落在库存服务、
/// 报废流程与批量入仓等 6 处，筛选条件与写入值靠字符串巧合对齐，
/// 前端筛选因此提交 normal/warning/frozen 这类库里根本不存在的值而恒零命中。
/// 库存质量状态（inventory_stocks.quality_status）取值。
///
/// 与库存台账状态 `inventory_stock_status` 是两列不同语义：本列描述质检结论，
/// 可用量、缺料预警与可出库筛选都按 PASS 取值过滤，写入侧不得使用其他域的
/// 检验状态字面量（如 passed），否则该行对所有可用性查询永久不可见。
pub mod inventory_stock_quality_status {
    /// 合格：质检通过，可参与可用量计算与出库
    pub const PASS: &str = "合格";

    /// 待检：降级或复验中，需重新质检后才可判合格
    pub const PENDING: &str = "待检";

    /// 不合格：报废或不合格判定，不得出库
    pub const FAIL: &str = "不合格";

    /// 本列全部合法取值：写入侧校验的取值来源（库存质量状态不可由界面自由提交，
    /// 但建单初始值必须落在本域内，见 handlers::inventory_stock_handler::initial_stock_statuses）
    pub const ALL: &[&str] = &[PASS, PENDING, FAIL];
}

/// 库存等级常量（inventory_stocks.grade，中文稳定值即库内取值）
///
/// 等级只有这三档，降级规则为一等品 → 二等品 → 等外品（等外品已是最低档）；
/// 匹号/验布另有一套 A/B/C 的 grade 取值域，与本列无关，不得互相赋值。
/// 界面上曾用译文当提交值，切换语言即把 "First Grade" 之类写进本列（见
/// frontend/src/constants/stock-grade.ts），因此写入与校验一律取本模块常量。
pub mod inventory_stock_grade {
    /// 一等品：标准品，按 A 级定价
    pub const FIRST: &str = "一等品";

    /// 二等品：让步接收/降级销售，按 A 级标准价的 80% 定价
    pub const SECOND: &str = "二等品";

    /// 等外品：不合格降级，只能返工或报废
    pub const OFF_GRADE: &str = "等外品";

    /// 全部合法等级，校验与降级映射的唯一取值来源
    pub const ALL: &[&str] = &[FIRST, SECOND, OFF_GRADE];
}

pub mod inventory_stock_status {
    /// 正常：可出库的在库库存
    pub const NORMAL: &str = "正常";

    /// 报废：批色报废流程置位，保留行与报废原因用于追溯
    pub const SCRAPPED: &str = "报废";

    /// 已删除：软删除标记，不再是在库库存，列表与导出默认不展示
    pub const DELETED: &str = "已删除";

    /// 本列全部合法取值：写入侧与列表/导出筛选校验的唯一取值来源。
    /// 通用主数据状态 `master_data::ACTIVE`（`active`）属另一张表，不得写入本列，
    /// 也不得用于过滤本列（按它过滤恒零命中）。
    pub const ALL: &[&str] = &[NORMAL, SCRAPPED, DELETED];
}

/// 坯布状态（greige_fabrics.status，中文值——坯布库存主数据）
///
/// 系统当前真实写入的两个值：建单/入库置 `在库`，出库后按剩余量归零则置 `已出库`
/// （见 handlers/greige_fabric_handler.rs 的 create/stock_in/stock_out）。删除走
/// `is_deleted` 软删标记，不占用本列取值，故本列没有「已删除」这类值。
/// 本列取值域是中文业务 token，与通用主数据状态 `active`/`inactive` 不通用：
/// 前端表单曾直接提交英文 `active/inactive`，写库后与门控/出库中文值逐字符不符，
/// 导致「在库不允许删除」等门控永不命中、库里中英混杂。写入侧校验与门控比较一律
/// 取本模块常量，禁止引入英文 token 或散落裸字面量。
pub mod greige_fabric_status {
    /// 在库：坯布已入库、可出库的正常库存（建单默认、入库、出库后仍有剩余的取值）
    pub const IN_STOCK: &str = "在库";

    /// 已出库：出库后剩余重量与长度均归零，已非可用库存
    pub const STOCKED_OUT: &str = "已出库";

    /// 本列全部合法取值：写入侧入参校验与门控比较的唯一取值来源。
    pub const ALL: &[&str] = &[IN_STOCK, STOCKED_OUT];
}

/// 缺料预警状态常量（material_shortage_alerts.status，小写值）
///
/// 状态机：identified → purchase_request → purchase_order → received → resolved。
/// 检测侧新建预警恒为 identified（persist_alerts），只有 update_status 会推进到后续值；
/// 前端可选值必须以本常量为源，此前提交的 pending / notified 不在状态机内，
/// 会被入参校验判为非法（写不进去），列表的状态筛选因此恒零命中。
pub mod shortage_alert_status {
    /// 已识别：检测到缺料并已落库，尚未发起采购
    pub const IDENTIFIED: &str = "identified";

    /// 已发起采购申请
    pub const PURCHASE_REQUEST: &str = "purchase_request";

    /// 已转为采购订单
    pub const PURCHASE_ORDER: &str = "purchase_order";

    /// 采购到货已入库，缺口待解除
    pub const RECEIVED: &str = "received";

    /// 已解除：缺料闭环，后续检测不再复用该行（同物料再次缺料会新建预警）
    pub const RESOLVED: &str = "resolved";

    /// 全部合法缺料预警状态，入参校验的唯一取值来源
    pub const ALL: &[&str] = &[
        IDENTIFIED,
        PURCHASE_REQUEST,
        PURCHASE_ORDER,
        RECEIVED,
        RESOLVED,
    ];
}

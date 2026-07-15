//! 业务状态常量模块
//!
//! 批次 100 P3-A 修复（v5 复审）：抽取 4 个 service 文件中的硬编码状态字符串为常量，
//! 提高可维护性，避免字符串拼写错误导致状态匹配失败。
//!
//! 按业务域分组：
//! - 通用状态：DRAFT/PENDING/APPROVED/CANCELLED/COMPLETED/ACTIVE（多业务共用）
//! - 生产订单专属：SCHEDULED/IN_PROGRESS/PENDING_APPROVAL/REJECTED
//! - 付款专属：REGISTERED/CONFIRMED/PAID/PARTIAL_PAID
//! - 采购订单状态：批次 158 v11 真实接入 po/ 子模块
//! - 销售订单状态：批次 158 v11 真实接入 so/ 子模块
//! - 通用审批状态：批次 158 v11 真实接入 color_price / budget_adjustment / ar_invoice
//! - 库存预留状态：批次 158 v11 真实接入 so/delivery
//! - 销售发货状态：批次 158 v11 真实接入 so/delivery

/// 通用状态常量（多业务共用）
pub mod common {
    /// 草稿：单据初始状态，可编辑/删除
    pub const STATUS_DRAFT: &str = "DRAFT";

    /// 待处理：通用待办或审批中状态
    pub const STATUS_PENDING: &str = "PENDING";

    /// 已审批：审批流程通过
    pub const STATUS_APPROVED: &str = "APPROVED";

    /// 已取消：单据作废，不可再变更
    pub const STATUS_CANCELLED: &str = "CANCELLED";

    /// 已完成：业务流程完结
    pub const STATUS_COMPLETED: &str = "COMPLETED";

    /// 激活：主数据或库存可用状态
    pub const STATUS_ACTIVE: &str = "ACTIVE";
}

/// 生产订单状态常量
pub mod production {
    /// 已排产：草稿审批通过后排入生产计划
    pub const PRODUCTION_SCHEDULED: &str = "SCHEDULED";

    /// 生产中：实际开始投料生产
    pub const PRODUCTION_IN_PROGRESS: &str = "IN_PROGRESS";

    /// 待审批：提交审批流程，等待审批结果
    pub const PRODUCTION_PENDING_APPROVAL: &str = "PENDING_APPROVAL";

    /// 已拒绝：审批未通过，可退回草稿重新编辑
    pub const PRODUCTION_REJECTED: &str = "REJECTED";
}

/// 付款状态常量
pub mod payment {
    /// 已登记：付款单已创建但未确认执行
    pub const PAYMENT_REGISTERED: &str = "REGISTERED";

    /// 已确认：付款已执行，等待银行对账
    pub const PAYMENT_CONFIRMED: &str = "CONFIRMED";

    /// 已付款：全部结清（应收/应付通用复用）
    pub const PAYMENT_PAID: &str = "PAID";

    /// 部分付款：尚未结清（应收/应付通用复用）
    pub const PAYMENT_PARTIAL_PAID: &str = "PARTIAL_PAID";
}

/// 应收账款专属状态常量（批次 102 v6 P3-1 修复，批次 231 v13 P1-1 统一小写）
///
/// 三类模型的状态字段值（批次 231 统一为小写，修复 ar_service.rs 大写与 ar/recon.rs 小写不一致的 P0 数据问题）：
/// - ar_collection.status：小写（pending/confirmed/cancelled）
/// - ar_reconciliation.reconciliation_status：小写（draft/sent/confirmed/disputed/closed/cancelled）
/// - ar_reconciliation_item.match_status：大写（MATCHED/UNMATCHED）
///
/// ar_invoice.status 复用 common::STATUS_* 与 payment::PAYMENT_*（与 ar_invoice_service.rs 保持一致）
pub mod ar {
    /// 收款单待确认（ar_collection.status，小写值）
    pub const COLLECTION_PENDING: &str = "pending";

    /// 收款单已确认（ar_collection.status，小写值）
    pub const COLLECTION_CONFIRMED: &str = "confirmed";

    /// 收款单已取消（ar_collection.status，小写值）
    pub const COLLECTION_CANCELLED: &str = "cancelled";

    /// 对账单草稿（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_DRAFT: &str = "draft";

    /// 对账单已发送（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_SENT: &str = "sent";

    /// 对账单已确认（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_CONFIRMED: &str = "confirmed";

    /// 对账单有争议（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_DISPUTED: &str = "disputed";

    /// 对账单已关闭（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_CLOSED: &str = "closed";

    /// 对账单已取消（ar_reconciliation.reconciliation_status，小写值）
    pub const RECONCILIATION_CANCELLED: &str = "cancelled";

    /// 核销明细已匹配（ar_reconciliation_item.match_status，大写值）
    pub const MATCH_MATCHED: &str = "MATCHED";

    /// 核销明细未匹配（ar_reconciliation_item.match_status，大写值）
    pub const MATCH_UNMATCHED: &str = "UNMATCHED";
}

/// 应付发票专属状态常量（批次 102 v6 P3-2 修复）
///
/// ap_invoice.invoice_status 字段值（大写）：
/// - DRAFT：草稿（与 common::STATUS_DRAFT 同值，但语义独立，单独定义便于维护）
/// - AUDITED：已审核（ap_invoice 专属，非通用审批状态）
/// - PAID：已付款（与 payment::PAYMENT_PAID 同值）
/// - PARTIAL_PAID：部分付款（与 payment::PAYMENT_PARTIAL_PAID 同值）
/// - CANCELLED：已取消（与 common::STATUS_CANCELLED 同值）
///
/// 复用策略：DRAFT/PAID/PARTIAL_PAID/CANCELLED 直接引用 common/payment 模块常量，
/// 仅 AUDITED 在本模块单独定义。
pub mod ap_invoice {
    /// 已审核（ap_invoice 专属状态，区别于通用 APPROVED）
    pub const INVOICE_AUDITED: &str = "AUDITED";
}

/// 应付付款申请专属审批状态常量（批次 102 v6 P3-3 修复）
///
/// ap_payment_request.approval_status 字段值（大写）：
/// - DRAFT：草稿（与 common::STATUS_DRAFT 同值，复用）
/// - APPROVING：审批中（ap_payment_request 专属）
/// - APPROVED：已审批（与 common::STATUS_APPROVED 同值，复用）
/// - REJECTED：已拒绝（与 approval::REJECTED 同值，但 approval 模块为 dead_code，
///   此处单独定义避免依赖 dead_code 模块）
pub mod ap_payment_request {
    /// 审批中（ap_payment_request 专属状态）
    pub const APPROVAL_APPROVING: &str = "APPROVING";

    /// 已拒绝（避免依赖 dead_code 的 approval::REJECTED，单独定义）
    pub const APPROVAL_REJECTED: &str = "REJECTED";
}

/// 凭证状态常量（批次 102 v6 P3-1 修复）
///
/// voucher.status 字段值（小写，凭证专属状态机）：
/// draft → submitted → reviewed → posted
///
/// 注意：与 ar_invoice / ap_invoice 的大写状态值不同，凭证全部用小写。
pub mod voucher {
    /// 草稿：凭证初始状态，可编辑
    pub const VOUCHER_DRAFT: &str = "draft";

    /// 已提交：等待审核
    pub const VOUCHER_SUBMITTED: &str = "submitted";

    /// 已审核：审核通过，等待过账
    pub const VOUCHER_REVIEWED: &str = "reviewed";

    /// 已过账：已记入账簿，不可再修改
    pub const VOUCHER_POSTED: &str = "posted";
}

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

/// 采购收货单状态常量（purchase_receipt.receipt_status，大写值）
///
/// 批次 214 P2-1 修复（v12 复审）：抽取 purchase_receipt_service.rs 和 po/receipt.rs 中的硬编码状态字符串
/// 状态机：DRAFT → CONFIRMED → COMPLETED
pub mod purchase_receipt {
    /// 草稿：收货单初始状态，可编辑
    pub const DRAFT: &str = "DRAFT";
    /// 已确认：收货已确认，等待入库
    pub const CONFIRMED: &str = "CONFIRMED";
    /// 已完成：收货入库流程完成（幂等键）
    pub const COMPLETED: &str = "COMPLETED";
}

// 销售订单状态
// 批次 14（2026-06-28）：修正常量值为小写，与业务代码（order_workflow.rs/order_crud.rs/delivery.rs）一致；
// 补全 partial_shipped 和 shipped 状态；删除业务中不存在的 PENDING_APPROVAL 和 CONFIRMED。
// 原常量值大写（"DRAFT"）与业务小写（"draft"）矛盾，若被引用会查不到数据（隐性 P0 风险）。
// 批次 158 v11 真实接入：移除 allow 标注，业务代码引用常量替代字符串字面量（规则 0）
pub mod sales_order {
    /// 草稿
    pub const DRAFT: &str = "draft";
    /// 待审核
    pub const PENDING: &str = "pending";
    /// 已审核
    pub const APPROVED: &str = "approved";
    /// 部分发货
    pub const PARTIAL_SHIPPED: &str = "partial_shipped";
    /// 已发货
    pub const SHIPPED: &str = "shipped";
    /// 已完成
    pub const COMPLETED: &str = "completed";
    /// 已取消
    pub const CANCELLED: &str = "cancelled";
    /// 已拒绝（so/contract.rs reject_order 接入，批次 158 v11 真实接入）
    pub const REJECTED: &str = "rejected";
}

// 通用审批状态（批次 158 v11 真实接入：color_price / budget_adjustment / ar_invoice 业务引用）
// 注：DRAFT 和 CANCELLED 在当前业务中无使用场景已删除；如未来审批流程扩展需要可重新添加
pub mod approval {
    /// 待审批
    pub const PENDING: &str = "PENDING";
    /// 已审批
    pub const APPROVED: &str = "APPROVED";
    /// 已拒绝
    pub const REJECTED: &str = "REJECTED";
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

// 销售发货状态（sales_delivery.status，小写值）
// 批次 158 v11 真实接入：so/delivery.rs 中发货状态字符串字面量统一引用此模块（规则 0）
pub mod sales_delivery {
    /// 待处理
    pub const PENDING: &str = "pending";
    /// 已发货
    pub const SHIPPED: &str = "shipped";
    /// 已取消（批次 216 真实接入：cancel_delivery 方法使用）
    pub const CANCELLED: &str = "cancelled";
}

/// 主数据启用/停用状态（小写值）
///
/// 批次 208 P2-5 修复（v12 复审）：
/// supplier/customer/fixed_asset 等主数据的 status 字段使用小写 "active"/"inactive"，
/// 与 common::STATUS_ACTIVE（大写 "ACTIVE"）不同，单独定义避免大小写混淆。
pub mod master_data {
    /// 启用：主数据可用状态
    pub const ACTIVE: &str = "active";

    /// 停用：主数据不可用状态
    pub const INACTIVE: &str = "inactive";
}

/// 预算管理状态常量（小写值）
///
/// 批次 209 P2-5 修复（v12 复审）：
/// budget_plan.status 与 budget_management.status（预算项目）使用小写状态值，
/// 状态机：draft → rejected / approved → active
pub mod budget {
    /// 草稿：预算方案初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";

    /// 已审批：审批通过，等待执行
    pub const APPROVED: &str = "approved";

    /// 执行中：预算方案已激活，预算项目处于活跃状态
    pub const ACTIVE: &str = "active";
}

/// 合同状态常量（小写值）
///
/// 批次 210 P2-5 修复（v12 复审）：
/// sales_contract.status 与 purchase_contract.status 使用小写状态值，
/// 状态机：draft → active → cancelled
pub mod contract {
    /// 草稿：合同初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 活跃：合同已激活，可执行
    pub const ACTIVE: &str = "active";

    /// 已取消：合同作废
    pub const CANCELLED: &str = "cancelled";
}

/// 会计期间状态常量（大写值）
///
/// 批次 232 v13 P1-1 修复：accounting_period.status 字段状态常量化
/// 状态机：OPEN（开放）→ CLOSED（已关账）
pub mod accounting_period {
    /// 开放：期间可进行凭证录入
    pub const OPEN: &str = "OPEN";

    /// 已关账：期间已结账，不可再录入凭证
    pub const CLOSED: &str = "CLOSED";
}

/// 运单状态常量（大写值）
///
/// 批次 232 v13 P1-1 修复：logistics_waybill.status 字段状态常量化
/// 状态机：IN_TRANSIT（运输中）→ DELIVERED（已送达）
pub mod logistics_waybill {
    /// 运输中：运单已创建，货物在途
    pub const IN_TRANSIT: &str = "IN_TRANSIT";

    /// 已送达：货物已送达目的地
    pub const DELIVERED: &str = "DELIVERED";
}

/// 销售退货状态常量（大写值）
///
/// 批次 232 v13 P1-1 修复：sales_return.status 字段状态常量化
/// 状态机：DRAFT → SUBMITTED → APPROVED → COMPLETED / REJECTED
pub mod sales_return {
    /// 草稿：退货单初始状态，可编辑
    pub const DRAFT: &str = "DRAFT";

    /// 已提交：等待审批
    pub const SUBMITTED: &str = "SUBMITTED";

    /// 已审批：审批通过，可执行退货
    pub const APPROVED: &str = "APPROVED";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "REJECTED";

    /// 已完成：退货流程完结
    pub const COMPLETED: &str = "COMPLETED";
}

/// 排程结果状态（scheduling_result.status，大写值）
/// 批次 234 v13 真实接入：scheduling_query.rs 中排程结果状态字符串字面量统一引用此模块（规则 0）
pub mod scheduling {
    /// 草稿：排程结果初始状态，可确认
    pub const DRAFT: &str = "DRAFT";

    /// 已确认：排程结果已确认，已应用到生产订单
    pub const CONFIRMED: &str = "CONFIRMED";
}

/// 应付对账状态（ap_reconciliation.reconciliation_status，大写值）
/// 批次 234 v13 真实接入：ap_reconciliation_service.rs 中对账状态字符串字面量统一引用此模块（规则 0）
pub mod ap_reconciliation {
    /// 待处理：对账单初始状态，可执行对账
    pub const PENDING: &str = "PENDING";

    /// 已确认：对账完成，已锁定
    pub const CONFIRMED: &str = "CONFIRMED";

    /// 有争议：对账结果存在异议，需复核
    pub const DISPUTED: &str = "DISPUTED";
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

/// 报价单状态（quotation.status，小写值）
/// 批次 234 v13 真实接入：quotation_service.rs 中报价状态字符串字面量统一引用此模块（规则 0）
pub mod quotation {
    /// 草稿：报价单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 已审批：审批通过
    pub const APPROVED: &str = "approved";

    /// 已拒绝：审批未通过
    pub const REJECTED: &str = "rejected";

    /// 已取消：报价单作废
    pub const CANCELLED: &str = "cancelled";
}

/// 定制订单状态（custom_order.status，小写值）
/// 批次 234 v13 真实接入：custom_order_crud_service.rs 中订单状态字符串字面量统一引用此模块（规则 0）
pub mod custom_order {
    /// 草稿：订单初始状态，可编辑
    pub const DRAFT: &str = "draft";

    /// 待处理：等待排产
    pub const PENDING: &str = "pending";

    /// 已完成：订单流程完结
    pub const COMPLETED: &str = "completed";

    /// 已取消：订单作废
    pub const CANCELLED: &str = "cancelled";
}

/// 定制订单流程节点状态（process_node.status，小写值）
/// 批次 234 v13 真实接入：custom_order_state_service.rs 中节点状态字符串字面量统一引用此模块（规则 0）
pub mod process_node {
    /// 进行中：节点正在执行
    pub const IN_PROGRESS: &str = "in_progress";

    /// 已完成：节点执行完毕
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

/// 财务发票状态（finance_invoice.status，小写值）
/// 批次 234 v13 真实接入：finance_invoice_service.rs 中发票状态字符串字面量统一引用此模块（规则 0）
pub mod finance_invoice {
    /// 待处理：发票初始状态，可审批
    pub const PENDING: &str = "pending";

    /// 已审批：审批通过
    pub const APPROVED: &str = "approved";
}

/// 财务付款状态（finance_payment.status，小写值）
/// 批次 234 v13 真实接入：finance_payment_service.rs 中付款状态字符串字面量统一引用此模块（规则 0）
pub mod finance_payment {
    /// 待处理：付款单初始状态
    pub const PENDING: &str = "pending";
}

/// BPM 流程实例状态（bpm_process_instance.status，大写值）
/// 批次 235 v13 真实接入：bpm_service.rs 中流程实例状态字符串字面量统一引用此模块（规则 0）
pub mod bpm_instance {
    /// 处理中：流程实例运行中
    pub const PROCESSING: &str = "PROCESSING";

    /// 已完成：流程实例正常结束
    pub const COMPLETED: &str = "COMPLETED";

    /// 已终止：流程实例被异常终止
    pub const TERMINATED: &str = "TERMINATED";

    /// 已取消：流程实例被取消
    pub const CANCELLED: &str = "CANCELLED";
}

/// BPM 任务状态（bpm_task.status，小写值）
/// 批次 235 v13 真实接入：bpm_service.rs 中任务状态字符串字面量统一引用此模块（规则 0）
pub mod bpm_task {
    /// 待处理：任务待办理
    pub const PENDING: &str = "pending";

    /// 已完成：任务已办理完成
    pub const COMPLETED: &str = "completed";

    /// 已拒绝：任务被拒绝
    pub const REJECTED: &str = "rejected";

    /// 已取消：任务被取消
    pub const CANCELLED: &str = "cancelled";
}

/// MRP 结果状态（mrp_result.status，大写值）
/// 批次 235 v13 真实接入：mrp_engine_service.rs 中 MRP 结果状态字符串字面量统一引用此模块（规则 0）
pub mod mrp {
    /// 已计划：MRP 计划生成，待发布
    pub const PLANNED: &str = "PLANNED";

    /// 已发布：MRP 计划已发布为生产订单
    pub const RELEASED: &str = "RELEASED";

    /// 已确认：MRP 计划已确认（采购订单类型）
    pub const CONFIRMED: &str = "CONFIRMED";

    /// 已取消：MRP 计划已取消
    pub const CANCELLED: &str = "CANCELLED";
}

/// 导入任务状态（import_task.status，小写值）
/// 批次 235 v13 真实接入：import_export_service.rs 中导入任务状态字符串字面量统一引用此模块（规则 0）
pub mod import_task {
    /// 运行中：导入任务正在执行
    pub const RUNNING: &str = "running";

    /// 成功：导入任务全部成功
    pub const SUCCESS: &str = "success";

    /// 失败：导入任务全部失败
    pub const FAILED: &str = "failed";

    /// 部分成功：导入任务部分成功部分失败
    pub const PARTIAL: &str = "partial";
}

/// CRM 线索状态（crm_lead.lead_status，小写值）
/// 批次 236 v13 真实接入：crm/assign.rs、crm/lead.rs、crm/pool.rs 等线索状态字符串字面量统一引用此模块（规则 0）
pub mod crm_lead {
    /// 新线索：刚创建的线索
    pub const NEW: &str = "new";

    /// 已转化：线索已转化为商机
    pub const CONVERTED: &str = "converted";

    /// 客户池：线索已进入客户池
    pub const POOL: &str = "pool";

    /// 已流失：线索已丢失
    pub const LOST: &str = "lost";
}

/// CRM 商机状态（opportunity.status，大写值）
/// 批次 236 v13 真实接入：crm/opp.rs 中商机状态字符串字面量统一引用此模块（规则 0）
pub mod crm_opportunity {
    /// 赢单：商机已赢
    pub const CLOSED_WON: &str = "CLOSED_WON";

    /// 输单：商机已输
    pub const CLOSED_LOST: &str = "CLOSED_LOST";
}

/// 库存裁片状态（inventory_piece.status，大写值）
/// 批次 236 v13 真实接入：barcode_scanner_handler.rs、piece_split_handler.rs 中裁片状态字符串字面量统一引用此模块（规则 0）
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
}

/// 登录日志状态（log_login.status，大写值）
/// 批次 236 v13 真实接入：login_security_handler.rs、auth_handler.rs 中登录日志状态字符串字面量统一引用此模块（规则 0）
pub mod login_log {
    /// 成功：登录成功
    pub const SUCCESS: &str = "SUCCESS";

    /// 失败：登录失败
    pub const FAILED: &str = "FAILED";
}

/// 邮件日志状态（email_log.status，大写值）
/// 批次 236 v13 真实接入：email_log_service.rs 中邮件日志状态字符串字面量统一引用此模块（规则 0）
pub mod email_log {
    /// 待发送：邮件待发送
    pub const PENDING: &str = "PENDING";

    /// 已发送：邮件已发送
    pub const SENT: &str = "SENT";

    /// 发送失败：邮件发送失败
    pub const FAILED: &str = "FAILED";
}

/// 启用/停用状态（通用，大写值 ACTIVE/INACTIVE）
/// 批次 236 v13 真实接入：email_template/report_subscription/report_template/bom_process_definition 等启用停用状态
/// 注意：与 master_data 模块（小写 active/inactive）区分，本模块用于大写 ACTIVE/INACTIVE 场景
pub mod active_status {
    /// 启用
    pub const ACTIVE: &str = "ACTIVE";

    /// 停用
    pub const INACTIVE: &str = "INACTIVE";
}

/// 工作中心状态（work_center.status，大写值，复用 active_status）
/// 产能负载项状态（capacity_load_item.status，大写值）
/// 批次 236 v13 真实接入：capacity_service.rs、scheduling_auto.rs 等
pub mod work_center {
    /// 空闲：产能负载项空闲
    pub const LOAD_IDLE: &str = "IDLE";

    /// 过载：产能负载项过载
    pub const LOAD_OVERLOADED: &str = "OVERLOADED";
}

/// 合同状态（sales_contract.status / purchase_contract.status，小写值）
/// 批次 236 v13 真实接入：sales_contract_service.rs、purchase_contract_service.rs 等
pub mod contract_status {
    /// 草稿：合同初始状态
    pub const DRAFT: &str = "draft";

    /// 已取消：合同已取消
    pub const CANCELLED: &str = "cancelled";
}

/// 报价单状态（sales_quotation.status，小写值，补充批次 234 的 quotation 模块）
/// 批次 236 v13 真实接入：quotation_approval_service.rs、quotation_convert_service.rs 等
/// 注意：quotation 模块（批次 234）已定义 DRAFT/APPROVED/REJECTED/CANCELLED，
/// 本模块补充审批/转换流程专属状态
pub mod quotation_ext {
    /// 待审批：报价单已提交审批
    pub const PENDING_APPROVAL: &str = "pending_approval";

    /// 已过期：报价单已过期
    pub const EXPIRED: &str = "expired";

    /// 已转化：报价单已转化为销售订单
    pub const CONVERTED: &str = "converted";
}

/// 价格审批状态（sales_price.status / purchase_price.status，小写值）
/// 批次 236 v13 真实接入：sales_price_service.rs、purchase_price_service.rs
pub mod price_approval {
    /// 待审批：价格待审批
    pub const PENDING: &str = "pending";

    /// 已审批：价格已审批
    pub const APPROVED: &str = "approved";
}

/// 质量标准状态（quality_standard.status，小写值）
/// 批次 236 v13 真实接入：quality_standard_service.rs
pub mod quality_standard {
    /// 草稿：质量标准初始状态
    pub const DRAFT: &str = "draft";

    /// 已审批：质量标准已审批
    pub const APPROVED: &str = "approved";

    /// 已拒绝：质量标准被拒绝
    pub const REJECTED: &str = "rejected";
}

/// 质量检验处理状态（quality_inspection_unqualified.handling_status，小写值）
/// 批次 236 v13 真实接入：quality_inspection_service.rs
pub mod quality_handling {
    /// 待处理：不合格品待处理
    pub const PENDING: &str = "pending";
}

/// 染色配方状态（dye_recipe.status，中文值）
/// v14 批次 423A 常量化：原 handler 硬编码中文字符串，现统一常量
/// 依据：面料行业真实业务调研文档 §11.1 化验室打样流程
pub mod dye_recipe {
    /// 草稿：配方初始状态
    pub const DRAFT: &str = "草稿";

    /// 已审核：配方已审核通过
    pub const APPROVED: &str = "已审核";

    /// 已停用：配方已停用
    pub const DISABLED: &str = "已停用";
}

/// 化验室打样通知单状态（lab_dip_request.status，小写值）
/// v14 批次 423B 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环
/// 状态机：pending → sampling → submitted → approved/rejected → completed
pub mod lab_dip_request {
    /// 待打样：通知单已下达，待技术科安排打样
    pub const PENDING: &str = "pending";

    /// 打样中：技术科正在打 ABCD 多版小样
    pub const SAMPLING: &str = "sampling";

    /// 已送客户：小样已送客户等待确认
    pub const SUBMITTED: &str = "submitted";

    /// OK 样确认：客户已选中 OK 样
    pub const APPROVED: &str = "approved";

    /// 重打：客户要求重打
    pub const REJECTED: &str = "rejected";

    /// 已建库：复样通过，处方已升级为大货模板入库
    pub const COMPLETED: &str = "completed";
}

/// 打样小样对色结果（lab_dip_sample.matching_result，小写值）
/// v14 批次 423B 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.1 对色规范（0/45 度观察，色差 4-5 级为 OK）
pub mod lab_dip_sample {
    /// 待对色：小样刚打出，尚未对色
    pub const PENDING: &str = "pending";

    /// 对色 OK：色差达 4-5 级
    pub const MATCHED: &str = "matched";

    /// 不匹配：色差低于 4 级，需重打
    pub const NOT_MATCHED: &str = "not_matched";

    /// 客户选中 OK 样：客户从多版中选中此版作为 OK 样
    pub const SELECTED: &str = "selected";
}

/// 复样结果状态（lab_dip_resample.result，小写值）
/// v14 批次 423B 真实业务常量化
/// 依据：面料行业真实业务调研文档 §11.1 复样（大货前验证，色差 4-5 级方可投产）
pub mod lab_dip_resample {
    /// 待复样：复样任务已创建，待执行
    pub const PENDING: &str = "pending";

    /// 复样通过：色差达 4-5 级，可投产
    pub const PASSED: &str = "passed";

    /// 复样失败：色差低于 4 级，不可投产
    pub const FAILED: &str = "failed";

    /// 需调整重试：处方需加成/冲减调整后重新复样
    pub const ADJUSTED: &str = "adjusted";
}

/// 应付核销状态（ap_verification.verification_status，大写值）
/// 批次 236 v13 真实接入：ap_verification_service.rs
pub mod ap_verification {
    /// 已完成：核销完成
    pub const COMPLETED: &str = "COMPLETED";

    /// 已取消：核销已取消
    pub const CANCELLED: &str = "CANCELLED";
}

/// 固定资产状态（fixed_asset.status / fixed_asset_depreciation.status）
/// 批次 236 v13 真实接入：fixed_asset_service.rs
pub mod fixed_asset {
    /// 已处置：固定资产已处置（小写）
    pub const DISPOSED: &str = "disposed";

    /// 已完成：折旧已完成（大写）
    pub const DEPRECIATION_COMPLETED: &str = "COMPLETED";
}

/// 审计消息状态（omni_audit_message.status，大写值）
/// 批次 236 v13 真实接入：omni_audit_service.rs
pub mod audit_message {
    /// 失败：审计消息失败
    pub const FAILED: &str = "FAILED";

    /// 拒绝：审计消息被拒绝
    pub const DENIED: &str = "DENIED";
}

/// 色卡状态（color_card.status，小写值）
/// 批次 236 v13 真实接入：color_card_crud_service.rs
pub mod color_card {
    /// 已归档：色卡已归档
    pub const ARCHIVED: &str = "archived";

    /// 已丢失：色卡已丢失
    pub const LOST: &str = "lost";
}

/// 成本归集状态（cost_collection.status，小写值）
/// 批次 236 v13 真实接入：cost_collection_service.rs
pub mod cost_collection {
    /// 草稿：成本归集初始状态
    pub const DRAFT: &str = "draft";
}

/// 定制订单售后/质量/工序状态（小写值）
/// 批次 236 v13 真实接入：custom_order_quality_service.rs、custom_order_aftersales_service.rs、custom_order_process_service.rs
pub mod custom_order_ext {
    /// 质量问题-开启
    pub const QUALITY_OPEN: &str = "open";

    /// 质量问题-关闭
    pub const QUALITY_CLOSED: &str = "closed";

    /// 质量问题-已解决
    pub const QUALITY_RESOLVED: &str = "resolved";

    /// 售后-已开启
    pub const AFTERSALES_OPENED: &str = "opened";

    /// 售后-已拒绝
    pub const AFTERSALES_REJECTED: &str = "rejected";

    /// 工序-待处理
    pub const PROCESS_PENDING: &str = "pending";
}

/// 采购收货检验状态（purchase_receipt.inspection_status，大写值）
/// 批次 236 v13 真实接入：purchase_receipt_service.rs
pub mod purchase_receipt_inspection {
    /// 待检验
    pub const PENDING: &str = "PENDING";
}

/// 会计期间补充状态（accounting_period.status，大写值，补充批次 232 的 OPEN/CLOSED）
/// 批次 236 v13 真实接入：missing_handlers.rs
pub mod accounting_period_closing {
    /// 关账中：会计期间正在关账
    pub const CLOSING: &str = "CLOSING";
}

/// 健康检查状态（health_check.status，小写值）
/// 批次 236 v13 真实接入：health_handler.rs
pub mod health_check {
    /// 健康
    pub const HEALTHY: &str = "healthy";

    /// 不健康
    pub const UNHEALTHY: &str = "unhealthy";

    /// 降级
    pub const DEGRADED: &str = "degraded";
}

/// 销售面料订单状态（sales_fabric_order.status，小写值）
/// 批次 236 v13 真实接入：sales_fabric_order_handler.rs
pub mod sales_fabric_order {
    /// 待处理
    pub const PENDING: &str = "pending";

    /// 已审批
    pub const APPROVED: &str = "approved";
}

/// 对账结果状态（reconcile_result.status，大写值）
/// 批次 236 v13 真实接入：ap_reconciliation_handler.rs
pub mod reconcile_result {
    /// 失败
    pub const FAILED: &str = "FAILED";
}

/// 故障切换状态（failover current_state，小写值）
/// 批次 236 v13 真实接入：failover_service.rs
pub mod failover {
    /// 主节点
    pub const PRIMARY: &str = "primary";

    /// 备节点
    pub const BACKUP: &str = "backup";
}

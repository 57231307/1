#![allow(dead_code)]
//! 工资/能耗/染化料/委外/业务模式状态常量分组
//!
//! 批次 490 D10-3b 拆分：从 models/status.rs 抽取的工资/能耗/染化料/委外/业务模式状态常量子模块组。
//! 包含：wage_rate_status/wage_type/wage_record_status/energy_type/energy_meter_status/energy_recording_method/energy_allocation_basis/energy_record_status/energy_rule_status/color_card/chemical_type/chemical_status/chemical_inspection_status/chemical_lot_status/chemical_requisition_type/chemical_requisition_status/outsourcing_order_type/outsourcing_order_status/outsourcing_loss_type/outsourcing_receipt_status/outsourcing_voucher_type/business_mode_code/business_material_source/business_settlement_method/business_rule_type

/// 工序工价状态（process_wage_rate.status，v14 批次 427，状态机 draft→active→disabled）
pub mod wage_rate_status {
    /// 草稿：新建工价方案，未启用
    pub const DRAFT: &str = "draft";

    /// 启用：工价生效，可用于工资计算
    pub const ACTIVE: &str = "active";

    /// 停用：工价失效，不再用于工资计算
    pub const DISABLED: &str = "disabled";
}

/// 工价类型（process_wage_rate.wage_type，v14 批次 427，计件/计时/混合）
pub mod wage_type {
    /// 计件：按产量计酬，wage = qualified_quantity × piece_price × grade_ratio
    pub const PIECE: &str = "piece";

    /// 计时：按工时计酬，wage = duration_minutes × time_price × grade_ratio
    pub const TIME: &str = "time";

    /// 混合：计件 + 计时，wage = piece_wage + time_wage
    pub const MIXED: &str = "mixed";
}

/// 工资记录状态（wage_record.status，v14 批次 427，状态机 draft→confirmed→paid→cancelled）
pub mod wage_record_status {
    /// 草稿：工资计算生成的初始状态，可重新计算或删除
    pub const DRAFT: &str = "draft";

    /// 已确认：审核通过，等待发放
    pub const CONFIRMED: &str = "confirmed";

    /// 已发放：工资已发放到工人
    pub const PAID: &str = "paid";

    /// 已取消：作废，不再发放
    pub const CANCELLED: &str = "cancelled";
}

/// 能源类型（energy_meter.meter_type 等，v14 批次 428，水/电/汽/天然气/压缩空气）
pub mod energy_type {
    /// 水：吨
    pub const WATER: &str = "water";

    /// 电：度（千瓦时）
    pub const ELECTRICITY: &str = "electricity";

    /// 蒸汽：立方米
    pub const STEAM: &str = "steam";

    /// 天然气：立方米
    pub const GAS: &str = "gas";

    /// 压缩空气：立方米
    pub const COMPRESSED_AIR: &str = "compressed_air";
}

/// 能耗计量设备状态（energy_meter.status，v14 批次 428，状态机 active→inactive/maintenance）
pub mod energy_meter_status {
    /// 启用：计量设备正常工作，可采集数据
    pub const ACTIVE: &str = "active";

    /// 停用：计量设备已停用
    pub const INACTIVE: &str = "inactive";

    /// 维护中：计量设备正在维护
    pub const MAINTENANCE: &str = "maintenance";
}

/// 能耗录入方式（energy_consumption_record.recording_method，v14 批次 428，手工/IoT/自动计算）
pub mod energy_recording_method {
    /// 手工：人工抄表录入
    pub const MANUAL: &str = "manual";

    /// IoT 自动：IoT 设备自动采集
    pub const IOT: &str = "iot";

    /// 自动计算：系统根据分摊规则自动计算
    pub const AUTO_CALC: &str = "auto_calc";
}

/// 能耗分摊基准（energy_allocation_rule/record.allocation_basis，v14 批次 428，按工时/产量/设备/车间分摊）
pub mod energy_allocation_basis {
    /// 按工时：按工序记录的工时分摊（duration_minutes）
    pub const BY_DURATION: &str = "by_duration";

    /// 按产量：按工序记录的产量分摊（qualified_quantity）
    pub const BY_OUTPUT: &str = "by_output";

    /// 按设备：按设备运行时长分摊
    pub const BY_EQUIPMENT: &str = "by_equipment";

    /// 按车间：按车间总产量平均分摊
    pub const BY_WORKSHOP: &str = "by_workshop";
}

/// 能耗记录状态（energy_consumption_record/allocation_record.status，v14 批次 428，状态机 draft→confirmed→cancelled）
pub mod energy_record_status {
    /// 草稿：新建记录，可编辑
    pub const DRAFT: &str = "draft";

    /// 已确认：已审核，可参与月末分摊
    pub const CONFIRMED: &str = "confirmed";

    /// 已取消：作废
    pub const CANCELLED: &str = "cancelled";
}

/// 能耗分摊规则状态（energy_allocation_rule.status，v14 批次 428，状态机 draft→active→disabled）
pub mod energy_rule_status {
    /// 草稿：新建规则，未启用
    pub const DRAFT: &str = "draft";

    /// 启用：规则生效，可用于分摊计算
    pub const ACTIVE: &str = "active";

    /// 停用：规则失效，不再用于分摊计算
    pub const DISABLED: &str = "disabled";
}

/// 色卡状态（color_card.status，小写值）
/// 批次 236 v13 真实接入：color_card_crud_service.rs
pub mod color_card {
    /// 已归档：色卡已归档
    pub const ARCHIVED: &str = "archived";

    /// 已丢失：色卡已丢失
    pub const LOST: &str = "lost";
}

/// 染化料类型（chemical_master.chemical_type 等，v14 批次 429，染料/助剂/化工原料）
pub mod chemical_type {
    /// 染料：分散/活性/还原/硫化/酸性/直接/阳离子
    pub const DYE: &str = "dye";

    /// 助剂：前处理/染色/后整理/印花
    pub const AUXILIARY: &str = "auxiliary";

    /// 化工原料
    pub const CHEMICAL: &str = "chemical";
}

/// 染化料主数据状态（chemical_master.status，v14 批次 429，状态机 active→inactive/discontinued）
pub mod chemical_status {
    /// 启用：染化料可用
    pub const ACTIVE: &str = "active";

    /// 停用：染化料临时停用
    pub const INACTIVE: &str = "inactive";

    /// 停产：染化料已停产，仅允许出库不允许入库
    pub const DISCONTINUED: &str = "discontinued";
}

/// 染化料批次来料检验状态（chemical_lot.inspection_status，v14 批次 429，状态机 pending→passed/failed/quarantine）
pub mod chemical_inspection_status {
    /// 待检：新到货批次，等待来料检验
    pub const PENDING: &str = "pending";

    /// 合格：检验通过，可领用
    pub const PASSED: &str = "passed";

    /// 不合格：检验不通过，需退货或报废
    pub const FAILED: &str = "failed";

    /// 隔离：存疑批次，暂时隔离待复检
    pub const QUARANTINE: &str = "quarantine";
}

/// 染化料批次状态（chemical_lot.status，v14 批次 429，状态机 active→consumed/expired/scrapped）
pub mod chemical_lot_status {
    /// 可用：批次可用库存大于 0 且未过期
    pub const ACTIVE: &str = "active";

    /// 已耗尽：可用库存为 0
    pub const CONSUMED: &str = "consumed";

    /// 已过期：超过失效日期
    pub const EXPIRED: &str = "expired";

    /// 已报废：因检验不合格或损坏而报废
    pub const SCRAPPED: &str = "scrapped";
}

/// 染化料领用单类型（chemical_requisition.requisition_type，v14 批次 429，生产/化验室/研发）
pub mod chemical_requisition_type {
    /// 生产领用：从车间仓库领用至染色缸号
    pub const PRODUCTION: &str = "production";

    /// 化验室领用：化验室打样测试用
    pub const LAB: &str = "lab";

    /// 研发领用：研发新工艺测试用
    pub const RD: &str = "rd";
}

/// 染化料领用单状态（chemical_requisition.status，v14 批次 429，状态机 draft→approved→issued→partial_returned→closed，任意非 closed→cancelled）
pub mod chemical_requisition_status {
    /// 草稿：新建领用单，可编辑
    pub const DRAFT: &str = "draft";

    /// 已审批：审批通过，待发料
    pub const APPROVED: &str = "approved";

    /// 已发料：仓库已发料，可部分退回
    pub const ISSUED: &str = "issued";

    /// 部分退回：发料后部分退回，等待全部退回或结案
    pub const PARTIAL_RETURNED: &str = "partial_returned";

    /// 已关闭：全部退回或正常结案
    pub const CLOSED: &str = "closed";

    /// 已取消：任意非 closed 状态可取消
    pub const CANCELLED: &str = "cancelled";
}

/// 委外加工类型（outsourcing_order.order_type，v14 批次 430，染色/印花/织布/后整理/其他）
pub mod outsourcing_order_type {
    /// 染色
    pub const DYEING: &str = "dyeing";
    /// 印花
    pub const PRINTING: &str = "printing";
    /// 织布
    pub const WEAVING: &str = "weaving";
    /// 后整理
    pub const FINISHING: &str = "finishing";
    /// 其他
    pub const OTHER: &str = "other";
}

/// 委外加工订单状态（outsourcing_order.status，v14 批次 430，状态机 draft→issued→processing→received→settled→closed，任意非 closed→cancelled）
pub mod outsourcing_order_status {
    /// 草稿：新建委外订单，可编辑
    pub const DRAFT: &str = "draft";
    /// 已发料：发出材料给外协厂，已生成发料凭证
    pub const ISSUED: &str = "issued";
    /// 加工中：外协厂正在加工
    pub const PROCESSING: &str = "processing";
    /// 已收回：成品已收回入库，已生成入库凭证
    pub const RECEIVED: &str = "received";
    /// 已结算：加工费已结算，已生成加工费凭证
    pub const SETTLED: &str = "settled";
    /// 已关闭：业务流程完结归档
    pub const CLOSED: &str = "closed";
    /// 已取消：任意非 closed 状态可取消
    pub const CANCELLED: &str = "cancelled";
}

/// 委外损耗类型（outsourcing_order/receipt.loss_type，v14 批次 430，正常损耗摊成本/非正常损耗计入营业外支出）
pub mod outsourcing_loss_type {
    /// 正常损耗：摊入成本（按实际收回数量结转）
    pub const NORMAL: &str = "normal";
    /// 非正常损耗：计入营业外支出（超定额损耗，单独追责）
    pub const ABNORMAL: &str = "abnormal";
}

/// 委外收回入库状态（outsourcing_receipt.status，v14 批次 430，状态机 draft→confirmed→cancelled）
pub mod outsourcing_receipt_status {
    /// 草稿：新建收回单，可编辑
    pub const DRAFT: &str = "draft";
    /// 已确认：损耗分类与单位成本已计算
    pub const CONFIRMED: &str = "confirmed";
    /// 已取消：作废
    pub const CANCELLED: &str = "cancelled";
}

/// 委外加工凭证类型（outsourcing_voucher.voucher_type，v14 批次 430，发料/加工费/入库/损耗处理）
pub mod outsourcing_voucher_type {
    /// 发料凭证：借 委托加工物资 / 贷 自制半成品-胚布
    pub const ISSUE: &str = "issue";
    /// 加工费凭证：借 委托加工物资+应交税费-进项税额 / 贷 银行存款
    pub const FEE: &str = "fee";
    /// 入库凭证：借 库存商品-成品布 / 贷 委托加工物资
    pub const RECEIPT: &str = "receipt";
    /// 损耗处理凭证：借 营业外支出 / 贷 委托加工物资（非正常损耗单独追责）
    pub const LOSS: &str = "loss";
}

/// v14 批次 431：业务模式代码（business_mode_config.mode_code，6 种模式）
pub mod business_mode_code {
    /// 坯布经销模式：采购坯布 → 库存 → 销售坯布
    pub const GREY_TRADING: &str = "grey_trading";
    /// 成品经销模式：采购坯布 → 染整加工 → 销售成品
    pub const FINISHED_TRADING: &str = "finished_trading";
    /// 染整加工模式（客供坯布）：客户提供坯布 → 染整加工 → 收取加工费
    pub const DYEING_PROCESSING: &str = "dyeing_processing";
    /// 自织自染模式：采购原料 → 纺纱 → 织布 → 染整 → 销售成品
    pub const SELF_WEAVE_DYE: &str = "self_weave_dye";
    /// 委托加工模式：自制半成品 → 委外加工 → 收回成品 → 销售
    pub const OUTSOURCING: &str = "outsourcing";
    /// 来料加工模式：客户来料 → 加工 → 收取加工费
    pub const TOLL_PROCESSING: &str = "toll_processing";
}

/// v14 批次 431：物料来源（business_mode_config.material_source）
pub mod business_material_source {
    /// 采购：从供应商采购物料
    pub const PURCHASE: &str = "purchase";
    /// 客供：客户提供物料
    pub const CUSTOMER_PROVIDED: &str = "customer_provided";
    /// 自制：内部生产物料
    pub const SELF_MADE: &str = "self_made";
    /// 来料：客户来料加工
    pub const TOLL: &str = "toll";
}

/// v14 批次 431：结算方式（business_mode_config.settlement_method）
pub mod business_settlement_method {
    /// 销售结算：按销售价格结算
    pub const SALE_SETTLEMENT: &str = "sale_settlement";
    /// 加工费结算：按加工费结算
    pub const PROCESSING_FEE_SETTLEMENT: &str = "processing_fee_settlement";
}

/// v14 批次 431：业务模式规则类型（business_mode_rule.rule_type）
pub mod business_rule_type {
    /// 必需：该模块必须存在
    pub const REQUIRED: &str = "required";
    /// 可选：该模块可选存在
    pub const OPTIONAL: &str = "optional";
    /// 禁止：该模块禁止存在
    pub const FORBIDDEN: &str = "forbidden";
}

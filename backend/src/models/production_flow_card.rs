#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 生产流转卡模型（production_flow_card 表）
//!
//! v14 批次 425：流转卡条码与车间工序流转
//! 依据：面料行业真实业务调研文档 §12.1 流转卡条码管理 + §12.7 缸号状态机
//! 真实业务：一缸一卡，卡随缸走，扫码即获取全部信息
//! 状态机：pending(待排缸) → scheduled(已排缸) → preparing(备布中) → dyeing(染色中) → dyed(已出缸) → inspecting(验布中) → completed(已完成) → shipped(已发货) / terminated(已终止)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 生产流转卡模型
///
/// 真实业务承载信息：
/// - 缸号（唯一标识）
/// - 订单信息（客户/订单号/产品/色号）
/// - 染整要求与注意事项
/// - 工序路线
/// - 计划配布数量
/// - 条码（一维码/二维码）
///
/// 扫码应用场景：
/// - 白坯仓库：扫描缸号条码 → 自动出库
/// - 染色车间：扫描缸卡条码 → 输入生产进度
/// - 称料室：扫描流转卡条码 → 加载大货处方 → 称料
/// - 车间流转：扫描流转卡条码 → 登记工人 → 自动跟进工序和产量
/// - 成品入库：PDA 扫描卷唛条码 → 自动入库
/// - 发货：输入或扫描缸号 → 获取缸号所有信息 → 发货
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "production_flow_card")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 流转卡号：FC-YYYYMMDDHHMMSS-NNN
    pub card_no: String,
    /// 流转卡条码（一维码/二维码内容，全局唯一）
    pub barcode: String,

    /// 关联生产订单 ID
    pub production_order_id: i32,
    /// 关联缸号（dye_batch.id）
    pub dye_batch_id: Option<i32>,
    /// 缸号字符串（冗余便于扫码查询，dye_batch.batch_no）
    pub dye_lot_no: Option<String>,
    /// 关联工序路线 ID
    pub process_route_id: Option<i32>,

    // ===== 客户/订单/产品信息（冗余自订单，便于流转卡直接展示） =====
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    /// 色号
    pub color_no: Option<String>,
    /// 染整要求与注意事项
    pub dyeing_requirements: Option<String>,

    // ===== 配布数量 =====
    /// 计划配布数量（kg）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub planned_fabric_weight: Option<Decimal>,
    /// 实际配布数量（kg，备布后回填）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub actual_fabric_weight: Option<Decimal>,

    // ===== 工序进度 =====
    /// 当前工序序号
    pub current_step_seq: i32,
    /// 流转卡状态：pending/scheduled/preparing/dyeing/dyed/inspecting/completed/shipped/terminated
    pub status: String,

    // ===== 关键时间节点（缸号状态机） =====
    /// 排缸时间
    pub scheduled_at: Option<DateTimeWithTimeZone>,
    /// 备布完成时间
    pub prepared_at: Option<DateTimeWithTimeZone>,
    /// 进缸时间
    pub dye_start_at: Option<DateTimeWithTimeZone>,
    /// 出缸时间
    pub dye_end_at: Option<DateTimeWithTimeZone>,
    /// 验布时间
    pub inspected_at: Option<DateTimeWithTimeZone>,
    /// 完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,
    /// 发货时间
    pub shipped_at: Option<DateTimeWithTimeZone>,

    /// 优先级（数字越大越优先）
    pub priority: i32,
    /// 备注
    pub remarks: Option<String>,

    // 软删除与审计
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联生产订单
    #[sea_orm(
        belongs_to = "super::production_order::Entity",
        from = "Column::ProductionOrderId",
        to = "super::production_order::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    ProductionOrder,

    /// 关联缸号
    #[sea_orm(
        belongs_to = "super::dye_batch::Entity",
        from = "Column::DyeBatchId",
        to = "super::dye_batch::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    DyeBatch,

    /// 关联工序路线
    #[sea_orm(
        belongs_to = "super::process_route::Entity",
        from = "Column::ProcessRouteId",
        to = "super::process_route::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProcessRoute,

    /// 关联客户
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Customer,

    /// 关联产品
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Product,

    /// 一对多：流转卡的工序流转记录
    #[sea_orm(has_many = "super::process_step_record::Entity")]
    StepRecords,

    /// 一对多：流转卡的质量反馈单
    #[sea_orm(has_many = "super::process_quality_feedback::Entity")]
    QualityFeedbacks,
}

impl Related<super::production_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductionOrder.def()
    }
}

impl Related<super::dye_batch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeBatch.def()
    }
}

impl Related<super::process_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessRoute.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::process_step_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StepRecords.def()
    }
}

impl Related<super::process_quality_feedback::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QualityFeedbacks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

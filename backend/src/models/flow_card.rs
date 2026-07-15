#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 流转卡模型（flow_card 表）
//!
//! v14 批次 425：流转卡工序流转模块
//! 依据：面料行业真实业务调研文档 §14.1 流转卡工序流转（基于同凯印染 ERP/KESHTECH 真实开卡字段）
//! 真实业务：流转卡=生产流程卡/工序流转卡/缸卡，一卡对应一缸布的生产任务，
//!          承载从开卡到成品入库的全部工序信息。流转卡=缸号+工单信息+工序路线+计划配布数量+条码。
//! 核心能力：扫码签入签出 + 工序状态机 + 分卡/合卡/拆卡 + 内修卡

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

/// 工序路线项（process_route JSON 数组元素）
///
/// 真实业务字段说明：
/// - sequence: 工序序号（1, 2, 3...）
/// - name: 工序名称（备布/染色/烘干/定型/验布/入库等）
/// - status: 工序状态（pending/in_progress/completed/transferred/stored/paused/rework）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ProcessRouteItem {
    /// 工序序号
    pub sequence: i32,
    /// 工序名称
    pub name: String,
    /// 工序状态
    pub status: String,
}

/// 流转卡模型
///
/// 真实业务要点：
/// - 同一缸号(dye_lot_no)只能有一张主卡（一缸一卡约束，Service 层校验）
/// - 内修卡号 = 原始卡号 + A/B/C 后缀（一次回修+A，二次回修+B）
/// - 分卡生成新卡号，原卡状态变更；合缸多卡共享缸号但保留各自卡号
/// - 拆卡生成子卡号关联母卡号(parent_card_id)
/// - 工序路线(process_route)记录该卡的全部工序及状态
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "flow_card")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 卡号：FC-YYYYMMDDHHMMSS-NNN（唯一）
    pub flow_card_no: String,

    // ===== 标识信息 =====
    /// 条码（Code128 格式字符串，扫码用）
    pub barcode: Option<String>,
    /// 缸号（一缸布的标识，合缸时多卡共享）
    pub dye_lot_no: Option<String>,

    // ===== 订单信息 =====
    /// 关联工单 ID
    pub work_order_id: Option<i32>,
    /// 关联生产订单 ID
    pub production_order_id: Option<i32>,
    /// 客户 ID
    pub customer_id: Option<i32>,
    /// 业务员 ID
    pub salesman_id: Option<i32>,

    // ===== 产品信息 =====
    /// 坯布 ID
    pub greige_fabric_id: Option<i32>,
    /// 布种
    pub fabric_type: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 成分
    pub composition: Option<String>,
    /// 克重 g/m²
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub gram_weight: Option<Decimal>,
    /// 门幅 cm
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub fabric_width: Option<Decimal>,

    // ===== 颜色信息 =====
    /// 色号
    pub color_no: Option<String>,
    /// 色名
    pub color_name: Option<String>,
    /// 对色光源（D65/TL84/U3000/CWF/A 等）
    pub light_source: Option<String>,

    // ===== 计划信息 =====
    /// 开卡匹数
    pub planned_pieces: Option<i32>,
    /// 计划总重 kg
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub planned_weight_kg: Option<Decimal>,
    /// 配布数量
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub planned_quantity: Option<Decimal>,
    /// 实际匹数
    pub actual_pieces: Option<i32>,
    /// 实际总重
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub actual_weight_kg: Option<Decimal>,

    // ===== 工艺与工序 =====
    /// 工序路线 JSON：[{sequence, name, status}]
    pub process_route: Option<Vec<ProcessRouteItem>>,
    /// 当前工序
    pub current_process: Option<String>,

    // ===== 交期与仓位 =====
    /// 交货期
    pub delivery_date: Option<NaiveDate>,
    /// 仓位
    pub warehouse_position: Option<String>,

    // ===== 状态机 =====
    /// 卡状态：opened → waiting_dyeing → scheduled → preparing → dyeing → dyed
    ///       → inspecting → stored → shipped；分支：paused / rework / terminated / cancelled
    pub status: String,

    // ===== 回修与拆卡关联 =====
    /// 原始卡号（回修卡关联）
    pub original_card_id: Option<i32>,
    /// 回修次数
    pub rework_count: Option<i32>,
    /// 母卡号（拆卡关联）
    pub parent_card_id: Option<i32>,
    /// 是否回修卡
    pub is_rework: bool,

    // ===== 软删除与审计 =====
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联工单/生产订单
    #[sea_orm(
        belongs_to = "super::production_order::Entity",
        from = "Column::WorkOrderId",
        to = "super::production_order::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    WorkOrder,

    /// 关联生产订单
    #[sea_orm(
        belongs_to = "super::production_order::Entity",
        from = "Column::ProductionOrderId",
        to = "super::production_order::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProductionOrder,

    /// 关联客户
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Customer,

    /// 关联坯布
    #[sea_orm(
        belongs_to = "super::greige_fabric::Entity",
        from = "Column::GreigeFabricId",
        to = "super::greige_fabric::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    GreigeFabric,

    /// 关联原始卡（回修卡关联原卡）
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::OriginalCardId",
        to = "Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    OriginalCard,

    /// 关联母卡（拆卡关联母卡）
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentCardId",
        to = "Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ParentCard,

    /// 一对多：流转卡下的工序操作记录
    #[sea_orm(has_many = "super::flow_card_operation::Entity")]
    Operations,
}

impl Related<super::production_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrder.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::greige_fabric::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GreigeFabric.def()
    }
}

impl Related<super::flow_card_operation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Operations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

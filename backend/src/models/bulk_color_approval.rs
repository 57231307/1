//! 大货批色审批 Model（V15 P0-F15 创建）
//!
//! 表 bulk_color_approval：记录面料大货的批色流程
//! 业务：剪样 → 发送客户 → 客户批色确认 → 通过/拒绝/返工/降级/报废
//!
//! 8 态状态机：
//!   pending → sampled → sent_to_customer → approved / rejected / rework
//!                                                    ↓
//!                                               downgraded / scrapped
//!
//! 关联任务：P0-F15（建表）/ P0-F16（剪大货样）/ P0-F17（客户批色确认）/ P0-F19（ship_order 校验）

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 大货批色审批实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bulk_color_approval")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 关联销售订单
    pub sales_order_id: i32,
    /// 关联染色批次（缸号）
    pub dye_batch_id: i32,
    /// 客户 ID
    pub customer_id: i64,
    /// 关联生产订单（可选，剪样时填入）
    pub production_order_id: Option<i32>,
    /// 四维标识：产品 ID
    pub product_id: Option<i32>,
    /// 四维标识：色号
    pub color_no: Option<String>,
    /// 四维标识：染色批号（lot 概念，防色差混批）
    pub dye_lot_no: Option<String>,
    /// 四维标识：批次号（缸号）
    pub batch_no: Option<String>,
    /// 样布类型：cut_sample(剪大货样) / lab_sample(化验室打样)
    pub sample_type: String,
    /// 样布 inventory_piece ID（剪样扣减后产生的 sample piece）
    pub sample_piece_id: Option<i64>,
    /// 样布长度（米）
    pub sample_length_m: Option<Decimal>,
    /// 批色状态：pending/sampled/sent_to_customer/approved/rejected/rework/downgraded/scrapped
    pub approval_status: String,
    /// 审批人用户 ID
    pub approver_id: Option<i32>,
    /// 批准时间
    pub approval_date: Option<DateTime<Utc>>,
    /// 发送给客户时间（批色时限计算锚点）
    pub sent_to_customer_at: Option<DateTime<Utc>>,
    /// 客户反馈
    pub customer_feedback: Option<String>,
    /// CIE D65 色差值 ΔE
    pub delta_e_value: Option<Decimal>,
    /// 拒绝/返工原因
    pub reject_reason: Option<String>,
    /// 交货门禁标志（true 时阻止发货）
    pub delivery_blocking: bool,
    /// 附件 URL（样布照片/批色单扫描件）
    pub attachment_url: Option<String>,
    /// 备注
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 批色记录 - 销售订单（多对一）
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::SalesOrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,
    /// 批色记录 - 染色批次（多对一）
    #[sea_orm(
        belongs_to = "super::dye_batch::Entity",
        from = "Column::DyeBatchId",
        to = "super::dye_batch::Column::Id"
    )]
    DyeBatch,
    /// 批色记录 - 客户（多对一）
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrder.def()
    }
}

impl Related<super::dye_batch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeBatch.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

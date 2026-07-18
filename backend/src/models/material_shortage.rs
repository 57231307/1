//! 缺料预警持久化 Model
//!
//! P0-B15（Batch 484）：修复审计报告 batch-18 §8.1 缺陷
//! 缺料预警状态不持久化 → 新建两表支持"识别→采购申请→采购订单→入库→解除"闭环
//!
//! 包含两个 Entity：
//! - `material_shortage_alerts`：缺料预警记录表
//! - `material_shortage_threshold_configs`：阈值配置表（单行 id=1）

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================
// material_shortage_alerts：缺料预警记录表
// ============================================================

/// 缺料预警记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "material_shortage_alerts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 缺料单号：MS-YYYYMMDD-NNN
    pub alert_no: String,
    /// 物料 ID（关联 products.id）
    pub material_id: i32,
    pub material_name: String,
    pub material_code: Option<String>,
    /// 需求量 / 可用量 / 缺口量 / 缺口率（识别时快照）
    pub required_quantity: Decimal,
    pub available_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub deficit_rate: Decimal,
    /// 级别：Critical / Severe / Warning / Normal
    pub level: String,
    /// 状态机：identified → purchase_request → purchase_order → received → resolved
    pub status: String,
    /// 受影响订单数（识别时快照）
    pub affected_orders_count: i32,
    /// 关联采购申请 ID（状态推进到 purchase_request 时填入）
    pub purchase_request_id: Option<i64>,
    /// 关联采购订单 ID（状态推进到 purchase_order 时填入）
    pub purchase_order_id: Option<i64>,
    /// 单位
    pub unit: Option<String>,
    /// 识别时间（首次检测到缺料的时间）
    pub identified_at: DateTime<Utc>,
    /// 解除时间（状态推进到 resolved 时填入）
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ============================================================
// material_shortage_threshold_configs：阈值配置表（单行 id=1）
// ============================================================

/// 缺料预警阈值配置表的子模块
///
/// 设计为单行配置（id=1 固定），通过 upsert 更新。
/// 使用独立子模块避免与 alerts Entity 名冲突。
pub mod threshold_config {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
    #[sea_orm(table_name = "material_shortage_threshold_configs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        /// 安全库存倍率（默认 1.00）
        pub safety_factor: Decimal,
        /// 紧急阈值（缺口百分比 >= 此值为 Critical，默认 100）
        pub critical_threshold: Decimal,
        /// 严重阈值（缺口百分比 >= 此值为 Severe，默认 50）
        pub severe_threshold: Decimal,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    /// 单行配置的固定 ID
    pub const SINGLE_ROW_ID: i64 = 1;
}

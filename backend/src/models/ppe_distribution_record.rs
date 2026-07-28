//! 防护用品配备记录模型（ppe_distribution_records 表）
//!
//! V15 P1 batch-08 缺陷 24：职业健康合规
//! 依据：《职业病防治法》个人防护用品配备

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 防护用品配备记录模型
///
/// 真实业务：记录 PPE（个人防护用品）发放情况，确保工人防护到位
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "ppe_distribution_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工人ID
    pub worker_id: i32,
    /// 防护用品名称
    pub ppe_name: String,
    /// 防护用品类型：mask(口罩) / gloves(手套) / goggles(护目镜) / earplug(耳塞) / respirator(防毒面具) / suit(防护服)
    pub ppe_type: String,
    /// 规格
    pub specification: Option<String>,
    /// 数量
    pub quantity: i32,
    /// 发放日期
    pub distribution_date: chrono::NaiveDate,
    /// 到期日期
    pub expiry_date: Option<chrono::NaiveDate>,
    /// 危害类型
    pub hazard_type: Option<String>,
    /// 状态：distributed(已发放) / returned(已回收) / expired(已过期)
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "unqualified_products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "unqualified_no")]
    pub unqualified_no: String,
    #[sea_orm(column_name = "inspection_id")]
    pub inspection_id: Option<i32>,
    #[sea_orm(column_name = "product_id")]
    pub product_id: i32,
    #[sea_orm(column_name = "batch_no")]
    pub batch_no: Option<String>,
    #[sea_orm(column_name = "unqualified_qty")]
    pub unqualified_qty: Decimal,
    #[sea_orm(column_name = "unqualified_reason")]
    pub unqualified_reason: String,
    #[sea_orm(column_name = "handling_method")]
    pub handling_method: String,
    #[sea_orm(column_name = "handling_status")]
    pub handling_status: String,
    #[sea_orm(column_name = "handling_by")]
    pub handling_by: Option<i32>,
    #[sea_orm(column_name = "handling_at")]
    pub handling_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    // v14 批次 421 T-P1-4：不合格品等级（B 级降级销售/C 级返工报废）
    #[sea_orm(column_name = "grade")]
    pub grade: Option<String>,
    // v14 批次 421：处理结果（降级销售单价/返工工时/报废损失金额）
    #[sea_orm(column_name = "handling_result")]
    pub handling_result: Option<String>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: DateTime<Utc>,
    // P1 batch-18 缺陷 5.1：降级联动库存等级同步标记
    #[sea_orm(column_name = "stock_grade_synced", default_value = false)]
    pub stock_grade_synced: bool,
    #[sea_orm(column_name = "stock_id")]
    pub stock_id: Option<i32>,
    // P1 batch-18 缺陷 5.3：报废二级审批（财务+总经理）
    #[sea_orm(column_name = "scrap_approval_status", default_value = "not_required")]
    pub scrap_approval_status: String,
    #[sea_orm(column_name = "approver_id_fin")]
    pub approver_id_fin: Option<i32>,
    #[sea_orm(column_name = "approver_id_gm")]
    pub approver_id_gm: Option<i32>,
    #[sea_orm(column_name = "approved_at_fin")]
    pub approved_at_fin: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "approved_at_gm")]
    pub approved_at_gm: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "scrap_loss_amount", column_type = "Decimal(Some((12, 2)))")]
    pub scrap_loss_amount: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 缺陷 5.3：报废审批状态常量
pub const SCRAP_NOT_REQUIRED: &str = "not_required";
pub const SCRAP_PENDING_FIN: &str = "pending_fin";
pub const SCRAP_PENDING_GM: &str = "pending_gm";
pub const SCRAP_APPROVED: &str = "approved";
pub const SCRAP_REJECTED: &str = "rejected";

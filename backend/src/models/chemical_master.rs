//! 染化料主数据模型（chemical_master 表）
//!
//! v14 批次 429：染化料主数据完善
//! 依据：面料行业真实业务调研文档 §4.3 染化料管理 + §11.4 染化料主数据管理
//! 真实业务：染料（分散/活性/还原/硫化/酸性/直接/阳离子）/ 助剂 / 化工原料
//! 含 GHS 危化品标注 + MSDS 安全数据表 + 保质期管理 + 安全库存

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 染化料主数据模型
///
/// 真实业务要点：
/// - 三种类型：染料 / 助剂 / 化工原料
/// - 染料专属：dye_category / color_index / fastness_light / fastness_washing
/// - 助剂专属：auxiliary_category / active_ingredient / concentration
/// - 危化品：GHS 分类 + UN 编号 + 信号词 + MSDS 链接
/// - 保质期：shelf_life_days + storage_condition + storage_temperature
/// - 安全库存：safety_stock / reorder_point / reorder_quantity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "chemical_master")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 染化料编码（唯一）
    pub chemical_code: String,
    /// 中文名
    pub chemical_name: String,
    /// 英文名
    pub chemical_name_en: Option<String>,
    /// 染化料类型：dye(染料) / auxiliary(助剂) / chemical(化工原料)
    pub chemical_type: String,
    /// 分类 ID（外键 → chemical_category）
    pub category_id: Option<i32>,
    /// 染料类别：dispersing(分散) / reactive(活性) / vat(还原) / sulfide(硫化) / acid(酸性) / direct(直接) / cationic(阳离子)
    pub dye_category: Option<String>,
    /// C.I. 染料索引号
    pub color_index: Option<String>,
    /// 助剂类别：pretreatment(前处理) / dyeing(染色) / finishing(后整理) / printing(印花)
    pub auxiliary_category: Option<String>,
    /// CAS 号
    pub cas_number: Option<String>,
    /// 分子式
    pub molecular_formula: Option<String>,
    /// 分子量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub molecular_weight: Option<Decimal>,
    /// 规格
    pub specification: Option<String>,
    /// 计量单位：kg/L/桶/袋/瓶
    pub unit: String,
    /// 标准价
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub standard_price: Decimal,
    /// 成本价
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub cost_price: Decimal,
    /// GHS 分类编码
    pub ghs_classification: Option<String>,
    /// UN 编号
    pub un_number: Option<String>,
    /// 危险级别
    pub hazard_class: Option<String>,
    /// GHS 象形图
    pub hazard_pictogram: Option<String>,
    /// 信号词：danger(危险) / warning(警告)
    pub signal_word: Option<String>,
    /// MSDS 文件 URL
    pub msds_url: Option<String>,
    /// MSDS 版本
    pub msds_version: Option<String>,
    /// MSDS 更新时间
    pub msds_updated_at: Option<DateTimeWithTimeZone>,
    /// 保质期天数
    pub shelf_life_days: Option<i32>,
    /// 存储条件：防潮/防火/防爆/避光
    pub storage_condition: Option<String>,
    /// 存储温度
    pub storage_temperature: Option<String>,
    /// 安全库存阈值
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub safety_stock: Decimal,
    /// 再订货点
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub reorder_point: Decimal,
    /// 再订货量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub reorder_quantity: Decimal,
    /// 包装单位：桶/袋/箱/瓶
    pub package_unit: Option<String>,
    /// 单包装容量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub package_capacity: Option<Decimal>,
    /// 每托盘件数
    pub packages_per_pallet: Option<i32>,
    /// 供应商 ID（外键 → suppliers）
    pub supplier_id: Option<i32>,
    /// 供应商产品编码
    pub supplier_product_code: Option<String>,
    /// 日晒牢度（染料专属）
    pub fastness_light: Option<String>,
    /// 水洗牢度（染料专属）
    pub fastness_washing: Option<String>,
    /// 有效成分（助剂专属）
    pub active_ingredient: Option<String>,
    /// 浓度（助剂专属）
    #[sea_orm(column_type = "Decimal(Some((8, 4)))")]
    pub concentration: Option<Decimal>,
    /// 状态：active(启用) / inactive(停用) / discontinued(停产)
    pub status: String,
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
    /// 关联染化料分类
    #[sea_orm(
        belongs_to = "super::chemical_category::Entity",
        from = "Column::CategoryId",
        to = "super::chemical_category::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Category,
    /// 关联供应商
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Supplier,
    /// 一对多：染化料批次
    #[sea_orm(has_many = "super::chemical_lot::Entity")]
    Lots,
}

impl Related<super::chemical_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Category.def()
    }
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::chemical_lot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lots.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

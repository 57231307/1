use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "suppliers")]
pub struct Model {
    /// 供应商 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 供应商编码
    pub supplier_code: String,
    /// 供应商名称
    pub supplier_name: String,
    /// 供应商简称
    pub supplier_short_name: String,
    /// 供应商类型
    pub supplier_type: String,
    /// 统一社会信用代码
    pub credit_code: String,
    /// 注册地址
    pub registered_address: String,
    /// 经营地址
    pub business_address: Option<String>,
    /// 法人代表
    pub legal_representative: String,
    /// 注册资本（万元）
    pub registered_capital: Decimal,
    /// 成立日期
    pub establishment_date: Date,
    /// 营业期限
    pub business_term: Option<String>,
    /// 经营范围
    pub business_scope: Option<String>,
    /// 纳税人类型
    pub taxpayer_type: String,
    /// 开户银行
    pub bank_name: String,
    /// 银行账号
    pub bank_account: String,
    /// 联系电话
    pub contact_phone: String,
    /// 传真
    pub fax: Option<String>,
    /// 公司网址
    pub website: Option<String>,
    /// 联系邮箱
    pub email: Option<String>,
    /// 主营业务
    pub main_business: Option<String>,
    /// 主要市场
    pub main_market: Option<String>,
    /// 员工人数
    pub employee_count: Option<i32>,
    /// 年营业额（万元）
    pub annual_revenue: Option<Decimal>,
    /// 供应商等级（A/B/C/D）
    pub grade: String,
    /// 等级评分
    pub grade_score: Decimal,
    /// 最后评估日期
    pub last_evaluation_date: Option<Date>,
    /// 状态：active/inactive/disabled/blacklisted
    pub status: String,
    /// 是否启用
    pub is_enabled: bool,
    /// 是否启用批次核算
    pub assist_batch: bool,
    /// 是否启用供应商核算
    pub assist_supplier: bool,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
    /// 创建人 ID
    pub created_by: Option<i32>,
    /// 更新人 ID
    pub updated_by: Option<i32>,
    /// 备注
    pub remarks: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 供应商 - 联系人（一对多）
    #[sea_orm(has_many = "super::supplier_contact::Entity")]
    SupplierContacts,
    /// 供应商 - 资质（一对多）
    #[sea_orm(has_many = "super::supplier_qualification::Entity")]
    SupplierQualifications,
    /// 供应商 - 黑名单（一对一）
    #[sea_orm(has_one = "super::supplier_blacklist::Entity")]
    SupplierBlacklist,
}

impl Related<super::supplier_contact::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierContacts.def()
    }
}

impl Related<super::supplier_qualification::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierQualifications.def()
    }
}

impl Related<super::supplier_blacklist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierBlacklist.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

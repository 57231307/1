//! 产品分类默认种子。
//!
//! 新库首启时产品分类表为空，新建产品表单的分类必填项没有候选，被前端校验拦截
//! 而表现为保存失败。本模块在启动初始化链中把产品分类表为空的情形补齐为三个
//! 默认分类（坯布、成品布、辅料），中文名称与产品类型词表常量单源复用，
//! code 为对应稳定 ASCII 符号键，重跑幂等不重复插入。

use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, NotSet, PaginatorTrait, Set};

use crate::models::product_category::{ActiveModel, Entity as ProductCategoryEntity};
use crate::models::status::general::product_type;
use crate::utils::error::AppError;

/// 产品分类表为空时写入三个默认分类，返回本次实际插入的行数。
///
/// 表非空即整体跳过并返回 0（重跑等价，不重复插入）。名称取产品类型词表常量值
/// （中文业务真相单源），编码取与之对应的稳定 ASCII 符号键，parent_id 与
/// description 留空，created_at 与 updated_at 取当前时间。
///
/// # Errors
///
/// 计数或插入的数据库访问失败时返回底层 `AppError`，由调用方决定告警口径。
pub async fn seed_default_product_categories(db: &DatabaseConnection) -> Result<u64, AppError> {
    let existing = ProductCategoryEntity::find().count(db).await?;
    if existing > 0 {
        return Ok(0);
    }

    // code 为稳定 ASCII 符号键，name 逐字取词表常量，禁止另起第二套分类名
    let defaults = [
        ("GREIGE", product_type::GREIGE),
        ("FINISHED", product_type::FINISHED),
        ("AUXILIARY", product_type::AUXILIARY),
    ];

    for (code, name) in defaults {
        let model = ActiveModel {
            id: NotSet,
            name: Set(name.to_string()),
            code: Set(code.to_string()),
            parent_id: Set(None),
            description: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };
        model.insert(db).await?;
    }

    Ok(defaults.len() as u64)
}

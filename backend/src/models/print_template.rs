#![allow(dead_code)]
//! 打印模板模型
//!
//! 映射 `print_templates` 表:存储单据类型到打印模板的映射,以及纸张/方向/份数/四边边距/
//! 条码等打印缺省参数与模板正文、样式、变量集合。字段可空性与 DDL 逐列一致
//! (NOT NULL 列为非 Option,可空列为 Option;时间列为 TIMESTAMPTZ,JSONB 为 Json)。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 打印模板 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "print_templates")]
pub struct Model {
    /// 模板主键
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 模板业务唯一编码
    pub template_code: String,

    /// 模板名称
    pub template_name: String,

    /// 模板描述
    pub description: String,

    /// 模板归属业务模块
    pub module: String,

    /// 模板类型(order/report 等,序列化为前端 type 键)
    pub template_type: String,

    /// 关联单据类型标识(“单据→模板”映射的查找键)
    pub doc_type: String,

    /// 缺省纸张尺寸
    pub paper_size: String,

    /// 缺省打印方向(portrait/landscape)
    pub orientation: String,

    /// 缺省打印份数
    pub copies_default: i16,

    /// 缺省上边距(毫米)
    pub margin_top_mm: i16,

    /// 缺省下边距(毫米)
    pub margin_bottom_mm: i16,

    /// 缺省左边距(毫米)
    pub margin_left_mm: i16,

    /// 缺省右边距(毫米)
    pub margin_right_mm: i16,

    /// 是否缺省启用条码
    pub barcode_default: bool,

    /// 缺省打印目标符号名(实际地址由运行期配置解析)
    pub default_target_name: Option<String>,

    /// 模板正文(HTML)
    pub content: String,

    /// 模板样式(CSS)
    pub css_styles: String,

    /// 模板变量集合
    pub variables: Json,

    /// 同模块内是否为缺省模板
    pub is_default: bool,

    /// 状态(active/inactive)
    pub status: String,

    /// 创建人用户 ID
    pub created_by: i32,

    /// 创建人用户名(展示用)
    pub created_by_name: String,

    /// 创建时间
    pub created_at: DateTimeWithTimeZone,

    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

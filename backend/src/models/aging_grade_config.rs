use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 账龄档位配置实体 - batch-15 P3: 账龄档位配置化
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "aging_grade_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 档位名称（如：当前、1-30天、31-60天、61-90天、90天以上）
    pub grade_name: String,
    /// 最小天数（含）
    pub min_days: i32,
    /// 最大天数（含），-1 表示无上限
    pub max_days: i32,
    /// 档位顺序
    pub sort_order: i32,
    /// 是否启用
    pub is_active: bool,
    /// 备注
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 创建账龄档位 DTO
#[derive(Deserialize)]
pub struct CreateAgingGradeDto {
    pub grade_name: String,
    pub min_days: i32,
    pub max_days: i32,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
    pub remark: Option<String>,
}

/// 更新账龄档位 DTO
#[derive(Deserialize)]
pub struct UpdateAgingGradeDto {
    pub grade_name: Option<String>,
    pub min_days: Option<i32>,
    pub max_days: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
    pub remark: Option<String>,
}

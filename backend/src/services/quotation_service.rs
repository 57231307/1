//! 销售报价单服务层（facade）
//!
//! D11 拆分：保留 QuotationService struct/构造函数/ServiceError/单元测试，
//! 业务方法（create_draft/list/get_by_id/update/cancel + helpers）迁移至 quotation_ops 子模块。

use sea_orm::DatabaseConnection;
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::utils::error::AppError;

/// 业务错误
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("报价单不存在")]
    NotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// 批次 265：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 销售报价单服务
pub struct QuotationService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl QuotationService {
    /// 从数据库连接直接构造（与项目其他服务保持一致）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }
}

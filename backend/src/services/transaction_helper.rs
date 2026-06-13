#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
use crate::utils::error::AppError;
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};
use std::future::Future;

pub async fn with_transaction<T, F, Fut>(
    db: &DatabaseConnection,
    operation: F,
) -> Result<T, AppError>
where
    F: FnOnce(&DatabaseTransaction) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let txn = db.begin().await.map_err(AppError::from)?;
    match operation(&txn).await {
        Ok(result) => {
            txn.commit().await.map_err(AppError::from)?;
            Ok(result)
        }
        Err(e) => {
            tracing::error!("Transaction rolled back due to error: {}", e);
            if let Err(rollback_err) = txn.rollback().await {
                tracing::error!("事务回滚失败: {}", rollback_err);
            }
            Err(e)
        }
    }
}

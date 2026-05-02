use crate::utils::error::AppError;
use sea_orm::{DatabaseConnection, TransactionTrait, DatabaseTransaction};
use std::future::Future;

pub async fn with_transaction<T, F, Fut>(
    db: &DatabaseConnection,
    operation: F,
) -> Result<T, AppError>
where
    F: FnOnce(DatabaseTransaction) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let txn = db.begin().await.map_err(AppError::from)?;
    match operation(txn.clone()).await {
        Ok(result) => {
            txn.commit().await.map_err(AppError::from)?;
            Ok(result)
        }
        Err(e) => {
            tracing::error!("Transaction rolled back due to error: {}", e);
            txn.rollback().await.ok();
            Err(e)
        }
    }
}

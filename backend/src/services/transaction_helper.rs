use crate::utils::error::AppError;
use sea_orm::{DatabaseConnection, TransactionTrait, DatabaseTransaction};
use std::future::Future;
use std::pin::Pin;

pub async fn with_transaction<T, F>(
    db: &DatabaseConnection,
    operation: F,
) -> Result<T, AppError>
where
    for<'a> F: FnOnce(&'a DatabaseTransaction) -> Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>,
{
    let txn = db.begin().await.map_err(AppError::from)?;
    match operation(&txn).await {
        Ok(result) => {
            txn.commit().await.map_err(AppError::from)?;
            Ok(result)
        }
        Err(e) => {
            txn.rollback().await.ok();
            Err(e)
        }
    }
}

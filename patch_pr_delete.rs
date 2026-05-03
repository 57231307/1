use sea_orm::TransactionTrait;
use crate::models::{purchase_return, purchase_return_item};
use sea_orm::EntityTrait;
use crate::utils::error::AppError;

pub async fn delete(&self, id: i32) -> Result<(), AppError> {
    let txn = self.db.begin().await?;
    
    // Check status
    let ret = purchase_return::Entity::find_by_id(id).one(&txn).await?.ok_or(AppError::NotFound("Return not found".to_string()))?;
    if ret.status != "DRAFT" {
        return Err(AppError::BadRequest("Only DRAFT returns can be deleted".to_string()));
    }
    
    purchase_return_item::Entity::delete_many()
        .filter(purchase_return_item::Column::ReturnId.eq(id))
        .exec(&txn)
        .await?;
        
    purchase_return::Entity::delete_by_id(id).exec(&txn).await?;
    
    txn.commit().await?;
    Ok(())
}

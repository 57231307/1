use sea_orm::*;
use serde::{Serialize, de::DeserializeOwned};

pub async fn update_with_audit<E, A, C>(
    active_model: A,
    db: &C,
) -> Result<<E as EntityTrait>::Model, DbErr>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + Send + Sync,
    C: ConnectionTrait,
    <E as EntityTrait>::Model: Serialize + DeserializeOwned + Sync + Send + Clone,
{
    // How to get PK value?
    let pk_col = E::PrimaryKey::iter().next().unwrap();
    let pk_val = active_model.get(pk_col);
    
    // We can't easily find_by_id with `sea_orm::Value`.
    // But we can use `find().filter(pk_col.eq(pk_val))`!
    let old_data = E::find()
        .filter(sea_query::SimpleExpr::Binary(
            Box::new(sea_query::SimpleExpr::Column(pk_col.into_column())),
            sea_query::BinOper::Equal,
            Box::new(sea_query::SimpleExpr::Value(pk_val))
        ))
        .one(db)
        .await?;

    let new_model = active_model.update(db).await?;
    
    // Compute JSON diff here!
    
    Ok(new_model)
}

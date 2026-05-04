use bingxi_backend::models::role_permission;
use sea_orm::{Database, EntityTrait, QueryFilter, ColumnTrait};

#[tokio::main]
async fn main() {
    let db = Database::connect("postgres://bingxi:d5eb610ccf1a701dac02d5.dbcba8f5f546a@39.99.34.194:5432/bingxi").await.unwrap();
    let role_perms = role_permission::Entity::find()
        .filter(role_permission::Column::RoleId.eq(1))
        .filter(role_permission::Column::Allowed.eq(true))
        .all(&db)
        .await;
    println!("{:?}", role_perms);
}

use bingxi_backend::models::role_permission;
use sea_orm::{Database, EntityTrait, QueryFilter, ColumnTrait};

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://bingxi:bingxi@localhost:5432/bingxi".to_string());
    let db = Database::connect(&db_url).await.unwrap();
    let role_perms = role_permission::Entity::find()
        .filter(role_permission::Column::RoleId.eq(1))
        .filter(role_permission::Column::Allowed.eq(true))
        .all(&db)
        .await;
    println!("{:?}", role_perms);
}

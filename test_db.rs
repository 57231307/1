use sea_orm::{Database, EntityTrait};
#[tokio::main]
async fn main() {
    let db = Database::connect("postgres://postgres:postgres@localhost:5432/bingxi").await.unwrap();
    let w = bingxi_backend::models::warehouse::Entity::find().all(&db).await;
    println!("{:?}", w);
}

use sea_orm::{Database, ConnectionTrait};

#[tokio::main]
async fn main() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let sql = "CREATE TABLE test (id INTEGER); INSERT INTO test VALUES (1);";
    let _ = db.execute_unprepared(sql).await.unwrap();
    println!("Success!");
}

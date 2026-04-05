use sea_orm::{Database, ConnectionTrait, Statement, DatabaseBackend};
#[tokio::main]
async fn main() {
    let db = Database::connect("postgres://test_user:test_pass@localhost:5432/test_db").await.unwrap();
    let sql = "CREATE TABLE IF NOT EXISTS test_tbl (id SERIAL PRIMARY KEY); INSERT INTO test_tbl DEFAULT VALUES;";
    db.execute(Statement::from_string(DatabaseBackend::Postgres, sql)).await.unwrap();
    println!("SUCCESS");
}

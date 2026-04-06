use sea_orm::{Database, ConnectionTrait, Statement, DatabaseBackend};

#[tokio::test]
async fn test_multi_sql() {
    let db = Database::connect("postgres://test_user:test_pass@localhost:5432/test_db").await.unwrap();
    let sql = "CREATE TABLE IF NOT EXISTS test_multi_1 (id INT); CREATE TABLE IF NOT EXISTS test_multi_2 (id INT);";
    
    match db.execute(Statement::from_string(DatabaseBackend::Postgres, sql)).await {
        Ok(_) => println!("SUCCESS with from_string"),
        Err(e) => println!("ERROR with from_string: {}", e),
    }

    match db.execute_unprepared(sql).await {
        Ok(_) => println!("SUCCESS with execute_unprepared"),
        Err(e) => println!("ERROR with execute_unprepared: {}", e),
    }
}

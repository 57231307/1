import re

with open('backend/src/services/init_service.rs', 'r', encoding='utf-8') as f:
    content = f.read()

replace_str = '''
                let backend = self.db.get_database_backend();

                // sqlx/sea-orm backend with prepared statements doesn't support multiple commands in one query
                // but we can use execute_unprepared which sends the raw SQL query to the database.
                // This supports multiple statements separated by semicolons and correctly handles PL/pgSQL functions with $$ quotes.
                self.db.execute_unprepared(&sql)
                    .await
                    .map_err(|e| InitError::DatabaseError(format!("执行SQL脚本 {:?} 失败: {}", path.file_name().unwrap(), e)))?;
'''

content = re.sub(
    r'let backend = self\.db\.get_database_backend\(\);\s*// sqlx/sea-orm backend.*?\}',
    replace_str.strip(),
    content,
    flags=re.DOTALL
)

with open('backend/src/services/init_service.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed init service with execute_unprepared")

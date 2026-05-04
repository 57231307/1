import psycopg2

conn = psycopg2.connect(
    dbname="bingxi_erp",
    user="postgres",
    password="password",
    host="127.0.0.1",
    port="5432"
)
cursor = conn.cursor()
cursor.execute("SELECT username, password_hash FROM users;")
print(cursor.fetchall())

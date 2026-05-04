import psycopg2

conn = psycopg2.connect(
    dbname="bingxi_erp",
    user="postgres",
    password="password",
    host="127.0.0.1",
    port="5432"
)
cursor = conn.cursor()
cursor.execute("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'active' AND pid <> pg_backend_pid();")
print(cursor.fetchall())

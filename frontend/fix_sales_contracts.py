import psycopg2

conn = psycopg2.connect(
    dbname="bingxi_erp",
    user="postgres",
    password="password",
    host="127.0.0.1",
    port="5432"
)
conn.autocommit = True
cursor = conn.cursor()

try:
    cursor.execute('ALTER TABLE sales_contracts ADD COLUMN tenant_id INTEGER DEFAULT 1;')
    print("Added tenant_id to sales_contracts")
except Exception as e:
    print(e)


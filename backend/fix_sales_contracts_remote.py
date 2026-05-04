import psycopg2

DB_URL = "postgres://bingxi:d5eb610ccf1a701dac02d5.dbcba8f5f546a@39.99.34.194:5432/bingxi"
with psycopg2.connect(DB_URL) as conn:
    conn.autocommit = True
    with conn.cursor() as cur:
        try:
            cur.execute('ALTER TABLE sales_contracts ADD COLUMN tenant_id INTEGER DEFAULT 1;')
            print("Added tenant_id to sales_contracts")
        except Exception as e:
            print(e)

import psycopg2
DB_URL = "postgres://bingxi:d5eb610ccf1a701dac02d5.dbcba8f5f546a@39.99.34.194:5432/bingxi"
with psycopg2.connect(DB_URL) as conn:
    with conn.cursor() as cur:
        cur.execute("SELECT username, password_hash FROM users WHERE username = 'admin';")
        print(cur.fetchall())

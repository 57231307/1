-- Migration: 修复所有 timestamp without time zone 列为 timestamp with time zone
-- 原因: Rust SeaORM DateTime<Utc> 需要 PostgreSQL 的 timestamp with time zone 类型
-- 日期: 2026-05-19

-- 批量转换所有 timestamp without time zone 列
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN 
        SELECT table_name, column_name 
        FROM information_schema.columns 
        WHERE table_schema = 'public' 
          AND data_type = 'timestamp without time zone'
    LOOP
        EXECUTE format('ALTER TABLE %I ALTER COLUMN %I TYPE timestamp with time zone USING %I AT TIME ZONE ''UTC''', 
                        r.table_name, r.column_name, r.column_name);
        RAISE NOTICE 'Fixed: %.%', r.table_name, r.column_name;
    END LOOP;
END
$$;

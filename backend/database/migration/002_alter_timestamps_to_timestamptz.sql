DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT c.table_schema, c.table_name, c.column_name
        FROM information_schema.columns c
        JOIN information_schema.tables t ON c.table_schema = t.table_schema AND c.table_name = t.table_name
        WHERE c.table_schema = 'public'
          AND c.data_type = 'timestamp without time zone'
          AND t.table_type = 'BASE TABLE'
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.%I ALTER COLUMN %I TYPE timestamp with time zone USING %I AT TIME ZONE ''UTC'';',
            r.table_schema, r.table_name, r.column_name, r.column_name
        );
    END LOOP;
END
$$;
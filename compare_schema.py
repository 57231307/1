import os
import re

MODELS_DIR = '/workspace/backend/src/models/'
SQL_FILE = '/workspace/backend/database/migration/001_consolidated_schema.sql'

with open(SQL_FILE, 'r', encoding='utf-8') as f:
    sql_content = f.read()

# Function to extract table name from a SeaORM model file
def get_table_name(content):
    # #[sea_orm(table_name = "users")]
    match = re.search(r'#\[sea_orm\(table_name\s*=\s*"([^"]+)"\)\]', content)
    if match:
        return match.group(1)
    return None

# Function to extract fields from a SeaORM model file
def get_fields(content):
    # Find the pub struct Model { ... } block
    match = re.search(r'pub\s+struct\s+Model\s*\{([^}]*)\}', content)
    if not match:
        return []
    
    fields_block = match.group(1)
    # Extract pub field_name: Type
    fields = re.findall(r'pub\s+([a-zA-Z0-9_]+)\s*:', fields_block)
    return fields

missing_columns = []
tables_processed = 0

for filename in os.listdir(MODELS_DIR):
    if not filename.endswith('.rs') or filename in ['mod.rs', 'prelude.rs']:
        continue
    
    with open(os.path.join(MODELS_DIR, filename), 'r', encoding='utf-8') as f:
        content = f.read()
        
    table_name = get_table_name(content)
    if not table_name:
        continue
        
    fields = get_fields(content)
    tables_processed += 1
    
    # Check if table exists in SQL
    table_create_pattern = re.compile(rf'CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?{table_name}\b', re.IGNORECASE)
    if not table_create_pattern.search(sql_content):
        # Table might not exist at all in the schema
        missing_columns.append((table_name, "TABLE_MISSING", filename))
        continue
    
    # Extract the block of CREATE TABLE
    # Note: this simple regex might fail if there are nested parentheses or triggers
    # So we'll also just do a global search for the column within the SQL
    
    for field in fields:
        # Ignore id and some generic stuff just in case, but let's check everything
        # Look for the field name in the CREATE TABLE block for this table, or in an ALTER TABLE block for this table
        
        # 1. Is there an ALTER TABLE {table} ADD COLUMN {field}?
        alter_pattern = re.compile(rf'ALTER\s+TABLE\s+{table_name}\s+ADD\s+COLUMN\s+(IF\s+NOT\s+EXISTS\s+)?{field}\b', re.IGNORECASE)
        if alter_pattern.search(sql_content):
            continue
            
        # 2. Is there {field} definition in the CREATE TABLE block?
        # We can find the CREATE TABLE block by finding "CREATE TABLE {table_name} (" and matching until ";"
        # This is a bit tricky with regex, let's use a simpler approach:
        # Does the string "{field} " or "{field}\t" or "{field}\n" exist?
        # A more precise way is to match `\b{field}\b` followed by a SQL type.
        
        # Let's find the CREATE TABLE block
        block_match = re.search(rf'CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?{table_name}\s*\((.*?)\);', sql_content, re.IGNORECASE | re.DOTALL)
        found_in_block = False
        if block_match:
            block = block_match.group(2)
            # Check if field exists as a column definition (at start of line or after comma)
            # e.g. "id SERIAL", " name VARCHAR"
            if re.search(rf'\b{field}\b\s+(VARCHAR|INTEGER|DECIMAL|BOOLEAN|TIMESTAMP|DATE|TEXT|JSONB|SERIAL|INT|NUMERIC|CHAR|DOUBLE|REAL|SMALLINT|BIGINT|UUID|JSON)', block, re.IGNORECASE):
                found_in_block = True
            elif re.search(rf'\b{field}\b', block, re.IGNORECASE):
                # Fallback if type is not matched
                found_in_block = True
                
        if not found_in_block:
            missing_columns.append((table_name, field, filename))

print(f"Processed {tables_processed} models.")

# Group by table
missing_by_table = {}
for t, f, file in missing_columns:
    if t not in missing_by_table:
        missing_by_table[t] = []
    missing_by_table[t].append((f, file))

for t, items in sorted(missing_by_table.items()):
    print(f"Table: {t}")
    for f, file in items:
        if f == "TABLE_MISSING":
            print(f"  [MISSING ENTIRE TABLE] ({file})")
        else:
            print(f"  - {f}")

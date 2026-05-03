import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Skip if already patched
    if 'async fn before_save' in content:
        return

    # Find table_name
    match = re.search(r'#\[sea_orm\(table_name\s*=\s*"([^"]+)"\)\]', content)
    if not match:
        return
    table_name = match.group(1)

    # Find Primary Key type and name
    # usually: 
    # #[sea_orm(primary_key)]
    # pub id: i32,
    pk_match = re.search(r'#\[sea_orm\(primary_key(?:[^\]]*)\)\]\s+pub\s+(\w+):\s+([^,;\n]+)', content)
    if not pk_match:
        return
    pk_name = pk_match.group(1)
    pk_type = pk_match.group(2)

    # Check if the model has serde derivations
    if 'Serialize' not in content and 'Deserialize' not in content:
        # We need serde to serialize to JSON
        # Actually most models already have it, if not we skip or add it
        pass

    replacement = f"""
use sea_orm::ActiveValue;
use serde_json::Value;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {{
    async fn before_save<C>(self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {{
        if !insert {{
            if let ActiveValue::Set({pk_name}) | ActiveValue::Unchanged({pk_name}) = self.{pk_name}.clone() {{
                if let Ok(Some(old_data)) = Entity::find_by_id({pk_name}.clone()).one(db).await {{
                    if let Ok(old_json) = serde_json::to_value(&old_data) {{
                        // To get new data, we apply ActiveModel over old_data
                        let mut new_data = old_data.clone();
                        // This requires applying the active model, but ActiveModel doesn't have a simple apply method without moving.
                        // Actually we can just serialize the ActiveModel directly if we want, but it serializes differently.
                    }}
                }}
            }}
        }}
        Ok(self)
    }}
}}
"""
    # Replace the default empty impl
    content = re.sub(r'impl ActiveModelBehavior for ActiveModel\s*\{\s*\}', replacement, content)
    
    with open(filepath, 'w') as f:
        f.write(content)

process_file('/home/root0/桌面/121/1/backend/src/models/finance_invoice.rs')

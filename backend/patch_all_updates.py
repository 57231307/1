import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Find the table_name from the Model if possible, or we can just pass "" for now.
    # Actually, getting the table_name is hard, so let's pass a placeholder "auto_audit"
    # and we can fix it later if needed.

    # Match active_model.update(db).await?
    # db could be `&*self.db`, `&txn`, `self.db.as_ref()`
    
    # We will replace `.update(DB).await?` with `crate::services::audit_log_service::AuditLogService::update_with_audit(DB, "auto_audit", active_model, Some(0)).await?`
    # But wait, `.update(DB)` is called on the object. We need to extract the object.
    
    # Regex to match `identifier.update(DB).await?`
    # It's better to use regex:
    # `([a-zA-Z0-9_]+)\.update\(([^)]+)\)\.await\?`
    
    def replacer(match):
        obj = match.group(1)
        db = match.group(2)
        return f"crate::services::audit_log_service::AuditLogService::update_with_audit({db}, \"auto_audit\", {obj}, Some(0)).await?"
    
    new_content = re.sub(r'([a-zA-Z0-9_]+)\.update\(([^)]+)\)\.await\?', replacer, content)
    
    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
        print(f"Patched {filepath}")

services_dir = "/home/root0/桌面/121/1/backend/src/services"
for filename in os.listdir(services_dir):
    if filename.endswith(".rs") and filename != "audit_log_service.rs":
        process_file(os.path.join(services_dir, filename))


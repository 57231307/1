import os
import re

files = [
    'backend/src/models/business_trace_snapshot.rs',
    'backend/src/models/batch_trace_log.rs',
    'backend/src/models/log_login.rs'
]

for filepath in files:
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
        
    # We want to insert `pub is_deleted: bool,` before `}` of `pub struct Model {`
    # and `IsDeleted,` before `}` of `pub enum Column {`
    
    model_match = re.search(r'pub struct Model \{([^}]+)\}', content)
    if model_match:
        inner = model_match.group(1)
        new_inner = inner + "    pub is_deleted: bool,\n"
        content = content.replace(model_match.group(0), f"pub struct Model {{{new_inner}}}")
        
    col_match = re.search(r'pub enum Column \{([^}]+)\}', content)
    if col_match:
        inner = col_match.group(1)
        new_inner = inner + "    IsDeleted,\n"
        content = content.replace(col_match.group(0), f"pub enum Column {{{new_inner}}}")
        
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Fixed the 3 files.")

import os
import re
import glob

services_dir = 'backend/src/services/'
files = glob.glob(os.path.join(services_dir, '*.rs'))

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Pattern to match `module::Entity::find()`
    # We will ignore `Entity::find()` without module prefix for simplicity, 
    # but the grep shows almost all have module prefix (e.g. `user::Entity::find()`).
    
    # 1. `module::Entity::find()`
    pattern_find = r'([a-zA-Z0-9_]+)::Entity::find\(\)'
    
    def replacer_find(match):
        module_name = match.group(1)
        # Avoid `crate::models::` part if captured, but `[a-zA-Z0-9_]+` won't capture `::`.
        return f'{module_name}::Entity::find().filter({module_name}::Column::IsDeleted.eq(false))'
        
    content = re.sub(pattern_find, replacer_find, content)
    
    # 2. `module::Entity::find_by_id(expr)`
    pattern_find_id = r'([a-zA-Z0-9_]+)::Entity::find_by_id\(([^)]+)\)'
    
    def replacer_find_id(match):
        module_name = match.group(1)
        expr = match.group(2)
        return f'{module_name}::Entity::find_by_id({expr}).filter({module_name}::Column::IsDeleted.eq(false))'
        
    content = re.sub(pattern_find_id, replacer_find_id, content)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Patched {os.path.basename(filepath)}")

for filepath in files:
    process_file(filepath)

import os
import re
import glob

services_dir = 'backend/src/services/'
files = glob.glob(os.path.join(services_dir, '*.rs'))

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    changed = False
    
    # Add imports if needed
    if '.filter(' in content and 'Column::IsDeleted' in content:
        if 'use sea_orm::QueryFilter;' not in content:
            content = content.replace('use sea_orm::{', 'use sea_orm::{QueryFilter, ')
            if 'use sea_orm::QueryFilter;' not in content:
                content = "use sea_orm::QueryFilter;\n" + content
            changed = True
            
        if 'use sea_orm::ColumnTrait;' not in content:
            content = content.replace('use sea_orm::{', 'use sea_orm::{ColumnTrait, ')
            if 'use sea_orm::ColumnTrait;' not in content:
                content = "use sea_orm::ColumnTrait;\n" + content
            changed = True

    # Fix `crate::models::user::Entity::find_by_id(user_id).filter(user::Column::IsDeleted.eq(false))`
    # to `crate::models::user::Entity::find_by_id(user_id).filter(crate::models::user::Column::IsDeleted.eq(false))`
    
    # We can just replace `([a-zA-Z0-9_]+)::Column::IsDeleted` with `crate::models::\1::Column::IsDeleted` if it's missing.
    # Actually, some use `crate::models::xxx::Column` and some use `xxx::Column` directly.
    # It's safer to check if `crate::models::xxx` is used.
    
    # Let's just find `crate::models::([a-zA-Z0-9_]+)::Entity::find(.*?).filter\(\1::Column::IsDeleted`
    # and replace `\1::Column::IsDeleted` with `crate::models::\1::Column::IsDeleted`
    
    pattern = r'crate::models::([a-zA-Z0-9_]+)::Entity::find([^)]*)\)\.filter\(\1::Column::IsDeleted'
    
    def replacer(match):
        module_name = match.group(1)
        method_call = match.group(2)
        return f'crate::models::{module_name}::Entity::find{method_call}).filter(crate::models::{module_name}::Column::IsDeleted'
        
    new_content = re.sub(pattern, replacer, content)
    if new_content != content:
        content = new_content
        changed = True

    if changed:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)

for filepath in files:
    process_file(filepath)

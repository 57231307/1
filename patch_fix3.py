import os
import re
import glob

services_dir = 'backend/src/services/'
files = glob.glob(os.path.join(services_dir, '*.rs'))

for filepath in files:
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Find all `use sea_orm::{...}` and `use sea_orm::...;`
    # This is a bit tricky if they span multiple lines.
    # Let's just collect everything that looks like `use sea_orm::...;`
    
    # We will remove all lines starting with `use sea_orm::` (and handling multi-line)
    # Then we will re-insert what is needed.
    # It's safer to just do a simple replacement for the known bad ones.
    
    # E0753: expected outer doc comment
    # Let's look for `///` or `//!` followed by `use sea_orm` which might be messed up.
    
    # Let's fix the duplicates:
    # Find `use sea_orm::{...};` block and deduplicate its contents.
    pattern = r'use sea_orm::\{([^}]+)\};'
    
    def replacer(match):
        items = match.group(1).replace('\n', ' ').split(',')
        clean_items = set()
        for item in items:
            item = item.strip()
            if item:
                clean_items.add(item)
        return 'use sea_orm::{' + ', '.join(sorted(list(clean_items))) + '};'
        
    content = re.sub(pattern, replacer, content)
    
    # ensure ColumnTrait and QueryFilter are there if `.filter` is used
    if '.filter(' in content and 'use sea_orm::{' in content:
        if 'QueryFilter' not in content:
            content = content.replace('use sea_orm::{', 'use sea_orm::{QueryFilter, ', 1)
        if 'ColumnTrait' not in content:
            content = content.replace('use sea_orm::{', 'use sea_orm::{ColumnTrait, ', 1)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

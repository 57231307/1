import os
import re
import glob

models_dir = 'backend/src/models/'
for filepath in glob.glob(os.path.join(models_dir, '*.rs')):
    if filepath.endswith('mod.rs') or filepath.endswith('p1p2_mod.rs') or filepath.endswith('ar_mod.rs') or filepath.endswith('cost_mod.rs'):
        continue
        
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
        
    # Check if it's a SeaORM model
    if 'pub struct Model {' in content and 'is_deleted: bool' not in content:
        # We need to insert `pub is_deleted: bool,` right before `pub created_at:` or at the end of the struct
        if 'pub created_at:' in content:
            content = re.sub(r'(\s*)(pub created_at:)', r'\1pub is_deleted: bool,\1\2', content)
        else:
            # Insert before the closing brace of Model
            content = re.sub(r'(\s*)(})', r'\1    pub is_deleted: bool,\n\1\2', content, count=1)
            
        # We also need to add `IsDeleted,` to the Column enum if it exists
        if 'pub enum Column {' in content:
            if 'CreatedAt,' in content:
                content = re.sub(r'(\s*)(CreatedAt,)', r'\1IsDeleted,\1\2', content)
            else:
                # Find the closing brace of Column enum
                # Wait, better to just append before `}`
                # We can find `pub enum Column {` and its `}`
                col_match = re.search(r'pub enum Column \{([^}]+)\}', content)
                if col_match:
                    inner = col_match.group(1)
                    if 'IsDeleted' not in inner:
                        new_inner = inner + "    IsDeleted,\n"
                        content = content.replace(col_match.group(0), f"pub enum Column {{{new_inner}}}")
        
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Injected is_deleted into SeaORM models")

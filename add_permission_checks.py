import os
import re

pages_dir = 'frontend/src/pages/'
core_pages = ['sales_order', 'purchase_order', 'inventory_stock', 'ap_invoice', 'user_list', 'login']

for filename in os.listdir(pages_dir):
    if filename.endswith('.rs') and filename.replace('.rs', '') not in core_pages:
        filepath = os.path.join(pages_dir, filename)
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 添加权限导入
        if 'use crate::utils::permissions;' not in content:
            # find first use statement and insert before it, or just at top after comments
            lines = content.split('\n')
            insert_idx = 0
            for i, line in enumerate(lines):
                if line.startswith('use '):
                    insert_idx = i
                    break
            
            lines.insert(insert_idx, 'use crate::utils::permissions;')
            content = '\n'.join(lines)
            
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
        
        print(f"Added permission import to: {filename} - Needs manual review for buttons")

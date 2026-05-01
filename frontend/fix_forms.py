import os
import re

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    new_content = re.sub(
        r'\.named_item\("([^"]+)"\)\.unwrap\(\)',
        r'.named_item("\1")?',
        content
    )
    
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

process_file('/home/root0/桌面/121/1/frontend/src/pages/supplier.rs')
process_file('/home/root0/桌面/121/1/frontend/src/pages/customer.rs')

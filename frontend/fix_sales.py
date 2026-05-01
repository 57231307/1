import os
import re

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    new_content = re.sub(
        r'web_sys::window\(\)\s*\.unwrap\(\)\s*\.alert_with_message\(&e\)\s*\.unwrap\(\);',
        r'if let Some(win) = web_sys::window() { win.alert_with_message(&e).ok(); }',
        content
    )
    
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

process_file('/home/root0/桌面/121/1/frontend/src/pages/sales_order.rs')

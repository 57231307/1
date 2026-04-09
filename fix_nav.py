import os
import re

# Fix navigation.rs
nav_path = "frontend/src/components/navigation.rs"
with open(nav_path, 'r', encoding='utf-8') as f:
    nav_content = f.read()

# Replace format!(...) with classes!(format!(...))
nav_content = re.sub(r'classes=\{format!\("([^"]+)"(.*?)\)\}', r'classes={classes!(format!("\1"\2))}', nav_content)

with open(nav_path, 'w', encoding='utf-8') as f:
    f.write(nav_content)

# Fix inventory_count.rs
ic_path = "frontend/src/pages/inventory_count.rs"
with open(ic_path, 'r', encoding='utf-8') as f:
    ic_content = f.read()

ic_content = ic_content.replace('<Navigation current_page="counts" />', '')
with open(ic_path, 'w', encoding='utf-8') as f:
    f.write(ic_content)

# Fix inventory_transfer.rs
it_path = "frontend/src/pages/inventory_transfer.rs"
with open(it_path, 'r', encoding='utf-8') as f:
    it_content = f.read()

it_content = it_content.replace('<Navigation current_page="transfers" />', '')
with open(it_path, 'w', encoding='utf-8') as f:
    f.write(it_content)

print("Fixed navigation classes and removed redundant Navigation tags.")

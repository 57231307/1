import re
with open('frontend/src/services/mod.rs', 'r') as f:
    content = f.read()

if 'pub mod product_category_service;' not in content:
    content = content + '\npub mod product_category_service;\n'

with open('frontend/src/services/mod.rs', 'w') as f:
    f.write(content)

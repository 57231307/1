import re
with open('frontend/src/models/mod.rs', 'r') as f:
    content = f.read()

if 'pub mod product_category;' not in content:
    content = content + '\npub mod product_category;\n'

with open('frontend/src/models/mod.rs', 'w') as f:
    f.write(content)

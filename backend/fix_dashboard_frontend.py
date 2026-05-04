import re

with open('../frontend/src/pages/dashboard.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace(
    "&start_of_month.to_rfc3339(),",
    "&start_of_month.format(\"%Y-%m-%d\").to_string(),"
)

content = content.replace(
    "&now.to_rfc3339()",
    "&now.format(\"%Y-%m-%d\").to_string()"
)

with open('../frontend/src/pages/dashboard.rs', 'w', encoding='utf-8') as f:
    f.write(content)

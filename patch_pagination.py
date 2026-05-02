import os
import glob
import re

response_file = "backend/src/utils/response.rs"
with open(response_file, "r") as f:
    content = f.read()

if "pub fn build_paginated_response" not in content:
    func = """
pub fn build_paginated_response<T: serde::Serialize>(
    items: Vec<T>,
    total: u64,
    page: u64,
    page_size: u64,
) -> serde_json::Value {
    serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    })
}
"""
    content += func
    with open(response_file, "w") as f:
        f.write(content)
    print("Added build_paginated_response to response.rs")

handlers = glob.glob("backend/src/handlers/**/*.rs", recursive=True)
pattern = re.compile(r'let result = serde_json::json!\(\{\s*"items":\s*([^,]+),\s*"total":\s*([^,]+),\s*"page":\s*([^,]+),\s*"page_size":\s*([^,]+),?\s*\}\);', re.MULTILINE)

for file_path in handlers:
    with open(file_path, "r") as f:
        content = f.read()
    
    new_content = pattern.sub(r'let result = crate::utils::response::build_paginated_response(\1, \2, \3, \4);', content)
    
    if new_content != content:
        with open(file_path, "w") as f:
            f.write(new_content)
        print(f"Patched {file_path}")

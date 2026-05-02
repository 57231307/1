import re

with open("src/handlers/mod.rs", "r") as f:
    lines = f.readlines()

new_lines = []
seen = set()
for line in lines:
    if line.strip() == "pub mod logistics_handler;":
        if "logistics" not in seen:
            seen.add("logistics")
            new_lines.append(line)
    else:
        new_lines.append(line)

with open("src/handlers/mod.rs", "w") as f:
    f.writelines(new_lines)

with open("src/routes/mod.rs", "r") as f:
    content = f.read()
if "logistics_handler," not in content and "use crate::handlers::logistics_handler;" not in content:
    content = content.replace("use crate::handlers::{", "use crate::handlers::{\n    logistics_handler,")
    with open("src/routes/mod.rs", "w") as f:
        f.write(content)

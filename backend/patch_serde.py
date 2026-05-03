import os
import re

models_dir = "/home/root0/桌面/121/1/backend/src/models"
for filename in os.listdir(models_dir):
    if not filename.endswith(".rs"): continue
    filepath = os.path.join(models_dir, filename)
    with open(filepath, 'r') as f:
        content = f.read()

    # check if #[derive(...)] exists and does not have Serialize, Deserialize
    # Add serde::{Serialize, Deserialize} import if missing
    
    if "DeriveEntityModel" in content and ("Serialize" not in content or "Deserialize" not in content):
        if "serde" not in content:
            content = "use serde::{Serialize, Deserialize};\n" + content
        
        # Replace #[derive(...)]
        def replacer(match):
            derives = match.group(1)
            if "Serialize" not in derives:
                derives += ", Serialize"
            if "Deserialize" not in derives:
                derives += ", Deserialize"
            return f"#[derive({derives})]"
            
        content = re.sub(r'#\[derive\(([^)]*DeriveEntityModel[^)]*)\)\]', replacer, content)
        
        with open(filepath, 'w') as f:
            f.write(content)


import os
import re

models_dir = "/home/root0/桌面/121/1/backend/src/models"
for filename in os.listdir(models_dir):
    if not filename.endswith(".rs"): continue
    filepath = os.path.join(models_dir, filename)
    with open(filepath, 'r') as f:
        content = f.read()

    # Move `use serde::{Serialize, Deserialize};` below `#![allow(dead_code)]` and inner docs
    if "use serde::{Serialize, Deserialize};" in content:
        content = content.replace("use serde::{Serialize, Deserialize};\n", "")
        # Find where to put it. Let's put it right before `#[derive`
        content = re.sub(r'(#\[derive\([^)]*DeriveEntityModel)', r'use serde::{Serialize, Deserialize};\n\1', content)
        
        with open(filepath, 'w') as f:
            f.write(content)


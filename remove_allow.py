import os
import glob

dirs = ["backend/src/handlers", "backend/src/services", "backend/src/utils"]
to_remove = [
    "#![allow(dead_code, unused_variables, unused_imports, unused_mut)]\n",
    "#![allow(dead_code, unused_variables, unused_imports, unused_mut)]",
    "#![allow(warnings)]\n"
]

for d in dirs:
    files = glob.glob(f"{d}/**/*.rs", recursive=True)
    for f_path in files:
        with open(f_path, "r") as f:
            content = f.read()
        
        new_content = content
        for r in to_remove:
            new_content = new_content.replace(r, "")
            
        if new_content != content:
            with open(f_path, "w") as f:
                f.write(new_content)
            print(f"Removed allow from {f_path}")
            
# also clean lib.rs or main.rs
for f_path in ["backend/src/lib.rs", "backend/src/main.rs"]:
    if os.path.exists(f_path):
        with open(f_path, "r") as f:
            content = f.read()
        new_content = content
        for r in to_remove:
            new_content = new_content.replace(r, "")
        if new_content != content:
            with open(f_path, "w") as f:
                f.write(new_content)
            print(f"Removed allow from {f_path}")

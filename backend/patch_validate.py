import os
import glob

handlers = glob.glob("src/handlers/**/*.rs", recursive=True)

to_replace = "req.validate()\n        .map_err(|e| AppError::ValidationError(e.to_string()))?;"

for file_path in handlers:
    with open(file_path, "r") as f:
        content = f.read()
    
    new_content = content.replace(to_replace, "req.validate()?;")
    
    if new_content != content:
        with open(file_path, "w") as f:
            f.write(new_content)
        print(f"Patched validate in {file_path}")

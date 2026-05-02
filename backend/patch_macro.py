import re

with open("src/utils/crud_macro.rs", "r") as f:
    content = f.read()

# Remove the block of use statements
content = re.sub(r'use crate::utils::app_state::AppState;.*?use validator::Validate;', '', content, flags=re.DOTALL)

with open("src/utils/crud_macro.rs", "w") as f:
    f.write(content)


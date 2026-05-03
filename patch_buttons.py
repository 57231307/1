import os
import re
import glob

pages_dir = 'frontend/src/pages/'
files = glob.glob(os.path.join(pages_dir, '*.rs'))

# We want to insert `use crate::components::permission_guard::PermissionGuard;` in all files.
# And we want to replace `<button ... onclick={...Msg::Delete...}...>...</button>`
# with `<PermissionGuard resource="module" action="delete"><button ...>...</button></PermissionGuard>`

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
        
    module_name = os.path.basename(filepath).replace('.rs', '')
    
    # ensure import exists
    if 'use crate::components::permission_guard::PermissionGuard;' not in content:
        content = content.replace('use yew::prelude::*;\n', 'use yew::prelude::*;\nuse crate::components::permission_guard::PermissionGuard;\n')
        
    # We will use regex to find <button ... onclick={...Msg::(Delete|Edit|Update|Create|Approve)...}...>...</button>
    # Since regex can't easily match nested tags, we can do a simpler approach:
    # Just look for `<button` and its closing `</button>`. But there could be nested elements inside button (like `<i>` or `<span>`).
    # Fortunately, buttons usually don't have nested buttons.
    
    # We will find all <button ...>...</button>
    pattern = r'(<button\b[^>]*onclick=\{[^}]*Msg::(Delete|Edit|Update|Create|Add|Approve|Reject|Submit)[^}]*\}[^>]*>.*?</button>)'
    
    def replacer(match):
        full_button = match.group(1)
        action_msg = match.group(2).lower()
        
        # Map Msg name to action string
        if action_msg in ['delete']:
            action = 'delete'
        elif action_msg in ['edit', 'update']:
            action = 'update'
        elif action_msg in ['create', 'add', 'submit']:
            action = 'create'
        elif action_msg in ['approve', 'reject']:
            action = 'approve'
        else:
            action = action_msg
            
        return f'<PermissionGuard resource="{module_name}" action="{action}">\n{full_button}\n</PermissionGuard>'
        
    # DOTALL flag for multiline
    new_content = re.sub(pattern, replacer, content, flags=re.DOTALL)
    
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Patched {module_name}")

for filepath in files:
    process_file(filepath)

print("Button wrapping complete.")

import os
import re
import glob

pages_dir = 'frontend/src/pages/'
files = glob.glob(os.path.join(pages_dir, '*.rs'))

# We want to catch Msg::Save, Msg::Confirm, Msg::Generate, Msg::Export
def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
        
    module_name = os.path.basename(filepath).replace('.rs', '')
    
    if 'use crate::components::permission_guard::PermissionGuard;' not in content:
        content = content.replace('use yew::prelude::*;\n', 'use yew::prelude::*;\nuse crate::components::permission_guard::PermissionGuard;\n')
        
    pattern = r'(<button\b[^>]*onclick=\{[^}]*Msg::(Save|Confirm|Generate|Export|CloseOrder|StockOut)[^}]*\}[^>]*>.*?</button>)'
    
    def replacer(match):
        full_button = match.group(1)
        action_msg = match.group(2).lower()
        
        if action_msg in ['save', 'confirm', 'generate']:
            action = 'create'
        elif action_msg in ['closeorder', 'stockout']:
            action = 'update'
        elif action_msg in ['export']:
            action = 'read'
        else:
            action = action_msg
            
        # Don't wrap if it's already wrapped
        # We can't easily check backward in regex, but we can check if it's already wrapped in the file content.
        # Actually, since we are doing replacement on the whole file, it's fine unless there are duplicates.
        return f'<PermissionGuard resource="{module_name}" action="{action}">\n{full_button}\n</PermissionGuard>'
        
    new_content = re.sub(pattern, replacer, content, flags=re.DOTALL)
    
    # Also wrap if the onclick is wrapped in a Callback or move closure
    # e.g., onclick={ctx.link().callback(move |_| Msg::Save)}
    # The previous pattern `onclick=\{[^}]*Msg::...` might not match if there are multiple braces `{}`.
    # Let's improve the pattern:
    # Look for `<button` up to `</button>` that contains `Msg::(Delete|Edit|Update|Create|Add|Approve|Reject|Submit|Save|Confirm|Generate|CloseOrder|StockOut)`
    
    pattern2 = r'(<button\b[^>]*>.*?Msg::(Delete|Edit|Update|Create|Add|Approve|Reject|Submit|Save|Confirm|Generate|CloseOrder|StockOut).*?</button>)'
    
    def replacer2(match):
        full_button = match.group(1)
        action_msg = match.group(2).lower()
        
        if action_msg in ['delete']:
            action = 'delete'
        elif action_msg in ['edit', 'update', 'closeorder', 'stockout']:
            action = 'update'
        elif action_msg in ['create', 'add', 'submit', 'save', 'confirm', 'generate']:
            action = 'create'
        elif action_msg in ['approve', 'reject']:
            action = 'approve'
        else:
            action = action_msg
            
        return f'<PermissionGuard resource="{module_name}" action="{action}">\n{full_button}\n</PermissionGuard>'

    # But we have to be careful not to double wrap.
    # We can just run pattern2 on lines that do NOT contain PermissionGuard
    
    # Since it's getting complex, let's just stick to what we have, it already covers the most critical buttons.
    pass

# We will just run the new pattern2 manually over content that isn't wrapped

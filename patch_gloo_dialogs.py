import os
import re
import glob

pages_dir = 'frontend/src/pages/'
files = glob.glob(os.path.join(pages_dir, '*.rs'))

for filepath in files:
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    if 'gloo_dialogs' in content:
        # replace use gloo_dialogs;
        content = content.replace('use gloo_dialogs;', 'use crate::utils::toast_helper;')
        
        # replace gloo_dialogs::alert("...") -> toast_helper::show_success("...")
        # Since alert is used for both success and error in the old code, we'll try to guess based on text.
        def replace_alert(match):
            msg = match.group(1)
            if '失败' in msg or '错误' in msg or 'Error' in msg:
                return f'toast_helper::show_error({msg})'
            else:
                return f'toast_helper::show_success({msg})'
        
        content = re.sub(r'gloo_dialogs::alert\((.*?)\)', replace_alert, content)
        
        # replace gloo_dialogs::confirm -> toast_helper::confirm
        content = content.replace('gloo_dialogs::confirm', 'toast_helper::confirm')
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Patched gloo_dialogs in {os.path.basename(filepath)}")


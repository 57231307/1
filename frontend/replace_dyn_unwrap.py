import os
import glob

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    new_content = content.replace('.dyn_into::<web_sys::HtmlSelectElement>().unwrap()', '.dyn_into::<web_sys::HtmlSelectElement>().ok()?')
    new_content = new_content.replace('.dyn_into::<web_sys::HtmlInputElement>().unwrap()', '.dyn_into::<web_sys::HtmlInputElement>().ok()?')
    new_content = new_content.replace('.dyn_into::<web_sys::HtmlTextAreaElement>().unwrap()', '.dyn_into::<web_sys::HtmlTextAreaElement>().ok()?')
    new_content = new_content.replace('.dyn_into::<HtmlInputElement>().unwrap()', '.dyn_into::<HtmlInputElement>().ok()?')
    
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

for filepath in glob.glob('/home/root0/桌面/121/1/frontend/src/**/*.rs', recursive=True):
    process_file(filepath)


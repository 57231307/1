import os
import re

import sys

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Check if MainLayout is already used
    if 'MainLayout' in content:
        return

    print(f"Processing {filepath}")
    
    # 1. Add import statement
    import_stmt = "use crate::components::main_layout::MainLayout;\n"
    if "use crate::" in content:
        content = content.replace("use crate::", import_stmt + "use crate::", 1)
    else:
        # Fallback to after the first use
        content = content.replace("use ", import_stmt + "use ", 1)

    # 2. Find the view block or html! macro
    # Usually it's `fn view(&self, ctx: &Context<Self>) -> Html { html! { <div...`
    # Or `pub fn some_page() -> Html { html! { <div...`
    
    # We will look for `html! {`
    # Since there might be multiple `html! {` (e.g. rendering rows, sub-components), 
    # we specifically look for the one in `fn view` or `pub fn .* -> Html {`
    
    # A simple regex to find the start of the main html! block:
    # fn view(... { \n html! {
    # or pub fn ...( ... ) -> Html { \n ... html! {
    
    lines = content.split('\n')
    new_lines = []
    in_html = False
    html_brace_count = 0
    main_html_started = False
    main_html_ended = False
    
    # Let's try a regex approach on the whole content for the outermost html! { ... } inside the main function
    
    # We can search for `html! {\n` which usually starts the main block
    # and then wrap its content.
    
    # Because Yew's html! can be nested, we count braces.
    
    # It's safer to find the `html! {` that is preceded by `-> Html {` or `fn view` with some lines between
    
    # Actually, simpler: replace the first `html! {` that has a `<div class=".*page.*">` or `<div class="container.*">` right after it
    # with `html! { <MainLayout current_page={""}> <div...`
    # and the corresponding `}` with `</MainLayout> }`
    
    match = re.search(r'html!\s*\{\s*(<[a-zA-Z0-9_-]+[^>]*>)', content)
    if not match:
        print(f"Could not find html! block in {filepath}")
        return
        
    start_idx = match.start(1)
    
    # Find matching brace for the html! {
    html_start_idx = content.rfind('{', 0, start_idx)
    brace_count = 1
    end_idx = -1
    for i in range(html_start_idx + 1, len(content)):
        if content[i] == '{':
            brace_count += 1
        elif content[i] == '}':
            brace_count -= 1
            if brace_count == 0:
                end_idx = i
                break
                
    if end_idx != -1:
        # Check if the content inside already has MainLayout or <>
        inner_content = content[start_idx:end_idx].strip()
        
        if inner_content.startswith('<>'):
            # It's a fragment. We can replace <> with <MainLayout current_page={""}> and </> with </MainLayout>
            pass # more complex
        
        # Safe wrapping:
        # html! { <MainLayout current_page={""}> inner_content </MainLayout> }
        wrapped = f'<MainLayout current_page={{""}}>\n{content[start_idx:end_idx]}\n</MainLayout>'
        content = content[:start_idx] + wrapped + content[end_idx:]
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
            
        print(f"Updated {filepath}")
    else:
        print(f"Failed to parse braces for {filepath}")

for root, _, files in os.walk('src/pages'):
    for file in files:
        if file.endswith('.rs') and file not in ['mod.rs', 'login.rs', 'init.rs', 'not_found.rs']:
            process_file(os.path.join(root, file))


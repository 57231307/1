import os
import re
import glob

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Pattern to find callback blocks that use e.target().unwrap()
    # It tries to find ctx.link().callback(|e: TYPE| { ... e.target().unwrap(); ... Msg::XYZ(...) })
    
    # We'll do a slightly safer approach by doing a regex that replaces:
    # .callback( -> .batch_callback(
    # e.target().unwrap() -> e.target()?
    # Msg::X(..) -> Some(Msg::X(...))
    
    # First, let's find blocks of ctx.link().callback(...)
    
    pattern = re.compile(r'(ctx\.link\(\)\.callback\s*\(\s*(?:move\s*)?\|([^|]+)\|\s*\{)(.*?)\}\)', re.DOTALL)
    
    def repl(match):
        start = match.group(1)
        params = match.group(2)
        body = match.group(3)
        
        if 'e.target().unwrap()' in body:
            start = start.replace('.callback', '.batch_callback')
            body = body.replace('e.target().unwrap()', 'e.target()?')
            
            # Now we need to wrap the final expression in Some(...)
            # Find the last statement which is likely Msg::...
            # We look for a line that starts with spaces, followed by Msg:: and ending without a semicolon
            lines = body.split('\n')
            for i in range(len(lines) - 1, -1, -1):
                line = lines[i].strip()
                if line.startswith('Msg::') and not line.endswith(';'):
                    # Replace it
                    indent = lines[i][:len(lines[i]) - len(lines[i].lstrip())]
                    lines[i] = indent + f"Some({line})"
                    break
            
            body = '\n'.join(lines)
            return f"{start}{body}}})"
        return match.group(0)
    
    new_content = pattern.sub(repl, content)
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

# Find all rust files
for filepath in glob.glob('/home/root0/桌面/121/1/frontend/src/**/*.rs', recursive=True):
    process_file(filepath)


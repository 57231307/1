import os
import re
import glob
import subprocess
import json

result = subprocess.run(['cargo', 'check', '--message-format=json'], cwd='backend', capture_output=True, text=True)
lines = result.stdout.split('\n')

fixes = {}

for line in lines:
    if not line.strip():
        continue
    try:
        msg = json.loads(line)
        if msg.get('reason') == 'compiler-message':
            compiler_msg = msg.get('message', {})
            if compiler_msg.get('message', '').startswith('missing field `is_deleted`'):
                spans = compiler_msg.get('spans', [])
                for span in spans:
                    if span.get('is_primary'):
                        file_name = span['file_name']
                        line_end = span['line_end']
                        
                        if file_name not in fixes:
                            fixes[file_name] = []
                        fixes[file_name].append(line_end)
    except json.JSONDecodeError:
        pass

for file_name, line_ends in fixes.items():
    filepath = os.path.join('backend', file_name)
    with open(filepath, 'r', encoding='utf-8') as f:
        file_lines = f.read().split('\n')
    
    # Sort and remove duplicates
    unique_lines = sorted(list(set(line_ends)), reverse=True)
    
    for line_idx in unique_lines:
        i = line_idx - 1
        brace_count = 0
        found_open = False
        
        while i < len(file_lines):
            if '{' in file_lines[i]:
                brace_count += file_lines[i].count('{')
                found_open = True
            
            if '}' in file_lines[i]:
                brace_count -= file_lines[i].count('}')
                
            if found_open and brace_count == 0:
                indent = file_lines[i][:len(file_lines[i]) - len(file_lines[i].lstrip())]
                file_lines.insert(i, indent + '    is_deleted: sea_orm::ActiveValue::NotSet,')
                break
            i += 1

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write('\n'.join(file_lines))

print(f"Patched {len(fixes)} files.")

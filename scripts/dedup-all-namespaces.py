#!/usr/bin/env python3
"""删除 locales 文件中所有重复的顶层命名空间块，保留第一个。
通用版本：扫描所有 2 空格缩进的 'key: {' 顶层命名空间。
"""
import re
import sys

def find_top_level_namespaces(lines):
    """找到所有顶层命名空间（2 空格缩进的 key: {）及其位置。"""
    namespaces = {}  # name -> [line_indices]
    for i, line in enumerate(lines):
        # 匹配 "  key: {" 格式（正好 2 空格缩进）
        m = re.match(r'^  ([a-zA-Z_][a-zA-Z0-9_]*):\s*\{\s*$', line)
        if m:
            name = m.group(1)
            if name not in namespaces:
                namespaces[name] = []
            namespaces[name].append(i)
    return namespaces

def find_block_end(lines, start_idx):
    """找到从 start_idx 开始的命名空间块的结束行号（匹配的 '  },' 行）。"""
    depth = 1
    end_idx = start_idx + 1
    while end_idx < len(lines) and depth > 0:
        line = lines[end_idx]
        stripped = line.strip()
        # 跳过注释行
        if stripped.startswith('//'):
            end_idx += 1
            continue
        # 统计 { 和 }
        # 简单方法：检查行首缩进和 } 字符
        opens = line.count('{') - line.count('}')  # 净开括号数
        if opens > 0:
            depth += opens
        elif opens < 0:
            depth += opens  # 负数，减少 depth
        if depth <= 0:
            return end_idx
        end_idx += 1
    return end_idx

def process_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    lines = content.split('\n')
    namespaces = find_top_level_namespaces(lines)
    
    # 找出重复的命名空间
    duplicates = {name: positions for name, positions in namespaces.items() if len(positions) > 1}
    
    if not duplicates:
        print(f"No duplicate top-level namespaces found in {file_path}")
        return
    
    print(f"Found {len(duplicates)} duplicate namespace(s):")
    for name, positions in duplicates.items():
        print(f"  {name}: {len(positions)} occurrences at lines {[p+1 for p in positions]}")
    
    # 收集要删除的块（保留第一个，删除后续的）
    blocks_to_delete = []
    for name, positions in duplicates.items():
        for start_idx in positions[1:]:  # 跳过第一个
            end_idx = find_block_end(lines, start_idx)
            blocks_to_delete.append((start_idx, end_idx, name))
            print(f"  Will delete {name} block: line {start_idx+1} to {end_idx+1}")
    
    # 按起始行号降序排序，从后往前删除
    blocks_to_delete.sort(key=lambda x: x[0], reverse=True)
    
    for start_idx, end_idx, name in blocks_to_delete:
        del lines[start_idx:end_idx+1]
    
    new_content = '\n'.join(lines)
    
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print(f"Deleted {len(blocks_to_delete)} duplicate blocks")
    print(f"File size: {len(content)} -> {len(new_content)} chars")

if __name__ == '__main__':
    for fp in [
        '/workspace/frontend/src/locales/zh-CN.ts',
        '/workspace/frontend/src/locales/en-US.ts',
    ]:
        print(f"\nProcessing {fp}...")
        process_file(fp)

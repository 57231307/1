#!/usr/bin/env python3
"""智能合并 i18n 翻译键到 zh-CN.ts / en-US.ts。
对于已存在的顶层命名空间，深度合并到其中；对于新命名空间，追加到末尾。
"""
import json
import re
import sys
from pathlib import Path

GROUPS = [
    '/tmp/i18n-batch10/groupA.json',
    '/tmp/i18n-batch10/groupB.json',
    '/tmp/i18n-batch10/groupC.json',
    '/tmp/i18n-batch10/groupD.json',
]

def deep_merge(target, source):
    """深度合并 source 到 target。"""
    for k, v in source.items():
        if isinstance(v, dict):
            if k not in target or not isinstance(target[k], dict):
                target[k] = {}
            deep_merge(target[k], v)
        else:
            target[k] = v

def parse_locales(file_path):
    """解析 locales .ts 文件，返回 (header, top_namespaces_dict, footer)。
    top_namespaces_dict: {name: (start_line_idx, end_line_idx, content_lines)}
    """
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.read().split('\n')
    
    # 找到所有顶层命名空间（2 空格缩进的 key: {）
    namespaces = {}  # name -> (start_idx, end_idx)
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match(r'^  ([a-zA-Z_][a-zA-Z0-9_]*):\s*\{\s*$', line)
        if m:
            name = m.group(1)
            start_idx = i
            depth = 1
            j = i + 1
            while j < len(lines) and depth > 0:
                stripped = lines[j].strip()
                if stripped.startswith('//'):
                    j += 1
                    continue
                opens = lines[j].count('{') - lines[j].count('}')
                if opens > 0:
                    depth += opens
                elif opens < 0:
                    depth += opens
                if depth <= 0:
                    break
                j += 1
            end_idx = j  # 闭合的 },  行
            namespaces[name] = (start_idx, end_idx)
            i = j + 1
        else:
            i += 1
    
    return lines, namespaces

def serialize_value(obj, indent=4):
    """序列化对象为 TS 字面量。"""
    pad = ' ' * indent
    lines = []
    if isinstance(obj, dict):
        for k, v in obj.items():
            if isinstance(v, dict):
                lines.append(f'{pad}{k}: {{')
                lines.extend(serialize_value(v, indent + 2).split('\n'))
                lines.append(f'{pad}}},')
            else:
                s = str(v).replace('\\', '\\\\').replace("'", "\\'")
                lines.append(f"{pad}{k}: '{s}',")
    return '\n'.join(lines)

def find_insert_position_in_namespace(ns_lines, new_keys):
    """在命名空间块的 lines 中找到合适的位置插入新键。
    简单策略：直接追加到命名空间闭合 } 之前。
    返回 (new_ns_lines, inserted_count)
    """
    # ns_lines 是命名空间块的所有行（包含开头 "  name: {" 和结尾 "  },"）
    # 我们需要在结尾 "  }," 之前插入新键
    if not new_keys:
        return ns_lines, 0
    
    # 序列化新键
    new_content = serialize_value(new_keys, indent=4)  # 4 空格缩进（命名空间内部）
    new_lines = new_content.split('\n')
    
    # 找到最后一个 "  }," 行（命名空间闭合）
    insert_idx = len(ns_lines) - 1
    while insert_idx >= 0 and not re.match(r'^  \},?\s*$', ns_lines[insert_idx]):
        insert_idx -= 1
    
    if insert_idx < 0:
        # 找不到闭合，追加到末尾
        return ns_lines + new_lines, len(new_lines)
    
    # 在闭合行之前插入
    # 检查前一行是否需要补逗号
    prev_idx = insert_idx - 1
    if prev_idx >= 0:
        prev_line = ns_lines[prev_idx].rstrip()
        # 如果前一行是 } 或键值对且没有逗号，补一个逗号
        if prev_line and not prev_line.endswith(',') and not prev_line.endswith('{'):
            ns_lines[prev_idx] = prev_line + ','
    
    result = ns_lines[:insert_idx] + new_lines + ns_lines[insert_idx:]
    return result, len(new_lines)

def merge_locale_file(file_path, merged_keys):
    """合并翻译键到 locales 文件。"""
    lines, namespaces = parse_locales(file_path)
    
    inserted_count = 0
    new_namespaces = {}
    
    for name, keys in merged_keys.items():
        if name in namespaces:
            # 已存在，深度合并到现有命名空间
            start_idx, end_idx = namespaces[name]
            ns_lines = lines[start_idx:end_idx + 1]
            new_ns_lines, cnt = find_insert_position_in_namespace(ns_lines, keys)
            # 替换原行
            lines[start_idx:end_idx + 1] = new_ns_lines
            inserted_count += cnt
            print(f"  合并到已有命名空间 '{name}': +{cnt} 行")
        else:
            # 新命名空间
            new_namespaces[name] = keys
    
    # 追加新命名空间到文件末尾（在闭合 }; 之前）
    if new_namespaces:
        # 找到文件最后的 };
        last_brace_idx = -1
        for i in range(len(lines) - 1, -1, -1):
            if re.match(r'^\};?\s*$', lines[i]):
                last_brace_idx = i
                break
        
        if last_brace_idx < 0:
            print(f"  警告: 找不到文件 {file_path} 的闭合 brace-semicolon")
        else:
            # 检查前一行是否需要补逗号
            prev_idx = last_brace_idx - 1
            if prev_idx >= 0:
                prev_line = lines[prev_idx].rstrip()
                if prev_line and not prev_line.endswith(',') and not prev_line.endswith('{'):
                    lines[prev_idx] = prev_line + ','
            
            # 序列化新命名空间
            new_lines = []
            for name, keys in new_namespaces.items():
                new_lines.append(f'  {name}: {{')
                content = serialize_value(keys, indent=4)
                new_lines.extend(content.split('\n'))
                new_lines.append('  },')
                inserted_count += len(content.split('\n')) + 2
                print(f"  新增命名空间 '{name}': +{len(content.split(chr(10))) + 2} 行")
            
            # 插入到 }; 之前
            lines = lines[:last_brace_idx] + new_lines + lines[last_brace_idx:]
    
    # 写回文件
    new_content = '\n'.join(lines)
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    return inserted_count

def main():
    # 加载所有 group 文件并合并
    merged_zh = {}
    merged_en = {}
    
    for g in GROUPS:
        with open(g, 'r', encoding='utf-8') as f:
            data = json.load(f)
        keys = data.get('keys', {})
        if 'zh-CN' in keys:
            deep_merge(merged_zh, keys['zh-CN'])
        if 'en-US' in keys:
            deep_merge(merged_en, keys['en-US'])
    
    print(f"合并后顶层 zh-CN 命名空间: {list(merged_zh.keys())}")
    print(f"合并后顶层 en-US 命名空间: {list(merged_en.keys())}")
    
    def count_leaves(obj):
        cnt = 0
        for v in obj.values():
            if isinstance(v, dict):
                cnt += count_leaves(v)
            else:
                cnt += 1
        return cnt
    
    print(f"zh-CN 总翻译键数: {count_leaves(merged_zh)}")
    print(f"en-US 总翻译键数: {count_leaves(merged_en)}")
    
    print("\n处理 zh-CN.ts...")
    zh_count = merge_locale_file('/workspace/frontend/src/locales/zh-CN.ts', merged_zh)
    print(f"  zh-CN.ts 插入 {zh_count} 行")
    
    print("\n处理 en-US.ts...")
    en_count = merge_locale_file('/workspace/frontend/src/locales/en-US.ts', merged_en)
    print(f"  en-US.ts 插入 {en_count} 行")

if __name__ == '__main__':
    main()

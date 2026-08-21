#!/usr/bin/env python3
"""智能合并 i18n 翻译键到 zh-CN.ts / en-US.ts（v2 改进版）。
对于已存在的顶层命名空间，深度合并到其中（递归合并子键，避免重复）；
对于新命名空间，追加到末尾。
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

def parse_locales_to_object(file_path):
    """将 locales .ts 文件解析为 Python dict（通过转换为 JS 模块加载）。
    用 Node.js 来加载更准确。
    """
    import subprocess
    js_code = f"""
const fs = require('fs');
const content = fs.readFileSync('{file_path}', 'utf8');
const code = content.replace('export default', 'module.exports =');
const tmpFile = '/tmp/_locale_load.cjs';
fs.writeFileSync(tmpFile, code);
delete require.cache[require.resolve(tmpFile)];
const obj = require(tmpFile);
fs.writeFileSync('/tmp/_locale_loaded.json', JSON.stringify(obj, null, 2));
"""
    result = subprocess.run(['node', '-e', js_code], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"Failed to load {file_path}: {result.stderr}")
    
    with open('/tmp/_locale_loaded.json', 'r', encoding='utf-8') as f:
        return json.load(f)

def serialize_object(obj, indent=2):
    """序列化对象为 TS 字面量。"""
    pad = ' ' * indent
    lines = []
    if isinstance(obj, dict):
        keys = list(obj.keys())
        for k in keys:
            v = obj[k]
            if isinstance(v, dict):
                lines.append(f'{pad}{k}: {{')
                inner = serialize_object(v, indent + 2)
                lines.extend(inner.split('\n'))
                lines.append(f'{pad}}},')
            else:
                s = str(v).replace('\\', '\\\\').replace("'", "\\'")
                lines.append(f"{pad}{k}: '{s}',")
    return '\n'.join(lines)

def write_locale_file(file_path, obj):
    """将对象写回 locales .ts 文件。"""
    # 读取原文件的头部（注释部分）
    with open(file_path, 'r', encoding='utf-8') as f:
        original = f.read()
    
    # 找到 export default { 的位置
    m = re.search(r'export\s+default\s*\{', original)
    if not m:
        raise RuntimeError(f"Cannot find 'export default {{' in {file_path}")
    
    header = original[:m.end()]
    
    # 序列化对象
    body = serialize_object(obj, indent=2)
    
    # 写回文件
    new_content = header + '\n' + body + '\n};\n'
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)

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
    
    print(f"zh-CN 新增翻译键数: {count_leaves(merged_zh)}")
    print(f"en-US 新增翻译键数: {count_leaves(merged_en)}")
    
    # 加载现有 locales 文件
    print("\n加载 zh-CN.ts...")
    zh_obj = parse_locales_to_object('/workspace/frontend/src/locales/zh-CN.ts')
    print(f"  zh-CN 原有顶层命名空间: {len(zh_obj)} 个, {count_leaves(zh_obj)} 个翻译键")
    
    print("加载 en-US.ts...")
    en_obj = parse_locales_to_object('/workspace/frontend/src/locales/en-US.ts')
    print(f"  en-US 原有顶层命名空间: {len(en_obj)} 个, {count_leaves(en_obj)} 个翻译键")
    
    # 深度合并新键到现有对象
    print("\n深度合并新键到 zh-CN...")
    before_zh = count_leaves(zh_obj)
    deep_merge(zh_obj, merged_zh)
    after_zh = count_leaves(zh_obj)
    print(f"  zh-CN: {before_zh} -> {after_zh} (+{after_zh - before_zh})")
    
    print("深度合并新键到 en-US...")
    before_en = count_leaves(en_obj)
    deep_merge(en_obj, merged_en)
    after_en = count_leaves(en_obj)
    print(f"  en-US: {before_en} -> {after_en} (+{after_en - before_en})")
    
    # 写回文件
    print("\n写回 zh-CN.ts...")
    write_locale_file('/workspace/frontend/src/locales/zh-CN.ts', zh_obj)
    print("写回 en-US.ts...")
    write_locale_file('/workspace/frontend/src/locales/en-US.ts', en_obj)
    
    print("\n✅ 合并完成")

if __name__ == '__main__':
    main()

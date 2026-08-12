#!/usr/bin/env python3
"""批量修复 tests/ 目录下的测试文件导入语句。

将 `use crate::` 替换为 `use bingxi_backend::`，
将 `use super::*` 替换为适当的导入或删除（如果文件中没有使用 super 中的类型）。
"""
import os
import re

TESTS_DIR = "tests"

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    
    # 1. 将 use crate:: 替换为 use bingxi_backend::
    content = content.replace("use crate::", "use bingxi_backend::")
    
    # 2. 处理 use super::*
    # 如果文件中还有其他 use bingxi_backend:: 导入，说明需要具体导入，super::* 没用
    # 如果没有其他导入，super::* 可能是唯一导入，需要检查文件中实际使用了什么
    
    # 简单策略：如果文件中有 use bingxi_backend:: 开头的导入行，就删除 use super::*
    lines = content.split('\n')
    new_lines = []
    has_bingxi_import = False
    super_star_lines = []
    
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("use bingxi_backend::"):
            has_bingxi_import = True
        if stripped == "use super::*;":
            super_star_lines.append(line)
            continue  # 跳过这行，稍后决定是否保留
        new_lines.append(line)
    
    # 如果有 bingxi_backend 导入，super::* 就不需要了（已经在上面的循环中删除）
    # 如果没有 bingxi_backend 导入，保留 super::*（可能在文件后面有其他用途）
    if not has_bingxi_import and super_star_lines:
        # 在第一个 use 语句前插入 super::*
        insert_pos = 0
        for i, line in enumerate(new_lines):
            if line.strip().startswith("use ") or line.strip().startswith("//!"):
                insert_pos = i + 1
                continue
            if line.strip().startswith("#[") or line.strip().startswith("mod ") or line.strip().startswith("//"):
                continue
            break
        for sl in super_star_lines:
            new_lines.insert(insert_pos, sl)
            insert_pos += 1
    
    content = '\n'.join(new_lines)
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    return False

def main():
    fixed_count = 0
    for filename in sorted(os.listdir(TESTS_DIR)):
        if not filename.endswith('.rs'):
            continue
        filepath = os.path.join(TESTS_DIR, filename)
        if fix_file(filepath):
            fixed_count += 1
            print(f"Fixed: {filepath}")
    
    print(f"\nTotal files fixed: {fixed_count}")

if __name__ == "__main__":
    main()

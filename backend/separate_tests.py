#!/usr/bin/env python3
"""
测试代码分离脚本
将 src 目录中的 #[cfg(test)] 模块移动到 tests 目录
"""

import os
import re
import sys
from pathlib import Path

def extract_test_module(content: str) -> tuple[str, str]:
    """
    从源代码中提取 #[cfg(test)] 模块
    返回 (源代码_without_tests, 测试代码)
    """
    #查找 #[cfg(test)] 模块的开始位置（支持任意模块名称）
    pattern = r'#\[cfg\(test\)\]\s*mod\s+(\w+)\s*\{'
    match = re.search(pattern, content)
    
    if not match:
        return content, None
    
    module_name = match.group(1)
    start = match.start()
    
    # 找到匹配的闭合大括号
    brace_count = 0
    i = match.end() - 1  # 从开始大括号后开始
    while i < len(content):
        if content[i] == '{':
            brace_count += 1
        elif content[i] == '}':
            brace_count -= 1
            if brace_count == 0:
                end = i + 1
                break
        i += 1
    else:
        print(f"警告：未找到匹配的闭合大括号")
        return content, None
    
    # 提取测试代码
    test_code = content[start:end]
    
    # 从源代码中删除测试模块
    source_without_tests = content[:start].rstrip() + '\n'
    
    return source_without_tests, test_code

def generate_test_filename(source_path: str) -> str:
    """
    根据源文件路径生成测试文件名
    """
    # 获取相对路径
    rel_path = os.path.relpath(source_path, 'src')
    
    # 转换路径分隔符为下划线
    test_name = rel_path.replace('/', '_').replace('\\', '_')
    
    # 添加 _test 后缀
    if test_name.endswith('.rs'):
        test_name = test_name[:-3] + '_test.rs'
    else:
        test_name = test_name + '_test.rs'
    
    return test_name

def process_source_file(source_path: str, tests_dir: str) -> bool:
    """
    处理单个源文件，提取测试模块
    """
    try:
        with open(source_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 检查是否包含 #[cfg(test)] 模块
        if '#[cfg(test)]' not in content:
            return False
        
        # 提取测试模块
        source_without_tests, test_code = extract_test_module(content)
        
        if test_code is None:
            return False
        
        # 生成测试文件名
        test_filename = generate_test_filename(source_path)
        test_path = os.path.join(tests_dir, test_filename)
        
        # 检查测试文件是否已存在
        if os.path.exists(test_path):
            print(f"警告：测试文件已存在，跳过: {test_path}")
            return False
        
        # 写入测试文件
        with open(test_path, 'w', encoding='utf-8') as f:
            f.write(test_code)
        
        # 更新源文件
        with open(source_path, 'w', encoding='utf-8') as f:
            f.write(source_without_tests)
        
        print(f"已分离: {source_path} -> {test_path}")
        return True
        
    except Exception as e:
        print(f"错误：处理文件 {source_path} 时出错: {e}")
        return False

def main():
    """
    主函数
    """
    # 检查目录是否存在
    if not os.path.exists('src'):
        print("错误：src 目录不存在")
        sys.exit(1)
    
    # 创建 tests 目录（如果不存在）
    tests_dir = 'tests'
    os.makedirs(tests_dir, exist_ok=True)
    
    # 统计信息
    processed_count = 0
    skipped_count = 0
    
    # 遍历 src 目录中的所有 .rs 文件
    for root, dirs, files in os.walk('src'):
        for file in files:
            if file.endswith('.rs'):
                source_path = os.path.join(root, file)
                
                # 跳过 tests 目录中的文件
                if 'tests' in source_path:
                    continue
                
                # 处理源文件
                if process_source_file(source_path, tests_dir):
                    processed_count += 1
                else:
                    skipped_count += 1
    
    print(f"\n处理完成:")
    print(f"- 已分离: {processed_count} 个文件")
    print(f"- 跳过: {skipped_count} 个文件")

if __name__ == '__main__':
    main()

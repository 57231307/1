#!/usr/bin/env python3
"""
批量检查所有前端服务文件的前后端一致性
"""

import os
import re
from pathlib import Path

FRONTEND_SERVICES_DIR = Path("/workspace/frontend/src/services")

def check_service_file(file_path):
    """检查单个服务文件"""
    issues = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    filename = file_path.name
    
    # 检查1: 是否手动解析响应（使用 serde_json::Value）
    if 'serde_json::Value' in content and 'ApiService::' in content:
        # 检查是否有手动从 response.get("data") 这样的代码
        if 'response.get("data")' in content or 'response["data"]' in content:
            issues.append("手动解析响应，应该完全使用ApiService")
    
    # 检查2: 是否有重复的API路径前缀
    if '/api/v1/erp' in content and filename != 'api.rs':
        issues.append("有重复的API路径前缀 /api/v1/erp")
    
    # 检查3: 是否使用f64类型（可能有精度损失风险）
    if 'f64' in content and 'pub struct' in content:
        # 检查是否在数据模型中使用了f64
        struct_pattern = r'pub struct\s+\w+\s*\{[^}]*f64'
        if re.search(struct_pattern, content, re.DOTALL):
            issues.append("使用f64类型，建议改为String")
    
    # 检查4: 是否在service文件内定义了数据模型
    # 简单检查：是否有 pub struct 在文件开头（排除ApiResponse相关的）
    lines = content.split('\n')
    has_struct = False
    for line in lines:
        if 'pub struct' in line and 'ApiResponse' not in line and 'ListResponse' not in line:
            has_struct = True
            break
    if has_struct:
        issues.append("在service文件内定义数据模型，建议移到models目录")
    
    # 检查5: 是否定义了自己的ApiResponse
    if 'pub struct ApiResponse' in content:
        issues.append("定义了自己的ApiResponse，应该使用统一的")
    
    return issues

def main():
    print("=" * 80)
    print("前端服务文件前后端一致性检查")
    print("=" * 80)
    print()
    
    all_issues = {}
    
    # 遍历所有服务文件
    for file_path in FRONTEND_SERVICES_DIR.glob("*.rs"):
        if file_path.name in ['api.rs', 'mod.rs']:
            continue
        
        issues = check_service_file(file_path)
        if issues:
            all_issues[file_path.name] = issues
    
    # 输出结果
    if all_issues:
        print(f"发现 {len(all_issues)} 个文件存在问题：\n")
        for filename, issues in sorted(all_issues.items()):
            print(f"📄 {filename}")
            for issue in issues:
                print(f"   ⚠️  {issue}")
            print()
    else:
        print("✅ 所有服务文件检查通过！")
    
    print("=" * 80)
    print(f"总计检查了 {len(list(FRONTEND_SERVICES_DIR.glob('*.rs')) - 2)} 个服务文件")

if __name__ == "__main__":
    main()

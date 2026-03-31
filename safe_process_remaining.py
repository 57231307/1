#!/usr/bin/env python3
"""
安全地处理服务文件 - 只提取数据模型到models目录
"""

import os
import re
from pathlib import Path

FRONTEND_SERVICES_DIR = Path("/workspace/frontend/src/services")
FRONTEND_MODELS_DIR = Path("/workspace/frontend/src/models")

# 已处理过的文件
PROCESSED_FILES = [
    'fabric_order_service.rs',
    'purchase_order_service.rs',
    'batch_service.rs',
    'cost_collection_service.rs',
    'inventory_count_service.rs',
    'inventory_transfer_service.rs',
    'purchase_inspection_service.rs',
    'purchase_price_service.rs',
    'purchase_receipt_service.rs',
    'purchase_return_service.rs',
    'sales_price_service.rs',
    'supplier_service.rs',
    'purchase_contract_service.rs',
    'sales_contract_service.rs',
    'business_trace_service.rs',
    'customer_credit_service.rs',
    'dashboard_service.rs',
    'dye_batch_service.rs',
    'dye_recipe_service.rs',
    'fixed_asset_service.rs',
    'greige_fabric_service.rs',
    'customer_service.rs',
    'department_service.rs',
    'product_service.rs',
    'warehouse_service.rs',
    'inventory_adjustment_service.rs',
    'mod.rs',
    'api.rs',
    'auth.rs',
    'user_service.rs',
    'inventory_service.rs',
]

# 需要跳过的struct名称（服务本身）
SKIP_STRUCTS = [
    'Service',
    'ApiService',
]

def extract_data_structs(file_path):
    """从service文件中提取数据模型struct和enum定义"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    lines = content.split('\n')
    structs = []
    current_struct = []
    in_struct = False
    brace_count = 0
    struct_name = ''
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        
        # 检查是否是struct或enum开始
        if not in_struct:
            # 匹配 pub struct 或 pub enum
            struct_match = re.match(r'^///.*?(数据|模型|响应|请求|参数|摘要|明细|列表)', stripped)
            pub_struct_match = re.match(r'^pub\s+(struct|enum)\s+(\w+)', stripped)
            
            if struct_match or pub_struct_match:
                if pub_struct_match:
                    struct_type, struct_name = pub_struct_match.groups()
                    # 跳过服务本身的struct
                    if struct_name.endswith('Service') or struct_name in SKIP_STRUCTS:
                        continue
                
                in_struct = True
                brace_count = 0
                current_struct = [line]
                
                # 统计当前行的花括号
                brace_count += line.count('{') - line.count('}')
        else:
            current_struct.append(line)
            brace_count += line.count('{') - line.count('}')
            
            if brace_count <= 0:
                # struct结束
                struct_text = '\n'.join(current_struct)
                # 检查是否真的是数据模型（不是服务）
                if not any(skip in struct_text for skip in ['impl.*Service', 'pub struct.*Service']):
                    structs.append(struct_text)
                in_struct = False
                current_struct = []
    
    # 处理文件末尾可能未闭合的struct
    if in_struct and current_struct:
        struct_text = '\n'.join(current_struct)
        if not any(skip in struct_text for skip in ['impl.*Service', 'pub struct.*Service']):
            structs.append(struct_text)
    
    return structs, content

def snake_to_pascal(snake_str):
    """将蛇形命名转换为帕斯卡命名"""
    components = snake_str.split('_')
    return ''.join(x.title() for x in components)

def process_service_file(service_file):
    """处理单个service文件"""
    file_name = service_file.name
    print(f"\n📄 处理文件: {file_name}")
    
    # 提取模型名称
    base_name = file_name.replace('_service.rs', '')
    
    # 读取service文件
    structs, content = extract_data_structs(service_file)
    
    if not structs:
        print(f"   ℹ️  未找到数据模型定义")
        return None
    
    # 过滤掉服务相关的struct
    filtered_structs = []
    for s in structs:
        if 'impl' not in s and 'Service' not in s:
            filtered_structs.append(s)
    
    if not filtered_structs:
        print(f"   ℹ️  过滤后未找到数据模型定义")
        return None
    
    print(f"   ✅ 找到 {len(filtered_structs)} 个数据模型定义")
    
    # 创建模型文件
    model_file_name = f"{base_name}.rs"
    model_file_path = FRONTEND_MODELS_DIR / model_file_name
    
    # 构建模型文件内容
    model_content = f"//! {base_name.replace('_', ' ').title()}模型\n\n"
    model_content += "use serde::{Deserialize, Serialize};\n\n"
    model_content += '\n\n'.join(filtered_structs)
    model_content += '\n'
    
    with open(model_file_path, 'w', encoding='utf-8') as f:
        f.write(model_content)
    
    print(f"   📝 创建模型文件: {model_file_name}")
    
    # 构建新的service文件内容 - 移除struct/enum定义
    new_content = content
    for s in filtered_structs:
        new_content = new_content.replace(s, '')
    
    # 移除多余的空行
    new_content = re.sub(r'\n{3,}', '\n\n', new_content).strip() + '\n'
    
    # 添加模型导入
    import_line = f"use crate::models::{base_name}::*;\n"
    
    # 查找第一个use语句的位置
    use_match = re.search(r'^use\s+', new_content, re.MULTILINE)
    if use_match:
        # 在第一个use语句之前插入导入
        pos = use_match.start()
        new_content = new_content[:pos] + import_line + new_content[pos:]
    else:
        new_content = import_line + '\n' + new_content
    
    # 写入新的service文件
    with open(service_file, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print(f"   🔧 更新服务文件")
    
    return base_name

def main():
    print("=" * 80)
    print("安全处理剩余服务文件 - 提取数据模型")
    print("=" * 80)
    
    # 获取所有服务文件
    service_files = list(FRONTEND_SERVICES_DIR.glob("*.rs"))
    
    # 过滤掉已处理的文件
    remaining_files = [f for f in service_files if f.name not in PROCESSED_FILES]
    
    print(f"\n发现 {len(remaining_files)} 个剩余文件需要处理\n")
    
    processed_models = []
    
    for service_file in sorted(remaining_files):
        model_name = process_service_file(service_file)
        if model_name:
            processed_models.append(model_name)
    
    # 更新mod.rs
    if processed_models:
        mod_file = FRONTEND_MODELS_DIR / 'mod.rs'
        with open(mod_file, 'r', encoding='utf-8') as f:
            mod_content = f.read()
        
        # 添加新的pub mod声明
        for model_name in processed_models:
            if f'pub mod {model_name};' not in mod_content:
                mod_content += f'pub mod {model_name};\n'
        
        with open(mod_file, 'w', encoding='utf-8') as f:
            f.write(mod_content)
        
        print(f"\n📦 更新 models/mod.rs，添加了 {len(processed_models)} 个新模块")
    
    print("\n" + "=" * 80)
    print(f"处理完成！共处理了 {len(processed_models)} 个文件")
    print("=" * 80)

if __name__ == "__main__":
    main()

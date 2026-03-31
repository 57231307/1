#!/usr/bin/env python3
"""
批量处理剩余的服务文件
将模型从service文件提取到models目录
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

def extract_structs_from_service(file_path):
    """从service文件中提取struct和enum定义"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 匹配pub struct和pub enum定义
    struct_pattern = r'///.*?\n(?:#\[.*?\]\n)*pub\s+(struct|enum)\s+(\w+)(?:.*?)\{[\s\S]*?\n\}'
    matches = re.finditer(struct_pattern, content, re.MULTILINE | re.DOTALL)
    
    structs = []
    for match in matches:
        structs.append(match.group(0))
    
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
    structs, content = extract_structs_from_service(service_file)
    
    if not structs:
        print(f"   ℹ️  未找到struct/enum定义")
        return None
    
    print(f"   ✅ 找到 {len(structs)} 个struct/enum定义")
    
    # 创建模型文件
    model_file_name = f"{base_name}.rs"
    model_file_path = FRONTEND_MODELS_DIR / model_file_name
    
    # 构建模型文件内容
    model_content = f"//! {base_name.replace('_', ' ').title()}模型\n\n"
    model_content += "use serde::{Deserialize, Serialize};\n\n"
    model_content += '\n\n'.join(structs)
    model_content += '\n'
    
    with open(model_file_path, 'w', encoding='utf-8') as f:
        f.write(model_content)
    
    print(f"   📝 创建模型文件: {model_file_name}")
    
    # 构建新的service文件内容
    # 移除struct/enum定义
    new_content = content
    for s in structs:
        new_content = new_content.replace(s, '')
    
    # 移除多余的空行
    new_content = re.sub(r'\n{3,}', '\n\n', new_content).strip()
    
    # 添加模型导入
    model_name_pascal = snake_to_pascal(base_name)
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
    print("批量处理剩余服务文件")
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

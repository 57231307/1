#!/usr/bin/env python3
"""
批量修复剩余服务文件的脚本
将模型移到models目录，使用统一的ApiResponse格式
"""

import os
import re
from pathlib import Path

FRONTEND_SERVICES_DIR = Path("/workspace/frontend/src/services")
FRONTEND_MODELS_DIR = Path("/workspace/frontend/src/models")

# 已经修复过的文件
FIXED_FILES = [
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
    'mod.rs',
    'api.rs',
    'auth.rs',
]

def main():
    print("=" * 80)
    print("批量分析剩余服务文件")
    print("=" * 80)
    
    # 获取所有服务文件
    service_files = list(FRONTEND_SERVICES_DIR.glob("*.rs"))
    
    # 过滤掉已修复的文件
    remaining_files = [f for f in service_files if f.name not in FIXED_FILES]
    
    print(f"\n发现 {len(remaining_files)} 个剩余文件需要分析：\n")
    
    for file_path in sorted(remaining_files):
        print(f"📄 {file_path.name}")
        
        # 读取文件内容
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 检查问题
        has_struct = bool(re.search(r'(pub )?(struct|enum)\s+\w+', content))
        has_f64 = 'f64' in content
        has_manual_api = 'serde_json::Value' in content or 'ApiResponse' in content
        
        issues = []
        if has_struct:
            issues.append("在service文件内定义数据模型")
        if has_f64:
            issues.append("使用f64类型")
        if has_manual_api:
            issues.append("可能需要统一ApiResponse格式")
        
        if issues:
            for issue in issues:
                print(f"   ⚠️  {issue}")
        else:
            print(f"   ✅ 未发现明显问题")
        
        print()
    
    print("=" * 80)
    print(f"总计分析了 {len(remaining_files)} 个剩余服务文件")
    print("=" * 80)

if __name__ == "__main__":
    main()

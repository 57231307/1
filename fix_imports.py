#!/usr/bin/env python3
import os
import re

def fix_imports():
    pages_dir = "/workspace/frontend/src/pages"
    
    for filename in os.listdir(pages_dir):
        if filename.endswith(".rs") and filename != "mod.rs" and filename != "login.rs":
            filepath = os.path.join(pages_dir, filename)
            print(f"处理文件: {filename}")
            
            with open(filepath, "r", encoding="utf-8") as f:
                content = f.read()
            
            # 匹配导入模式
            # 模式: use crate::services::xxx_service::{ ServiceName, Model1, Model2, ... };
            pattern = r'use crate::services::([a-z_]+)_service::\{([^}]+)\};'
            
            def replace_import(match):
                service_name = match.group(1)
                imports = match.group(2).strip()
                
                # 分离服务名和模型名
                import_list = [imp.strip() for imp in imports.split(",") if imp.strip()]
                service_import = None
                model_imports = []
                
                for imp in import_list:
                    if imp.endswith("Service"):
                        service_import = imp
                    else:
                        model_imports.append(imp)
                
                if not service_import:
                    return match.group(0)
                
                # 构建新的导入语句
                result = ""
                if model_imports:
                    result += f'use crate::models::{service_name}::{{\n    {", ".join(model_imports)},\n}};\n'
                result += f'use crate::services::{service_name}_service::{service_import};'
                
                return result
            
            # 执行替换
            new_content = re.sub(pattern, replace_import, content)
            
            if new_content != content:
                with open(filepath, "w", encoding="utf-8") as f:
                    f.write(new_content)
                print(f"  ✅ 已修复导入")
            else:
                print(f"  ℹ️  无需修改")

if __name__ == "__main__":
    fix_imports()

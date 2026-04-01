#!/usr/bin/env python3
import os
import re

# 要修改的目录
handlers_dir = '/workspace/backend/src/handlers'

# 匹配 State<Arc<DatabaseConnection>> 的正则表达式
state_pattern = re.compile(r'State<Arc<DatabaseConnection>>')

# 匹配需要添加 AppState 导入的模式
import_pattern = re.compile(r'use sea_orm::DatabaseConnection;')

# 遍历所有 .rs 文件
for root, dirs, files in os.walk(handlers_dir):
    for file in files:
        if file.endswith('.rs'):
            file_path = os.path.join(root, file)
            
            # 读取文件内容
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            # 检查是否需要修改
            if 'State<Arc<DatabaseConnection>>' in content:
                print(f'Fixing {file_path}...')
                
                # 添加 AppState 导入（如果没有）
                if 'use crate::utils::app_state::AppState;' not in content:
                    # 在 sea_orm 导入后添加 AppState 导入
                    content = import_pattern.sub(r'use sea_orm::DatabaseConnection;\nuse crate::utils::app_state::AppState;', content)
                
                # 替换 State<Arc<DatabaseConnection>> 为 State<AppState>
                content = state_pattern.sub('State<AppState>', content)
                
                # 替换函数参数中的 db 为 state
                content = content.replace('State(db): State<AppState>', 'State(state): State<AppState>')
                content = content.replace('State(_db): State<AppState>', 'State(_state): State<AppState>')
                
                # 替换函数体内的 db 为 state.db
                content = content.replace('db.clone()', 'state.db.clone()')
                content = content.replace('&db', '&state.db')
                
                # 写回文件
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write(content)
                
                print(f'Fixed {file_path}')

print('All files fixed!')

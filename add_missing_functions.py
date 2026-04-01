#!/usr/bin/env python3
import os

handlers_to_fix = {
    "/workspace/backend/src/handlers/fixed_asset_handler.rs": [
        ("update_asset", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "固定资产更新功能尚未实现"),
        ("delete_asset", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "固定资产删除功能尚未实现"),
    ],
    "/workspace/backend/src/handlers/budget_management_handler.rs": [
        ("list_budgets", "Query(_params): Query<serde_json::Value>", "State(_state): State<AppState>", "auth: AuthContext", "预算列表查询功能尚未实现"),
        ("create_budget", "State(_state): State<AppState>", "auth: AuthContext", "Json(_req): Json<serde_json::Value>", "预算创建功能尚未实现"),
        ("get_budget", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "预算详情查询功能尚未实现"),
        ("update_budget", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "预算更新功能尚未实现"),
        ("delete_budget", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "预算删除功能尚未实现"),
        ("approve_budget", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "预算审批功能尚未实现"),
    ],
    "/workspace/backend/src/handlers/customer_credit_handler.rs": [
        ("create_credit", "State(_state): State<AppState>", "auth: AuthContext", "Json(_req): Json<serde_json::Value>", "客户信用创建功能尚未实现"),
        ("update_credit", "Path(_id): Path<i32>", "State(_state): State<AppState>", "auth: AuthContext", "客户信用更新功能尚未实现"),
    ],
}

for filepath, functions in handlers_to_fix.items():
    if not os.path.exists(filepath):
        print(f"文件不存在: {filepath}")
        continue
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    for func_name, *args in functions:
        if f"pub async fn {func_name}" in content:
            print(f"函数 {func_name} 已存在于 {filepath}")
            continue
        
        message = args[-1]
        params = args[:-1]
        params_str = ", ".join(params)
        
        new_func = f'''

/// {message}
pub async fn {func_name}(
    {params_str},
) -> Result<Json<ApiResponse<String>>, AppError> {{
    info!("用户 {{}} 正在{message}", auth.user_id);
    Err(AppError::ValidationError("{message}".to_string()))
}}
'''
        content += new_func
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"已添加缺失函数到: {filepath}")

print("\n批量添加缺失函数完成!")

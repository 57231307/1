此次合并主要实现了API响应格式的统一化处理，并新增了项目文档，同时对前端服务调用方式进行了优化。变更涉及后端认证和用户处理逻辑，以及前端API服务和认证服务的调整，提升了系统的一致性和可维护性。
| 文件 | 变更 |
|------|---------|
| CODE_WIKI.md | 新增项目详细文档，包含项目概述、技术栈、项目结构、核心功能模块、数据库结构、API接口、部署与监控、运行方式、开发指南等完整信息 |
| backend/src/handlers/auth_handler.rs | - 引入ApiResponse统一响应格式<br>- 替换错误处理方式，使用统一的ApiResponse结构<br>- 调整返回值类型，使用ApiResponse包装响应数据<br>- 简化注销和刷新令牌的实现 |
| backend/src/handlers/user_handler.rs | - 引入ApiResponse统一响应格式<br>- 替换错误处理方式，使用统一的ApiResponse结构<br>- 调整返回值类型，使用ApiResponse包装响应数据<br>- 新增DeleteUserResponse结构，统一删除用户的响应格式 |
| frontend/src/models/api_response.rs | 新增统一的API响应格式定义，包括ApiResponse和PaginatedResponse结构 |
| frontend/src/services/api.rs | - 引入ApiResponse统一响应格式<br>- 调整API基础路径为相对路径<br>- 修改请求处理逻辑，使用统一的ApiResponse结构解析响应<br>- 优化错误处理和重试逻辑 |
| frontend/src/services/auth.rs | - 简化AuthService实现，移除api_base_url字段<br>- 使用ApiService替代直接的HTTP请求<br>- 添加logout和refresh_token的异步实现 |
| frontend/src/models/mod.rs | 新增模块导出，包含所有模型定义 |
| frontend/src/services/user_service.rs | - 调整服务实现，使用ApiService进行API调用<br>- 优化错误处理逻辑 |
| batch_fix_remaining.py | 新增批处理脚本，用于修复剩余问题 |
| batch_process_remaining.py | 新增批处理脚本，用于处理剩余任务 |
| check_all_services.py | 新增服务检查脚本，用于验证所有服务状态 |
| fix_api_paths.py | 新增API路径修复脚本，用于统一API路径 |
| fix_imports.py | 新增导入修复脚本，用于修复导入语句 |
| safe_process_remaining.py | 新增安全处理脚本，用于安全处理剩余任务 |
| build_all.bat | 删除构建脚本 |
| frontend/build.bat | 删除前端构建脚本 |
| releases/README.md | 删除发布说明文件
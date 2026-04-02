# 秉羲管理系统优化与功能扩展 - 实施计划（任务分解与优先级排序）

## 阶段一：基础设施层 - 安全强化与后端访问控制

### [ ] 任务 1.1：前端路由守卫实现
- **优先级**：P0
- **依赖**：无
- **描述**：
  - 在前端路由切换时检查localStorage中的token
  - token不存在或已过期时重定向到登录页
  - 实现Yew Router的路由守卫组件
- **验收标准**：AC-1
- **测试要求**：
  - `programmatic` TR-1.1.1：未登录访问受保护路由时重定向到登录页
  - `programmatic` TR-1.1.2：token过期后访问受保护路由时重定向到登录页
  - `programmatic` TR-1.1.3：已登录访问受保护路由时正常显示页面
- **备注**：需要修改frontend/src/app/mod.rs中的路由配置

### [ ] 任务 1.2：后端CORS配置优化
- **优先级**：P0
- **依赖**：无
- **描述**：
  - 配置CORS只允许前端域名访问
  - 禁止携带凭证的跨域请求
  - 添加安全响应头
- **验收标准**：AC-2
- **测试要求**：
  - `programmatic` TR-1.2.1：非允许域名的跨域请求被拒绝
  - `programmatic` TR-1.2.2：允许域名的跨域请求正常处理
  - `programmatic` TR-1.2.3：响应头包含安全相关头部
- **备注**：修改backend/src/main.rs中的CORS配置

### [ ] 任务 1.3：请求头验证中间件
- **优先级**：P0
- **依赖**：任务1.2
- **描述**：
  - 创建请求头验证中间件
  - 验证X-Requested-With请求头
  - 缺少请求头时返回403错误
- **验收标准**：AC-3
- **测试要求**：
  - `programmatic` TR-1.3.1：缺少X-Requested-With请求头的请求返回403
  - `programmatic` TR-1.3.2：包含正确请求头的请求正常处理
  - `programmatic` TR-1.3.3：公开路径不需要验证请求头
- **备注**：创建backend/src/middleware/request_validator.rs

### [ ] 任务 1.4：认证中间件增强
- **优先级**：P0
- **依赖**：任务1.3
- **描述**：
  - 将公开路径配置化
  - 添加认证失败日志记录
  - 优化错误响应
- **验收标准**：AC-4
- **测试要求**：
  - `programmatic` TR-1.4.1：公开路径无需认证即可访问
  - `programmatic` TR-1.4.2：无效token返回401并记录日志
  - `programmatic` TR-1.4.3：过期token返回401并记录日志
- **备注**：修改backend/src/middleware/auth.rs

### [ ] 任务 1.5：Nginx配置优化
- **优先级**：P1
- **依赖**：任务1.2, 1.3
- **描述**：
  - 配置后端只监听内网端口
  - 所有外部请求通过Nginx代理
  - 添加安全响应头
- **验收标准**：AC-2, AC-3
- **测试要求**：
  - `programmatic` TR-1.5.1：直接访问后端端口被拒绝
  - `programmatic` TR-1.5.2：通过Nginx代理访问正常
  - `programmatic` TR-1.5.3：响应头包含X-Frame-Options等安全头
- **备注**：修改deploy/nginx.conf

## 阶段二：数据模型层 - 权限体系与供应商产品映射

### [ ] 任务 2.1：权限数据模型设计
- **优先级**：P0
- **依赖**：阶段一完成
- **描述**：
  - 设计权限表结构（permissions）
  - 设计角色权限关联表（role_permissions）
  - 设计数据权限配置表（data_permissions）
- **验收标准**：AC-5, AC-6
- **测试要求**：
  - `programmatic` TR-2.1.1：权限表创建成功
  - `programmatic` TR-2.1.2：角色权限关联表创建成功
  - `programmatic` TR-2.1.3：数据权限配置表创建成功
- **备注**：创建数据库迁移文件

### [ ] 任务 2.2：权限中间件实现
- **优先级**：P0
- **依赖**：任务2.1
- **描述**：
  - 实现功能权限验证中间件
  - 实现数据权限验证中间件
  - 集成到路由层
- **验收标准**：AC-5, AC-6
- **测试要求**：
  - `programmatic` TR-2.2.1：无功能权限的用户访问被拒绝
  - `programmatic` TR-2.2.2：有功能权限的用户访问正常
  - `programmatic` TR-2.2.3：数据权限验证正确执行
- **备注**：创建backend/src/middleware/permission.rs

### [ ] 任务 2.3：供应商产品映射数据模型
- **优先级**：P0
- **依赖**：阶段一完成
- **描述**：
  - 创建供应商产品表（supplier_products）
  - 创建供应商产品色号表（supplier_product_colors）
  - 创建产品色号映射表（product_color_mappings）
- **验收标准**：AC-7, AC-8
- **测试要求**：
  - `programmatic` TR-2.3.1：供应商产品表创建成功
  - `programmatic` TR-2.3.2：供应商产品色号表创建成功
  - `programmatic` TR-2.3.3：产品色号映射表创建成功
- **备注**：创建数据库迁移文件

### [ ] 任务 2.4：供应商产品映射模型文件
- **优先级**：P0
- **依赖**：任务2.3
- **描述**：
  - 创建backend/src/models/supplier_product.rs
  - 创建backend/src/models/supplier_product_color.rs
  - 创建backend/src/models/product_color_mapping.rs
  - 更新models/mod.rs导出新模块
- **验收标准**：AC-7, AC-8
- **测试要求**：
  - `programmatic` TR-2.4.1：模型文件编译通过
  - `programmatic` TR-2.4.2：模型与数据库表结构一致
- **备注**：使用SeaORM实体宏

### [ ] 任务 2.5：供应商产品映射前端模型
- **优先级**：P1
- **依赖**：任务2.4
- **描述**：
  - 创建frontend/src/models/supplier_product.rs
  - 创建frontend/src/models/product_color_mapping.rs
  - 更新models/mod.rs导出新模块
- **验收标准**：AC-7, AC-8
- **测试要求**：
  - `programmatic` TR-2.5.1：前端模型编译通过
  - `programmatic` TR-2.5.2：前端模型与后端模型一致
- **备注**：使用Serde序列化

## 阶段三：业务逻辑层 - 功能实现

### [ ] 任务 3.1：供应商产品管理服务
- **优先级**：P0
- **依赖**：任务2.4
- **描述**：
  - 创建供应商产品CRUD服务
  - 创建供应商产品色号CRUD服务
  - 创建产品色号映射CRUD服务
- **验收标准**：AC-7, AC-8
- **测试要求**：
  - `programmatic` TR-3.1.1：创建供应商产品成功
  - `programmatic` TR-3.1.2：查询供应商产品列表成功
  - `programmatic` TR-3.1.3：创建产品色号映射成功
  - `programmatic` TR-3.1.4：查询产品色号映射成功
- **备注**：创建backend/src/services/supplier_product_service.rs

### [ ] 任务 3.2：供应商产品管理API接口
- **优先级**：P0
- **依赖**：任务3.1
- **描述**：
  - 创建供应商产品管理Handler
  - 注册API路由
  - 添加权限控制
- **验收标准**：AC-7, AC-8
- **测试要求**：
  - `programmatic` TR-3.2.1：POST /api/v1/erp/supplier-products 创建成功
  - `programmatic` TR-3.2.2：GET /api/v1/erp/supplier-products 查询成功
  - `programmatic` TR-3.2.3：无权限用户访问返回403
- **备注**：创建backend/src/handlers/supplier_product_handler.rs

### [ ] 任务 3.3：供应商产品管理前端服务
- **优先级**：P1
- **依赖**：任务2.5, 3.2
- **描述**：
  - 创建前端API调用服务
  - 实现供应商产品管理页面
  - 实现产品色号映射管理页面
- **验收标准**：AC-7, AC-8
- **测试要求**：
  - `programmatic` TR-3.3.1：前端页面渲染成功
  - `programmatic` TR-3.3.2：创建供应商产品表单提交成功
  - `programmatic` TR-3.3.3：产品色号映射功能正常
- **备注**：创建frontend/src/services/supplier_product_service.rs

### [ ] 任务 3.4：销售转采购转换服务
- **优先级**：P0
- **依赖**：任务3.1
- **描述**：
  - 实现销售订单审批后自动触发转换
  - 实现按供应商拆分采购订单逻辑
  - 实现产品色号自动转换逻辑
  - 实现默认供应商选择逻辑
- **验收标准**：AC-9, AC-10, AC-11
- **测试要求**：
  - `programmatic` TR-3.4.1：销售订单审批后自动生成采购订单
  - `programmatic` TR-3.4.2：采购订单按供应商正确拆分
  - `programmatic` TR-3.4.3：产品色号正确转换
  - `programmatic` TR-3.4.4：默认供应商选择正确
- **备注**：修改backend/src/services/sales_order_service.rs

### [ ] 任务 3.5：权限管理服务
- **优先级**：P0
- **依赖**：任务2.2
- **描述**：
  - 实现权限CRUD服务
  - 实现角色权限分配服务
  - 实现数据权限配置服务
- **验收标准**：AC-5, AC-6
- **测试要求**：
  - `programmatic` TR-3.5.1：创建权限成功
  - `programmatic` TR-3.5.2：分配角色权限成功
  - `programmatic` TR-3.5.3：配置数据权限成功
- **备注**：创建backend/src/services/permission_service.rs

### [ ] 任务 3.6：权限管理API接口
- **优先级**：P0
- **依赖**：任务3.5
- **描述**：
  - 创建权限管理Handler
  - 注册API路由
  - 集成权限验证中间件
- **验收标准**：AC-5, AC-6
- **测试要求**：
  - `programmatic` TR-3.6.1：权限API接口正常工作
  - `programmatic` TR-3.6.2：权限验证中间件正确拦截
- **备注**：修改backend/src/handlers/role_handler.rs

## 阶段四：代码优化层 - 性能与质量提升

### [ ] 任务 4.1：数据库查询优化
- **优先级**：P1
- **依赖**：阶段三完成
- **描述**：
  - 分析慢查询日志
  - 添加必要的数据库索引
  - 修复N+1查询问题
  - 优化复杂查询语句
- **验收标准**：AC-12
- **测试要求**：
  - `programmatic` TR-4.1.1：所有查询使用索引
  - `programmatic` TR-4.1.2：无N+1查询问题
  - `programmatic` TR-4.1.3：查询响应时间符合要求
- **备注**：使用EXPLAIN分析查询计划

### [ ] 任务 4.2：缓存策略实现
- **优先级**：P1
- **依赖**：任务4.1
- **描述**：
  - 实现内存缓存模块
  - 缓存频繁访问的数据（如权限配置、产品信息）
  - 实现缓存失效策略
- **验收标准**：AC-12
- **测试要求**：
  - `programmatic` TR-4.2.1：缓存命中时响应时间减少
  - `programmatic` TR-4.2.2：缓存失效后数据正确更新
- **备注**：创建backend/src/utils/cache.rs

### [ ] 任务 4.3：前端性能优化
- **优先级**：P2
- **依赖**：阶段三完成
- **描述**：
  - 实现组件懒加载
  - 优化大列表渲染
  - 减少不必要的重渲染
- **验收标准**：NFR-2
- **测试要求**：
  - `programmatic` TR-4.3.1：首屏加载时间 < 3s
  - `human-judgment` TR-4.3.2：页面交互流畅，无明显卡顿
- **备注**：使用Yew的lazy组件

### [ ] 任务 4.4：代码重复消除
- **优先级**：P1
- **依赖**：阶段三完成
- **描述**：
  - 识别并消除重复代码
  - 提取公共函数和组件
  - 统一错误处理模式
- **验收标准**：AC-13
- **测试要求**：
  - `human-judgment` TR-4.4.1：代码审查确认无重复代码块
  - `programmatic` TR-4.4.2：编译无警告
- **备注**：使用cargo clippy检测

### [ ] 任务 4.5：未使用代码清理
- **优先级**：P2
- **依赖**：任务4.4
- **描述**：
  - 移除未使用的函数和变量
  - 移除未使用的导入
  - 移除未使用的依赖
- **验收标准**：AC-13
- **测试要求**：
  - `programmatic` TR-4.5.1：cargo clippy无dead_code警告
  - `programmatic` TR-4.5.2：编译成功
- **备注**：使用cargo clippy检测

### [ ] 任务 4.6：API响应格式统一
- **优先级**：P1
- **依赖**：阶段三完成
- **描述**：
  - 统一所有API响应格式为ApiResponse<T>
  - 统一错误响应格式
  - 更新前端API调用适配
- **验收标准**：AC-15
- **测试要求**：
  - `programmatic` TR-4.6.1：所有API响应格式一致
  - `programmatic` TR-4.6.2：错误响应格式一致
- **备注**：修改backend/src/utils/response.rs

### [ ] 任务 4.7：缺失CRUD操作补全
- **优先级**：P1
- **依赖**：阶段三完成
- **描述**：
  - 检查所有模块的CRUD操作完整性
  - 补全缺失的更新和删除操作
  - 确保前后端接口一致
- **验收标准**：AC-14
- **测试要求**：
  - `programmatic` TR-4.7.1：所有模块都有完整的CRUD操作
  - `programmatic` TR-4.7.2：前后端接口一一对应
- **备注**：检查所有handler和service文件

### [ ] 任务 4.8：重复功能接口消除
- **优先级**：P2
- **依赖**：任务4.6, 4.7
- **描述**：
  - 识别功能重复的API接口
  - 合并或删除重复接口
  - 更新前端调用
- **验收标准**：AC-14, AC-15
- **测试要求**：
  - `programmatic` TR-4.8.1：无功能重复的API接口
  - `programmatic` TR-4.8.2：前端调用正常工作
- **备注**：检查routes/mod.rs中的路由定义

### [ ] 任务 4.9：代码注释完善
- **优先级**：P2
- **依赖**：任务4.4, 4.5
- **描述**：
  - 为公共API添加文档注释
  - 为复杂逻辑添加说明注释
  - 更新过时的注释
- **验收标准**：NFR-3
- **测试要求**：
  - `human-judgment` TR-4.9.1：代码注释覆盖率 > 30%
  - `human-judgment` TR-4.9.2：注释内容准确有用
- **备注**：使用cargo doc生成文档

### [ ] 任务 4.10：集成测试与验证
- **优先级**：P0
- **依赖**：所有任务完成
- **描述**：
  - 执行完整的功能测试
  - 执行安全测试
  - 执行性能测试
  - 修复发现的问题
- **验收标准**：所有AC
- **测试要求**：
  - `programmatic` TR-4.10.1：所有功能测试通过
  - `programmatic` TR-4.10.2：安全测试无漏洞
  - `programmatic` TR-4.10.3：性能指标达标
- **备注**：使用cargo test执行测试

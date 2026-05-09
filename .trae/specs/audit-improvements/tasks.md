# 审计改进实施计划

## 阶段一：安全修复（P1优先级）

### [ ] 任务 1.1：移除生产环境debug日志
- **文件**: backend/src/middleware/auth.rs, backend/src/middleware/permission.rs
- **描述**: 
  - auth.rs第67行：移除 `tracing::debug!("JWT key length: {}", key.len())`
  - permission.rs第40行：移除 `tracing::debug!("User: {:?}, Resource: {}, Action: {}", auth.user, resource, action)`
- **验证**: cargo check 通过，无编译错误

### [ ] 任务 1.2：修复硬编码密钥警告
- **文件**: backend/src/utils/app_state.rs
- **描述**: 
  - Default实现中的硬编码密钥仅用于测试环境
  - 添加明确注释说明仅用于测试
  - 考虑在release模式下panic
- **验证**: cargo check 通过

### [ ] 任务 1.3：安全中间件增强
- **文件**: backend/src/middleware/auth.rs
- **描述**: 
  - 将公开路径配置化（当前硬编码在auth函数中）
  - 添加认证失败日志记录（仅记录事件，不记录敏感信息）
- **验证**: 编译通过，功能正常

## 阶段二：数据一致性（P2优先级）

### [ ] 任务 2.1：添加核心表外键约束
- **文件**: database/migration/008_core_foreign_keys.sql
- **描述**: 
  - sales_orders → customers (customer_id)
  - purchase_orders → suppliers (supplier_id)
  - inventory_stock → products (product_id)
  - inventory_stock → warehouses (warehouse_id)
  - purchase_receipts → purchase_orders (order_id)
  - sales_deliveries → sales_orders (order_id)
- **验证**: 迁移文件可正常执行

### [ ] 任务 2.2：添加财务表外键约束
- **文件**: database/migration/009_finance_foreign_keys.sql
- **描述**: 
  - ap_invoices → purchase_orders
  - ap_payments → ap_invoices
  - ar_invoices → sales_orders
  - vouchers → accounting_periods
- **验证**: 迁移文件可正常执行

### [ ] 任务 2.3：添加库存表外键约束
- **文件**: database/migration/010_inventory_foreign_keys.sql
- **描述**: 
  - inventory_transfers → warehouses (from_warehouse_id, to_warehouse_id)
  - inventory_counts → warehouses
  - inventory_adjustments → warehouses
- **验证**: 迁移文件可正常执行

## 阶段三：架构优化（P3优先级）

### [ ] 任务 3.1：DI容器注册核心服务
- **文件**: backend/src/utils/di_container.rs, backend/src/utils/app_state.rs
- **描述**: 
  - 将AuthService、UserService等核心服务注册到DI容器
  - AppState通过DI容器获取服务实例
- **验证**: 编译通过，服务可正常获取

### [ ] 任务 3.2：推广DI容器使用示例
- **文件**: backend/src/handlers/auth_handler.rs（作为示例）
- **描述**: 
  - 修改auth_handler使用DI容器获取AuthService
  - 展示DI容器的使用模式
- **验证**: 编译通过，功能正常

## 阶段四：代码质量（P4优先级）

### [ ] 任务 4.1：创建集成测试框架
- **文件**: backend/tests/integration/mod.rs
- **描述**: 
  - 创建测试数据库连接池
  - 创建测试用例基类
  - 添加测试辅助函数
- **验证**: cargo test 可正常执行

### [ ] 任务 4.2：添加核心流程集成测试
- **文件**: backend/tests/integration/auth_flow.rs, backend/tests/integration/sales_flow.rs
- **描述**: 
  - 认证流程测试（登录→访问→登出）
  - 销售订单流程测试（创建→审批→发货）
- **验证**: cargo test 通过

### [ ] 任务 4.3：统一API响应格式
- **文件**: backend/src/utils/response.rs
- **描述**: 
  - 确保所有Handler使用统一的ApiResponse<T>格式
  - 统一错误响应格式
- **验证**: 编译通过，API响应格式一致

## 阶段五：gRPC服务完善（P5优先级）

### [ ] 任务 5.1：启用gRPC服务注册
- **文件**: backend/src/main.rs
- **描述**: 
  - 在main.rs中注册gRPC服务
  - 配置gRPC监听端口
- **验证**: gRPC服务可正常启动

### [ ] 任务 5.2：完善gRPC服务实现
- **文件**: backend/src/grpc/service.rs
- **描述**: 
  - 实现缺失的gRPC方法
  - 添加错误处理
- **验证**: gRPC客户端可正常调用

# 任务依赖关系
- 任务1.1、1.2、1.3 可并行执行
- 任务2.1、2.2、2.3 可并行执行
- 任务3.1 依赖 阶段一完成
- 任务3.2 依赖 任务3.1
- 任务4.1 依赖 阶段一完成
- 任务4.2 依赖 任务4.1
- 任务4.3 可独立执行
- 任务5.1、5.2 可并行执行

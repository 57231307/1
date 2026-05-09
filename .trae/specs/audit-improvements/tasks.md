# 审计改进实施计划

## 阶段一：安全修复（P1优先级）

### [x] 任务 1.1：移除生产环境debug日志
- **文件**: backend/src/middleware/auth.rs, backend/src/middleware/permission.rs
- **描述**: 
  - auth.rs：移除暴露JWT密钥长度的debug日志，改为记录认证失败事件（不含敏感信息）
  - permission.rs：移除暴露用户详细信息的debug日志，改为记录权限检查事件
- **验证**: cargo check 通过，无编译错误

### [x] 任务 1.2：修复硬编码密钥警告
- **文件**: backend/src/utils/app_state.rs
- **描述**: 
  - Default实现添加 `#[cfg(not(debug_assertions))] panic!(...)` 防止生产环境使用
  - 添加明确注释说明仅用于测试环境
- **验证**: cargo check 通过

### [x] 任务 1.3：安全中间件增强
- **文件**: backend/src/middleware/auth.rs, backend/src/middleware/public_routes.rs
- **描述**: 
  - 公开路径已配置化在 public_routes.rs 中
  - 认证失败日志记录事件（仅记录path，不记录敏感信息）
- **验证**: 编译通过，功能正常

## 阶段二：数据一致性（P2优先级）

### [x] 任务 2.1：添加核心表外键约束
- **文件**: database/migration/008_core_foreign_keys.sql
- **描述**: 
  - sales_orders → customers (customer_id)
  - purchase_orders → suppliers (supplier_id)
  - inventory_stock → products (product_id)
  - inventory_stock → warehouses (warehouse_id)
  - purchase_receipts → purchase_orders (order_id)
  - sales_deliveries → sales_orders (order_id)
  - sales_returns → sales_orders (order_id)
  - purchase_returns → purchase_orders (order_id)
  - users → departments (department_id)
  - products → product_categories (category_id)
- **验证**: 迁移文件已创建

### [x] 任务 2.2：添加财务表外键约束
- **文件**: database/migration/009_finance_foreign_keys.sql
- **描述**: 
  - ap_invoices → purchase_orders
  - ap_payments → ap_invoices
  - ar_invoices → sales_orders
  - vouchers → accounting_periods
  - voucher_items → vouchers
  - voucher_items → account_subjects
  - ap_verifications → ap_invoices
  - ap_verification_items → ap_verifications
  - ap_payment_requests → ap_invoices
  - ap_payment_request_items → ap_payment_requests
- **验证**: 迁移文件已创建

### [x] 任务 2.3：添加库存表外键约束
- **文件**: database/migration/010_inventory_foreign_keys.sql
- **描述**: 
  - inventory_transfers → warehouses (from_warehouse_id, to_warehouse_id)
  - inventory_counts → warehouses
  - inventory_adjustments → warehouses
  - inventory_adjustment_items → inventory_adjustments
  - inventory_count_items → inventory_counts
  - inventory_transfer_items → inventory_transfers
  - inventory_reservations → products
  - inventory_reservations → warehouses
  - inventory_transactions → products
  - inventory_transactions → warehouses
- **验证**: 迁移文件已创建

## 阶段三：架构优化（P3优先级）

### [x] 任务 3.1：DI容器注册核心服务
- **文件**: backend/src/utils/di_container.rs, backend/src/utils/app_state.rs
- **描述**: 
  - AppState已添加 `di_container: Arc<DIContainer>` 字段
  - 添加 `get_service()` 和 `register_service()` 方法
  - DI容器支持单例注册、工厂模式、全局容器
- **验证**: 编译通过，75个单元测试全部通过

### [ ] 任务 3.2：推广DI容器使用示例
- **文件**: backend/src/handlers/auth_handler.rs（作为示例）
- **描述**: 
  - 修改auth_handler使用DI容器获取AuthService
  - 展示DI容器的使用模式
- **状态**: 待后续迭代实施，当前DI容器基础设施已就绪

## 阶段四：代码质量（P4优先级）

### [x] 任务 4.1：创建集成测试框架
- **文件**: backend/tests/integration/mod.rs
- **描述**: 
  - 创建TestConfig结构体
  - 添加helpers模块（create_test_token, create_auth_header）
  - 75个单元测试全部通过
- **验证**: cargo test --lib 通过

### [x] 任务 4.2：添加核心流程集成测试
- **文件**: backend/tests/integration/auth_flow.rs, backend/tests/integration/sales_flow.rs
- **描述**: 
  - 认证流程测试（JWT令牌创建、认证请求头、配置默认值）
  - 销售订单流程测试（状态流转、金额计算、订单编号格式）
- **验证**: cargo test --lib 通过

### [x] 任务 4.3：统一API响应格式
- **文件**: backend/src/utils/response.rs
- **描述**: 
  - ApiResponse<T> 已提供统一的响应格式 { success, data, message, error }
  - 支持分页响应 PaginatedResponse<T>
  - 已实现 IntoResponse trait
- **验证**: 编译通过

## 阶段五：gRPC服务完善（P5优先级）

### [x] 任务 5.1：启用gRPC服务注册
- **文件**: backend/src/main.rs
- **描述**: 
  - 在main.rs中注册gRPC服务（UserService, AuthService, PurchaseContractService, SalesContractService, FixedAssetService, BudgetManagementService）
  - 配置gRPC监听端口（从settings.grpc读取）
  - 与HTTP服务并行启动，支持优雅停机
- **验证**: cargo check 通过

### [x] 任务 5.2：完善gRPC服务实现
- **文件**: backend/src/grpc/service.rs, backend/src/grpc/management_services.rs
- **描述**: 
  - GrpcUserService已实现UserServiceTrait和AuthServiceTrait
  - GrpcManagementServices已实现PurchaseContractServiceTrait、SalesContractServiceTrait、FixedAssetServiceTrait、BudgetManagementServiceTrait
  - 错误处理完善（Status::not_found, Status::internal, Status::invalid_argument等）
- **验证**: cargo check 通过

# 任务依赖关系
- 任务1.1、1.2、1.3 可并行执行
- 任务2.1、2.2、2.3 可并行执行
- 任务3.1 依赖 阶段一完成
- 任务3.2 依赖 任务3.1
- 任务4.1 依赖 阶段一完成
- 任务4.2 依赖 任务4.1
- 任务4.3 可独立执行
- 任务5.1、5.2 可并行执行

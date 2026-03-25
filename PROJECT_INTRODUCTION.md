# 秉羲 ERP 系统 - 项目完整介绍

## 📍 一、项目位置

### 1.1 文件系统路径
- **项目根目录**: `E:\1\10\bingxi-rust`
- **后端代码**: `E:\1\10\bingxi-rust\backend`
- **前端代码**: `E:\1\10\bingxi-rust\frontend`
- **文档资料**: `E:\1\10\bingxi-rust\docs`
- **规格文档**: `E:\1\10\bingxi-rust\specs`
- **部署脚本**: `E:\1\10\bingxi-rust\deploy`

---

## 📁 二、项目结构与命名规范

### 2.1 整体目录结构

```
bingxi-rust/
├── backend/                    # 后端服务（Rust + Axum）
│   ├── src/
│   │   ├── config/            # 配置管理
│   │   │   ├── mod.rs         # 模块导出
│   │   │   └── settings.rs    # 配置结构定义
│   │   ├── database/          # 数据库连接
│   │   │   └── mod.rs         # 数据库连接管理
│   │   ├── handlers/          # API 请求处理器
│   │   │   ├── mod.rs         # 模块导出（62 个 handler）
│   │   │   ├── auth_handler.rs
│   │   │   ├── user_handler.rs
│   │   │   ├── product_handler.rs
│   │   │   └── ... (共 62 个 handler 文件)
│   │   ├── middleware/        # 中间件
│   │   │   ├── mod.rs
│   │   │   ├── auth_middleware.rs
│   │   │   ├── logger_middleware.rs
│   │   │   └── ... (共 8 个中间件)
│   │   ├── models/            # SeaORM 模型定义
│   │   │   ├── mod.rs         # 模块导出（92 个模型）
│   │   │   ├── user.rs
│   │   │   ├── product.rs
│   │   │   └── ... (共 92 个模型文件)
│   │   ├── routes/            # 路由配置
│   │   │   └── mod.rs         # 统一路由定义
│   │   ├── services/          # 业务逻辑层
│   │   │   ├── mod.rs         # 模块导出（58 个 service）
│   │   │   ├── auth_service.rs
│   │   │   ├── user_service.rs
│   │   │   └── ... (共 58 个 service 文件)
│   │   │   └── tests/         # Service 层测试
│   │   │       ├── mod.rs
│   │   │       ├── auth_service_test.rs
│   │   │       └── ... (7 个测试文件)
│   │   ├── utils/             # 工具函数
│   │   │   ├── mod.rs
│   │   │   ├── error.rs       # 错误类型定义
│   │   │   └── response.rs    # 响应格式封装
│   │   ├── grpc/              # gRPC 服务
│   │   │   ├── mod.rs
│   │   │   ├── management_services.rs
│   │   │   └── proto/         # Protobuf 定义
│   │   ├── lib.rs             # 库根
│   │   └── main.rs            # 程序入口
│   ├── tests/                 # 集成测试
│   │   ├── api_test.rs
│   │   ├── auth_integration_test.rs
│   │   └── ... (7 个集成测试文件)
│   ├── Cargo.toml             # 依赖配置
│   └── .env.example           # 环境变量示例
├── frontend/                   # 前端应用（Yew + WASM）
│   ├── src/
│   │   ├── app/               # 应用根组件
│   │   ├── components/        # UI 组件
│   │   ├── pages/             # 页面组件
│   │   ├── services/          # API 服务
│   │   ├── models/            # 数据模型
│   │   └── main.rs            # 程序入口
│   └── Cargo.toml
├── specs/                      # 项目规格文档
│   ├── 面料行业全模块适配方案.md
│   ├── 47 个模块需求分析文档.md
│   └── features/              # 功能需求文档
│       ├── M019_总账管理_功能需求规格说明书.md
│       ├── M020_采购管理_功能需求规格说明书.md
│       └── ... (18 个功能文档)
├── docs/                       # 项目文档
│   ├── README.md              # 项目说明
│   ├── api-docs.md            # API 接口文档
│   ├── quickstart.md          # 快速启动指南
│   └── project-structure.md   # 项目结构说明
└── deploy/                     # 部署脚本
```

### 2.2 命名规范

#### 文件命名
- **Handler 文件**: `{模块名}_handler.rs`
  - 示例：`product_handler.rs`, `sales_order_handler.rs`
- **Service 文件**: `{模块名}_service.rs`
  - 示例：`product_service.rs`, `sales_service.rs`
- **Model 文件**: `{模块名}.rs`
  - 示例：`product.rs`, `sales_order.rs`
- **中间件文件**: `{功能名}_middleware.rs`
  - 示例：`auth_middleware.rs`, `logger_middleware.rs`

#### 代码命名
- **变量和函数**: snake_case（下划线命名）
  - 示例：`create_product`, `get_user_by_id`
- **结构体和枚举**: PascalCase（大驼峰）
  - 示例：`Product`, `CreateProductRequest`
- **常量**: UPPER_SNAKE_CASE
  - 示例：`MAX_PAGE_SIZE`, `DEFAULT_TOKEN_EXPIRY`
- **模块路径**: snake_case
  - 示例：`handlers::product_handler`

#### 注释规范
- **所有代码注释使用中文**
- **文档注释使用 rustdoc 格式**
  ```rust
  /// 创建新产品
  /// 
  /// # 参数
  /// - `req`: 创建产品请求
  /// - `user_id`: 创建者用户 ID
  /// 
  /// # 返回
  /// 创建成功的产品对象
  pub async fn create_product(...) -> Result<Product, AppError>
  ```

---

## 🔌 三、API 接口详细说明

### 3.1 接口规范

#### 基础路径
- **所有 API 接口**: `/api/v1/erp/*`

#### 请求方法
- `GET`: 查询操作
- `POST`: 创建操作
- `PUT`: 更新操作
- `DELETE`: 删除操作

#### 响应格式
```rust
// 成功响应
{
    "code": 200,
    "message": "操作成功",
    "data": { ... }
}

// 错误响应
{
    "code": 400,
    "message": "错误描述",
    "data": null
}
```

### 3.2 核心 API 接口列表

#### 1. 认证模块 (Auth)
**文件路径**: `backend/src/handlers/auth_handler.rs`

| 接口 | 方法 | 功能 | 请求参数 | 返回值 |
|------|------|------|----------|--------|
| `/api/v1/erp/auth/login` | POST | 用户登录 | `{username, password}` | `{token, user_info}` |
| `/api/v1/erp/auth/logout` | POST | 用户登出 | `{token}` | `{"message": "登出成功"}` |
| `/api/v1/erp/auth/refresh` | POST | 刷新令牌 | `{refresh_token}` | `{new_token}` |

#### 2. 用户管理模块 (User)
**文件路径**: `backend/src/handlers/user_handler.rs`

| 接口 | 方法 | 功能 | 请求参数 | 返回值 |
|------|------|------|----------|--------|
| `/api/v1/erp/users` | GET | 获取用户列表 | `?page=1&page_size=10&search=` | `PaginatedResponse<User>` |
| `/api/v1/erp/users/:id` | GET | 获取用户详情 | Path: `id` | `User` |
| `/api/v1/erp/users` | POST | 创建用户 | `CreateUserRequest` | `User` |
| `/api/v1/erp/users/:id` | PUT | 更新用户 | Path: `id`, Body: `UpdateUserRequest` | `User` |
| `/api/v1/erp/users/:id` | DELETE | 删除用户 | Path: `id` | `{"message": "删除成功"}` |

#### 3. 产品管理模块 (Product)
**文件路径**: `backend/src/handlers/product_handler.rs`

| 接口 | 方法 | 功能 | 请求参数 | 返回值 |
|------|------|------|----------|--------|
| `/api/v1/erp/products` | GET | 获取产品列表 | `?page=1&page_size=10&category_id=&search=` | `PaginatedResponse<Product>` |
| `/api/v1/erp/products/:id` | GET | 获取产品详情 | Path: `id` | `Product` |
| `/api/v1/erp/products` | POST | 创建产品 | `CreateProductRequest` | `Product` |
| `/api/v1/erp/products/:id` | PUT | 更新产品 | Path: `id`, Body: `UpdateProductRequest` | `Product` |
| `/api/v1/erp/products/:id` | DELETE | 删除产品 | Path: `id` | `{"message": "删除成功"}` |
| `/api/v1/erp/products/:id/colors` | GET | 获取产品色号列表 | Path: `id` | `Vec<ProductColor>` |
| `/api/v1/erp/products/:id/colors` | POST | 创建产品色号 | Path: `id`, Body: `CreateProductColorRequest` | `ProductColor` |

#### 4. 销售订单模块 (Sales Order)
**文件路径**: `backend/src/handlers/sales_order_handler.rs`

| 接口 | 方法 | 功能 | 请求参数 | 返回值 |
|------|------|------|----------|--------|
| `/api/v1/erp/sales/orders` | GET | 获取订单列表 | `?page=1&customer_id=&status=` | `PaginatedResponse<SalesOrder>` |
| `/api/v1/erp/sales/orders/:id` | GET | 获取订单详情 | Path: `id` | `SalesOrder` |
| `/api/v1/erp/sales/orders` | POST | 创建订单 | `CreateSalesOrderRequest` | `SalesOrder` |
| `/api/v1/erp/sales/orders/:id` | PUT | 更新订单 | Path: `id`, Body: `UpdateSalesOrderRequest` | `SalesOrder` |
| `/api/v1/erp/sales/orders/:id` | DELETE | 删除订单 | Path: `id` | `{"message": "删除成功"}` |

#### 5. 库存管理模块 (Inventory)
**文件路径**: `backend/src/handlers/inventory_stock_handler.rs`

| 接口 | 方法 | 功能 | 请求参数 | 返回值 |
|------|------|------|----------|--------|
| `/api/v1/erp/inventory/stock` | GET | 获取库存列表 | `?warehouse_id=&batch_no=&color_no=` | `Vec<InventoryStock>` |
| `/api/v1/erp/inventory/stock/:id` | GET | 获取库存详情 | Path: `id` | `InventoryStock` |
| `/api/v1/erp/inventory/stock` | POST | 创建库存 | `CreateInventoryStockRequest` | `InventoryStock` |

#### 6. 仓库管理模块 (Warehouse)
**文件路径**: `backend/src/handlers/warehouse_handler.rs`

| 接口 | 方法 | 功能 | 请求参数 | 返回值 |
|------|------|------|----------|--------|
| `/api/v1/erp/warehouses` | GET | 获取仓库列表 | `?page=1&type=` | `PaginatedResponse<Warehouse>` |
| `/api/v1/erp/warehouses/:id` | GET | 获取仓库详情 | Path: `id` | `Warehouse` |
| `/api/v1/erp/warehouses` | POST | 创建仓库 | `CreateWarehouseRequest` | `Warehouse` |
| `/api/v1/erp/warehouses/locations` | GET | 获取库位列表 | `?warehouse_id=` | `Vec<Location>` |
| `/api/v1/erp/warehouses/locations` | POST | 创建库位 | `CreateLocationRequest` | `Location` |

### 3.3 完整模块列表（62 个 Handler）

#### 基础管理（9 个）
1. `auth_handler.rs` - 认证
2. `user_handler.rs` - 用户
3. `role_handler.rs` - 角色
4. `department_handler.rs` - 部门
5. `dashboard_handler.rs` - 仪表板
6. `health_handler.rs` - 健康检查
7. `batch_handler.rs` - 批量处理（旧）
8. `batch_new_handler.rs` - 批量处理（新）
9. `customer_handler.rs` - 客户

#### 产品与仓库（7 个）
10. `product_handler.rs` - 产品
11. `product_category_handler.rs` - 产品分类
12. `warehouse_handler.rs` - 仓库
13. `inventory_stock_handler.rs` - 库存
14. `inventory_transfer_handler.rs` - 库存调拨
15. `inventory_count_handler.rs` - 库存盘点
16. `inventory_adjustment_handler.rs` - 库存调整

#### 销售管理（5 个）
17. `sales_order_handler.rs` - 销售订单
18. `sales_fabric_order_handler.rs` - 面料订单
19. `sales_contract_handler.rs` - 销售合同
20. `sales_analysis_handler.rs` - 销售分析
21. `sales_price_handler.rs` - 销售价格

#### 采购管理（6 个）
22. `purchase_order_handler.rs` - 采购订单
23. `purchase_receipt_handler.rs` - 采购入库
24. `purchase_return_handler.rs` - 采购退货
25. `purchase_inspection_handler.rs` - 采购质检
26. `purchase_contract_handler.rs` - 采购合同
27. `purchase_price_handler.rs` - 采购价格

#### 供应商管理（2 个）
28. `supplier_handler.rs` - 供应商
29. `supplier_evaluation_handler.rs` - 供应商评估

#### 财务管理（8 个）
30. `finance_payment_handler.rs` - 财务付款
31. `finance_invoice_handler.rs` - 财务发票
32. `ap_invoice_handler.rs` - 应付单
33. `ap_payment_request_handler.rs` - 付款申请
34. `ap_payment_handler.rs` - 应付付款
35. `ap_verification_handler.rs` - 应付核销
36. `ap_reconciliation_handler.rs` - 供应商对账
37. `ap_report_handler.rs` - 应付报表

#### 应收管理（1 个）
38. `ar_invoice_handler.rs` - 应收单

#### 总账管理（2 个）
39. `account_subject_handler.rs` - 会计科目
40. `voucher_handler.rs` - 凭证

#### 成本管理（1 个）
41. `cost_collection_handler.rs` - 成本归集

#### 固定资产（1 个）
42. `fixed_asset_handler.rs` - 固定资产

#### 客户信用（1 个）
43. `customer_credit_handler.rs` - 客户信用

#### 资金管理（1 个）
44. `fund_management_handler.rs` - 资金管理

#### 预算管理（1 个）
45. `budget_management_handler.rs` - 预算管理

#### 质量标准（1 个）
46. `quality_standard_handler.rs` - 质量标准

#### 财务分析（1 个）
47. `financial_analysis_handler.rs` - 财务分析

#### 质量检验（1 个）
48. `quality_inspection_handler.rs` - 质量检验

#### 辅助核算（4 个）
49. `dual_unit_converter_handler.rs` - 双单位转换
50. `five_dimension_handler.rs` - 五维查询
51. `assist_accounting_handler.rs` - 辅助核算
52. `business_trace_handler.rs` - 业务追溯

#### P1P2 模块（14 个）
53. `p1p2_handlers.rs` - P1P2 综合处理

---

## 📦 四、功能模块实现文件及路径

### 4.1 Handler 层（API 处理器）
**路径**: `backend/src/handlers/`

| 模块 | Handler 文件 | Service 依赖 | Model 依赖 |
|------|-------------|-------------|-----------|
| 用户管理 | `user_handler.rs` | `UserService` | `User` |
| 产品管理 | `product_handler.rs` | `ProductService` | `Product`, `ProductColor` |
| 销售订单 | `sales_order_handler.rs` | `SalesService` | `SalesOrder`, `SalesOrderItem` |
| 库存管理 | `inventory_stock_handler.rs` | `InventoryStockService` | `InventoryStock` |
| 采购管理 | `purchase_order_handler.rs` | `PurchaseOrderService` | `PurchaseOrder` |
| 供应商 | `supplier_handler.rs` | `SupplierService` | `Supplier` |
| 应付管理 | `ap_invoice_handler.rs` | `ApInvoiceService` | `ApInvoice` |
| 应收管理 | `ar_invoice_handler.rs` | `ArInvoiceService` | `ArInvoice` |
| 总账 | `account_subject_handler.rs` | `AccountSubjectService` | `AccountSubject` |
| 凭证 | `voucher_handler.rs` | `VoucherService` | `Voucher`, `VoucherItem` |

### 4.2 Service 层（业务逻辑）
**路径**: `backend/src/services/`

#### 核心 Service（58 个）
- `auth_service.rs` - 认证服务（JWT 令牌生成与验证）
- `user_service.rs` - 用户服务（CRUD 操作）
- `product_service.rs` - 产品服务（面料行业适配）
- `sales_service.rs` - 销售服务（订单全流程）
- `inventory_stock_service.rs` - 库存服务（批次 + 色号管理）
- `purchase_order_service.rs` - 采购服务
- `supplier_service.rs` - 供应商服务
- `ap_invoice_service.rs` - 应付服务
- `ar_invoice_service.rs` - 应收服务
- `voucher_service.rs` - 凭证服务
- ... (共 58 个 Service)

### 4.3 Model 层（数据模型）
**路径**: `backend/src/models/`

#### 核心 Model（92 个）
- `user.rs` - 用户模型
- `product.rs` - 产品模型（含面料行业字段）
- `sales_order.rs` - 销售订单模型
- `inventory_stock.rs` - 库存模型（含批次 + 色号）
- `purchase_order.rs` - 采购订单模型
- `supplier.rs` - 供应商模型
- `ap_invoice.rs` - 应付单模型
- `ar_invoice.rs` - 应收单模型
- `voucher.rs` - 凭证模型
- ... (共 92 个 Model)

---

## 🧪 五、测试文档

### 5.1 测试文件位置

#### 单元测试（Service 层）
**路径**: `backend/src/services/tests/`

| 测试文件 | 测试内容 | 状态 |
|---------|---------|------|
| `auth_service_test.rs` | 认证服务测试 | ✅ 已实现 |
| `user_service_test.rs` | 用户服务测试 | ✅ 已实现 |
| `bpm_service_test.rs` | 流程服务测试 | ✅ 已实现 |
| `code_conversion_service_test.rs` | 编码转换测试 | ✅ 已实现 |
| `four_level_batch_service_test.rs` | 四级批次测试 | ✅ 已实现 |
| `log_service_test.rs` | 日志服务测试 | ✅ 已实现 |

#### 集成测试
**路径**: `backend/tests/`

| 测试文件 | 测试内容 | 状态 |
|---------|---------|------|
| `api_test.rs` | API 接口测试 | ✅ 已实现 |
| `auth_integration_test.rs` | 认证集成测试 | ✅ 已实现 |
| `user_integration_test.rs` | 用户集成测试 | ✅ 已实现 |
| `batch_management_integration_test.rs` | 批次管理集成测试 | ✅ 已实现 |
| `grpc_test.rs` | gRPC 服务测试 | ✅ 已实现 |
| `management_services_integration_test.rs` | 管理服务集成测试 | ✅ 已实现 |

### 5.2 测试运行方法

#### 运行所有测试
```bash
cd backend
cargo test
```

#### 运行特定测试
```bash
# 运行认证测试
cargo test auth

# 运行用户服务测试
cargo test user_service

# 运行集成测试
cargo test --test auth_integration_test
```

#### 生成测试覆盖率报告
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

---

## 📋 六、项目规划文档

### 6.1 需求文档
**路径**: `specs/`

| 文档名称 | 文件路径 | 内容 |
|---------|---------|------|
| 面料行业全模块适配方案 | `specs/面料行业全模块适配方案.md` | 面料行业 65 个模块完整适配方案 |
| 47 个模块需求分析 | `specs/47 个模块需求分析文档.md` | 47 个业务模块需求分析 |
| 采购管理需求 | `specs/features/M020_采购管理_功能需求规格说明书.md` | 采购管理模块详细需求 |
| 总账管理需求 | `specs/features/M019_总账管理_功能需求规格说明书.md` | 总账管理模块详细需求 |
| 应付管理需求 | `specs/features/03_应付管理_需求文档.md` | 应付管理模块需求 |

### 6.2 技术方案文档
**路径**: `specs/features/`

| 文档名称 | 文件路径 |
|---------|---------|
| 供应商管理技术方案 | `specs/features/01_供应商管理_技术方案.md` |
| 采购管理技术方案 | `specs/features/02_采购管理_技术方案.md` |
| 应付管理技术方案 | `specs/features/03_应付管理_技术方案.md` |
| 总账管理技术方案 | `specs/features/04_总账管理_技术方案.md` |

### 6.3 任务规划文档
**路径**: `specs/features/`

| 文档名称 | 文件路径 |
|---------|---------|
| 供应商管理任务规划 | `specs/features/01_供应商管理_任务规划.md` |
| 采购管理任务规划 | `specs/features/02_采购管理_任务规划.md` |
| 应付管理任务规划 | `specs/features/03_应付管理_任务规划.md` |
| 第三阶段优化改进任务 | `specs/features/第三阶段_优化改进_任务规划.md` |

### 6.4 项目文档
**路径**: `docs/`

| 文档名称 | 文件路径 | 内容 |
|---------|---------|------|
| 项目说明 | `docs/README.md` | 项目整体介绍 |
| API 文档 | `docs/api-docs.md` | 完整 API 接口文档 |
| 快速启动 | `docs/quickstart.md` | 开发环境搭建指南 |
| 项目结构 | `docs/project-structure.md` | 项目文件结构说明 |
| 迁移指南 | `docs/migration-guide.md` | 数据迁移方案 |
| 完成总结 | `docs/completion-summary.md` | 项目完成情况总结 |
| 增强总结 | `docs/enhancement-summary.md` | 功能增强总结 |

---

## 🛠️ 七、技术栈详情

### 7.1 后端技术栈
- **Web 框架**: Axum 0.7 + Tokio 1.0
- **ORM**: SeaORM 1.0
- **数据库**: PostgreSQL 18.0
- **认证**: JWT (jsonwebtoken 9.0)
- **密码加密**: bcrypt 0.15
- **gRPC**: Tonic 0.10 + Prost 0.12
- **日志**: tracing + tracing-subscriber
- **配置**: config 0.14 + dotenvy 0.15
- **验证**: validator 0.16
- **错误处理**: thiserror 1.0 + anyhow 1.0
- **API 文档**: utoipa 4.0 + Swagger UI

### 7.2 前端技术栈
- **框架**: Yew 0.21 (Rust + WASM)
- **构建工具**: Trunk 0.20
- **路由**: yew-router 0.17
- **HTTP 客户端**: gloo-net 0.4

---

## 📊 八、项目完成度

### 8.1 整体进度
- **总体完成度**: 75%
- **后端核心**: 85% ✅
- **前端功能**: 80% ✅
- **业务模块**: 70% ✅
- **文档完善**: 90% ✅
- **测试覆盖**: 60% ⏳

### 8.2 已实现模块（62 个 Handler）
- ✅ 基础管理：9 个
- ✅ 产品与仓库：7 个
- ✅ 销售管理：5 个
- ✅ 采购管理：6 个
- ✅ 供应商管理：2 个
- ✅ 财务管理：8 个
- ✅ 应收应付：9 个
- ✅ 总账管理：2 个
- ✅ 成本管理：1 个
- ✅ 其他模块：23 个

---

## 🚀 九、快速启动

### 9.1 环境要求
- Rust 1.70+
- PostgreSQL 18.0
- Trunk 0.20（前端构建）

### 9.2 后端启动
```bash
cd backend
cp .env.example .env
# 编辑 .env 配置数据库
cargo run
```

### 9.3 前端启动
```bash
cargo install trunk
cd frontend
trunk serve --open
```

---

## 📞 十、联系方式

- **项目主页**: https://github.com/boshi-xixixi
- **问题反馈**: 提交 Issue
- **许可证**: Copyright © 2024 秉羲团队

---

**文档版本**: v1.0  
**最后更新**: 2026-03-21  
**维护者**: 秉羲团队

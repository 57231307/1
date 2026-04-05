# 秉羲管理系统

## 目录
1. [项目概述](#项目概述)
2. [系统架构](#系统架构)
3. [技术栈详解](#技术栈详解)
4. [功能模块详解](#功能模块详解)
5. [数据库设计](#数据库设计)
6. [API接口文档](#api接口文档)
7. [部署指南](#部署指南)
8. [开发指南](#开发指南)
9. [安全机制](#安全机制)
10. [监控与运维](#监控与运维)

---

## 项目概述

秉羲管理系统是一个专为面料行业定制的企业资源规划（ERP）系统，采用现代化的技术栈，提供完整的企业管理功能。系统采用前后端分离架构，支持多租户、模块化设计，具有高性能、高安全性、高可扩展性等特点。

### 核心特性

- **完整的ERP功能：从采购到销售，从库存到财务，从生产到分析
- **面料行业特色功能：批次管理、双计量单位、缸号管理、色号管理、坯布管理
- **高性能：基于Rust语言开发，性能优异**
- **高安全性：JWT认证、权限管理、请求验证、CORS配置**
- **高可扩展性：模块化设计，易于添加新功能**
- **完整的监控体系：Prometheus + Grafana + Alertmanager**

---

## 系统架构

### 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                         前端层                           │
│  ┌─────────────────┐  ┌─────────────────┐          │
│  │  Yew (WebAssembly) │  │  桌面客户端      │          │
│  │  - 路由管理       │  │  - gRPC客户端    │          │
│  │  - 状态管理       │  │  - 实时通信      │          │
│  └─────────────────┘  └─────────────────┘          │
└──────────────────────┬──────────────────────────────────┘
                       │ HTTP + WebSocket
                       │ gRPC
┌──────────────────────┴──────────────────────────────────┐
│                      Nginx 反向代理                           │
│  - 静态资源托管                                      │
│  - API 反向代理                                      │
│  - 负载均衡                                          │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────┐
│                      后端层 (Axum)                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │  │
│  │  │中间件│  │  路由层  │  │  处理层  │          │  │
│  │  │  └──┘  └────┬─────┘  └────┬─────┘          │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │          业务逻辑层                    │  │  │
│  │  │  (Services)                          │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │          数据访问层                    │  │  │
│  │  │  (SeaORM)                        │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────┐
│                      数据层                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  PostgreSQL  │  │   Redis     │  │  Prometheus  │ │
│  │   (主数据库)  │  │   (缓存)    │  │  (监控数据)  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### 架构设计原则

1. **前后端分离**：前端使用Yew WebAssembly，后端使用Axum，通过HTTP和gRPC通信
2. **分层架构**：中间件层、路由层、处理层、业务逻辑层、数据访问层
3. **模块化设计**：每个功能模块独立，易于扩展和维护
4. **高可用性**：支持负载均衡、缓存、监控等

---

## 技术栈详解

### 后端技术栈

| 技术 | 版本 | 用途 | 文件位置 |
|------|------|------|---------|
| Rust | 2021 | 后端开发语言 | [backend/Cargo.toml](file:///workspace/backend/Cargo.toml) |
| Axum | 0.7 | Web框架 | [backend/src/main.rs](file:///workspace/backend/src/main.rs) |
| SeaORM | - | 数据库ORM | [backend/src/database/mod.rs](file:///workspace/backend/src/database/mod.rs) |
| PostgreSQL | 14+ | 关系型数据库 | [backend/.env.example](file:///workspace/backend/.env.example) |
| Tonic | - | gRPC框架 | [backend/proto/bingxi.proto](file:///workspace/backend/proto/bingxi.proto) |
| JWT | - | 身份认证 | [backend/src/services/auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) |
| Tracing | - | 日志追踪 | [backend/src/middleware/logger_middleware.rs](file:///workspace/backend/src/middleware/logger_middleware.rs) |
| Prometheus | - | 指标收集 | [backend/src/middleware/metrics.rs](file:///workspace/backend/src/middleware/metrics.rs) |

### 前端技术栈

| 技术 | 版本 | 用途 | 文件位置 |
|------|------|------|---------|
| Rust | 2021 | 前端开发语言 | [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml) |
| Yew | 0.21 | WebAssembly框架 | [frontend/src/main.rs](file:///workspace/frontend/src/main.rs) |
| Yew Router | - | 路由管理 | [frontend/src/app/mod.rs](file:///workspace/frontend/src/app/mod.rs) |
| Gloo Net | - | HTTP客户端 | [frontend/src/services/api.rs](file:///workspace/frontend/src/services/api.rs) |
| Trunk | - | 构建工具 | [frontend/Trunk.toml](file:///workspace/frontend/Trunk.toml) |
| Utoo | - | 统一前端工具链 | [frontend/.utoo.toml](file:///workspace/frontend/.utoo.toml) |

### 部署与监控技术栈

| 技术 | 用途 | 文件位置 |
|------|------|---------|
| Nginx | 反向代理和静态资源托管 | [deploy/nginx.conf](file:///workspace/deploy/nginx.conf) |
| Systemd | 系统服务管理 | [deploy/bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service) |
| Prometheus | 指标收集 | [monitoring/prometheus/prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml) |
| Grafana | 指标可视化 | [monitoring/grafana/dashboards/bingxi-erp-overview.json](file:///workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json) |
| Alertmanager | 告警管理 | [monitoring/alertmanager/alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml) |

---

## 功能模块详解

### 1. 用户与权限管理模块

#### 1.1 用户管理

**功能描述**：
- 用户CRUD操作
- 用户信息管理
- 用户状态管理
- 用户密码重置

**核心模型**：
- [user.rs](file:///workspace/backend/src/models/user.rs) - 用户模型
- [user_service.rs](file:///workspace/backend/src/services/user_service.rs) - 用户服务
- [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) - 用户处理函数

**前端页面**：
- [user_list.rs](file:///workspace/frontend/src/pages/user_list.rs) - 用户列表页面

**API接口**：
- `GET /api/v1/erp/users` - 获取用户列表
- `POST /api/v1/erp/users` - 创建用户
- `GET /api/v1/erp/users/:id` - 获取用户详情
- `PUT /api/v1/erp/users/:id` - 更新用户
- `DELETE /api/v1/erp/users/:id` - 删除用户

#### 1.2 角色管理

**功能描述**：
- 角色CRUD操作
- 角色权限分配
- 角色层级管理

**核心模型**：
- [role.rs](file:///workspace/backend/src/models/role.rs) - 角色模型
- [role_permission.rs](file:///workspace/backend/src/models/role_permission.rs) - 角色权限模型
- [role_permission_service.rs](file:///workspace/backend/src/services/role_permission_service.rs) - 角色权限服务
- [role_handler.rs](file:///workspace/backend/src/handlers/role_handler.rs) - 角色处理函数

**前端页面**：
- [role_list.rs](file:///workspace/frontend/src/pages/role_list.rs) - 角色列表页面

**API接口**：
- `GET /api/v1/erp/roles` - 获取角色列表
- `POST /api/v1/erp/roles` - 创建角色
- `GET /api/v1/erp/roles/:id` - 获取角色详情
- `PUT /api/v1/erp/roles/:id` - 更新角色
- `DELETE /api/v1/erp/roles/:id` - 删除角色

#### 1.3 部门管理

**功能描述**：
- 部门CRUD操作
- 部门层级管理
- 部门用户关联

**核心模型**：
- [department.rs](file:///workspace/backend/src/models/department.rs) - 部门模型
- [department_service.rs](file:///workspace/backend/src/services/department_service.rs) - 部门服务
- [department_handler.rs](file:///workspace/backend/src/handlers/department_handler.rs) - 部门处理函数

**前端页面**：
- [department_list.rs](file:///workspace/frontend/src/pages/department_list.rs) - 部门列表页面

**API接口**：
- `GET /api/v1/erp/departments` - 获取部门列表
- `POST /api/v1/erp/departments` - 创建部门
- `GET /api/v1/erp/departments/:id` - 获取部门详情
- `PUT /api/v1/erp/departments/:id` - 更新部门
- `DELETE /api/v1/erp/departments/:id` - 删除部门

#### 1.4 认证服务

**功能描述**：
- 用户登录
- 用户注销
- JWT令牌生成
- JWT令牌验证
- 令牌刷新

**核心模型**：
- [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) - 认证服务
- [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) - 认证处理函数
- [auth.rs](file:///workspace/backend/src/middleware/auth.rs) - 认证中间件
- [auth_context.rs](file:///workspace/backend/src/middleware/auth_context.rs) - 认证上下文

**前端页面**：
- [login.rs](file:///workspace/frontend/src/pages/login.rs) - 登录页面

**API接口**：
- `POST /api/v1/erp/auth/login` - 用户登录
- `POST /api/v1/erp/auth/logout` - 用户注销
- `POST /api/v1/erp/auth/refresh` - 刷新令牌

### 2. 产品管理模块

#### 2.1 产品管理

**功能描述**：
- 产品CRUD操作
- 产品分类管理
- 产品规格管理
- 产品状态管理

**核心模型**：
- [product.rs](file:///workspace/backend/src/models/product.rs) - 产品模型
- [product_service.rs](file:///workspace/backend/src/services/product_service.rs) - 产品服务
- [product_handler.rs](file:///workspace/backend/src/handlers/product_handler.rs) - 产品处理函数

**前端页面**：
- [product_list.rs](file:///workspace/frontend/src/pages/product_list.rs) - 产品列表页面

**API接口**：
- `GET /api/v1/erp/products` - 获取产品列表
- `POST /api/v1/erp/products` - 创建产品
- `GET /api/v1/erp/products/:id` - 获取产品详情
- `PUT /api/v1/erp/products/:id` - 更新产品
- `DELETE /api/v1/erp/products/:id` - 删除产品

#### 2.2 产品分类管理

**功能描述**：
- 产品分类CRUD操作
- 分类层级管理
- 分类状态管理

**核心模型**：
- [product_category.rs](file:///workspace/backend/src/models/product_category.rs) - 产品分类模型
- [product_category_service.rs](file:///workspace/backend/src/services/product_category_service.rs) - 产品分类服务
- [product_category_handler.rs](file:///workspace/backend/src/handlers/product_category_handler.rs) - 产品分类处理函数

**前端页面**：
- [product_category.rs](file:///workspace/frontend/src/pages/product_category.rs) - 产品分类页面

**API接口**：
- `GET /api/v1/erp/product-categories` - 获取产品分类列表
- `POST /api/v1/erp/product-categories` - 创建产品分类
- `GET /api/v1/erp/product-categories/:id` - 获取产品分类详情
- `PUT /api/v1/erp/product-categories/:id` - 更新产品分类
- `DELETE /api/v1/erp/product-categories/:id` - 删除产品分类

#### 2.3 产品颜色管理

**功能描述**：
- 产品颜色CRUD操作
- 色号管理
- 颜色状态管理

**核心模型**：
- [product_color.rs](file:///workspace/backend/src/models/product_color.rs) - 产品颜色模型
- [color_code_mapping.rs](file:///workspace/backend/src/models/color_code_mapping.rs) - 色号映射模型

#### 2.4 产品编码映射

**功能描述**：
- 产品编码映射管理
- 内部产品与外部产品映射
- 色号映射
- 匹号映射

**核心模型**：
- [product_code_mapping.rs](file:///workspace/backend/src/models/product_code_mapping.rs) - 产品编码映射模型
- [piece_mapping.rs](file:///workspace/backend/src/models/piece_mapping.rs) - 匹号映射模型

### 3. 仓库管理模块

#### 3.1 仓库管理

**功能描述**：
- 仓库CRUD操作
- 仓库信息管理
- 仓库状态管理
- 仓库位置管理

**核心模型**：
- [warehouse.rs](file:///workspace/backend/src/models/warehouse.rs) - 仓库模型
- [location.rs](file:///workspace/backend/src/models/location.rs) - 仓库位置模型
- [warehouse_service.rs](file:///workspace/backend/src/services/warehouse_service.rs) - 仓库服务
- [warehouse_handler.rs](file:///workspace/backend/src/handlers/warehouse_handler.rs) - 仓库处理函数

**前端页面**：
- [warehouse_list.rs](file:///workspace/frontend/src/pages/warehouse_list.rs) - 仓库列表页面

**API接口**：
- `GET /api/v1/erp/warehouses` - 获取仓库列表
- `POST /api/v1/erp/warehouses` - 创建仓库
- `GET /api/v1/erp/warehouses/:id` - 获取仓库详情
- `PUT /api/v1/erp/warehouses/:id` - 更新仓库
- `DELETE /api/v1/erp/warehouses/:id` - 删除仓库

### 4. 库存管理模块

#### 4.1 库存查询

**功能描述**：
- 库存列表查询
- 库存统计
- 库存明细查询
- 库存预警

**核心模型**：
- [inventory_stock.rs](file:///workspace/backend/src/models/inventory_stock.rs) - 库存模型
- [inventory_transaction.rs](file:///workspace/backend/src/models/inventory_transaction.rs) - 库存交易模型
- [inventory_stock_service.rs](file:///workspace/backend/src/services/inventory_stock_service.rs) - 库存服务
- [inventory_stock_handler.rs](file:///workspace/backend/src/handlers/inventory_stock_handler.rs) - 库存处理函数

**前端页面**：
- [inventory_stock.rs](file:///workspace/frontend/src/pages/inventory_stock.rs) - 库存页面

**API接口**：
- `GET /api/v1/erp/inventory/stock` - 获取库存列表
- `POST /api/v1/erp/inventory/stock` - 创建库存记录
- `GET /api/v1/erp/inventory/stock/:id` - 获取库存详情

#### 4.2 库存调拨

**功能描述**：
- 调拨单创建
- 调拨单审批
- 调拨单执行
- 调拨单查询

**核心模型**：
- [inventory_transfer.rs](file:///workspace/backend/src/models/inventory_transfer.rs) - 库存调拨模型
- [inventory_transfer_item.rs](file:///workspace/backend/src/models/inventory_transfer_item.rs) - 库存调拨明细模型
- [inventory_transfer_service.rs](file:///workspace/backend/src/services/inventory_transfer_service.rs) - 库存调拨服务
- [inventory_transfer_handler.rs](file:///workspace/backend/src/handlers/inventory_transfer_handler.rs) - 库存调拨处理函数

**前端页面**：
- [inventory_transfer.rs](file:///workspace/frontend/src/pages/inventory_transfer.rs) - 库存调拨页面

**API接口**：
- `GET /api/v1/erp/inventory/transfers` - 获取调拨列表
- `POST /api/v1/erp/inventory/transfers` - 创建调拨单
- `GET /api/v1/erp/inventory/transfers/:id` - 获取调拨详情

#### 4.3 库存盘点

**功能描述**：
- 盘点单创建
- 盘点单执行
- 盘点差异处理
- 盘点单查询

**核心模型**：
- [inventory_count.rs](file:///workspace/backend/src/models/inventory_count.rs) - 库存盘点模型
- [inventory_count_item.rs](file:///workspace/backend/src/models/inventory_count_item.rs) - 库存盘点明细模型
- [inventory_count_service.rs](file:///workspace/backend/src/services/inventory_count_service.rs) - 库存盘点服务
- [inventory_count_handler.rs](file:///workspace/backend/src/handlers/inventory_count_handler.rs) - 库存盘点处理函数

**前端页面**：
- [inventory_count.rs](file:///workspace/frontend/src/pages/inventory_count.rs) - 库存盘点页面

**API接口**：
- `GET /api/v1/erp/inventory/counts` - 获取盘点列表
- `POST /api/v1/erp/inventory/counts` - 创建盘点单
- `GET /api/v1/erp/inventory/counts/:id` - 获取盘点详情

#### 4.4 库存调整

**功能描述**：
- 调整单创建
- 调整单审批
- 调整单执行
- 调整单查询

**核心模型**：
- [inventory_adjustment.rs](file:///workspace/backend/src/models/inventory_adjustment.rs) - 库存调整模型
- [inventory_adjustment_item.rs](file:///workspace/backend/src/models/inventory_adjustment_item.rs) - 库存调整明细模型
- [inventory_adjustment_service.rs](file:///workspace/backend/src/services/inventory_adjustment_service.rs) - 库存调整服务
- [inventory_adjustment_handler.rs](file:///workspace/backend/src/handlers/inventory_adjustment_handler.rs) - 库存调整处理函数

**前端页面**：
- [inventory_adjustment.rs](file:///workspace/frontend/src/pages/inventory_adjustment.rs) - 库存调整页面

**API接口**：
- `GET /api/v1/erp/inventory/adjustments` - 获取调整列表
- `POST /api/v1/erp/inventory/adjustments` - 创建调整单
- `GET /api/v1/erp/inventory/adjustments/:id` - 获取调整详情

#### 4.5 库存预留

**功能描述**：
- 库存预留创建
- 库存预留释放
- 库存预留查询

**核心模型**：
- [inventory_reservation.rs](file:///workspace/backend/src/models/inventory_reservation.rs) - 库存预留模型
- [inventory_reservation_service.rs](file:///workspace/backend/src/services/inventory_reservation_service.rs) - 库存预留服务

#### 4.6 匹数管理

**功能描述**：
- 匹数信息管理
- 匹数映射管理

**核心模型**：
- [inventory_piece.rs](file:///workspace/backend/src/models/inventory_piece.rs) - 库存匹数模型

#### 4.7 面料行业特色库存功能

**功能描述**：
- 批次管理
- 色号管理
- 缸号管理
- 双计量单位（米/公斤）自动换算
- 坯布管理

**核心模型**：
- [batch_service.rs](file:///workspace/backend/src/services/batch_service.rs) - 批次服务
- [batch_handler.rs](file:///workspace/backend/src/handlers/batch_handler.rs) - 批次处理函数
- [dual_unit_converter_service.rs](file:///workspace/backend/src/services/dual_unit_converter_service.rs) - 双计量单位转换服务
- [dual_unit_converter_handler.rs](file:///workspace/backend/src/handlers/dual_unit_converter_handler.rs) - 双计量单位转换处理函数

**前端页面**：
- [batch.rs](file:///workspace/frontend/src/pages/batch.rs) - 批次页面
- [dual_unit_converter.rs](file:///workspace/frontend/src/pages/dual_unit_converter.rs) - 双计量单位转换页面

### 5. 销售管理模块

#### 5.1 销售订单管理

**功能描述**：
- 销售订单创建
- 销售订单审批
- 销售订单执行
- 销售订单查询
- 销售订单状态管理

**核心模型**：
- [sales_order.rs](file:///workspace/backend/src/models/sales_order.rs) - 销售订单模型
- [sales_order_item.rs](file:///workspace/backend/src/models/sales_order_item.rs) - 销售订单明细模型
- [sales_service.rs](file:///workspace/backend/src/services/sales_service.rs) - 销售服务
- [sales_order_handler.rs](file:///workspace/backend/src/handlers/sales_order_handler.rs) - 销售订单处理函数

**前端页面**：
- [sales_order.rs](file:///workspace/frontend/src/pages/sales_order.rs) - 销售订单页面

**API接口**：
- `GET /api/v1/erp/sales/orders` - 获取销售订单列表
- `POST /api/v1/erp/sales/orders` - 创建销售订单
- `GET /api/v1/erp/sales/orders/:id` - 获取销售订单详情
- `PUT /api/v1/erp/sales/orders/:id` - 更新销售订单
- `DELETE /api/v1/erp/sales/orders/:id` - 删除销售订单

#### 5.2 面料销售订单

**功能描述**：
- 面料销售订单创建
- 面料销售订单审批
- 面料销售订单执行
- 面料销售订单查询
- 双计量单位支持
- 色号管理
- 批次追踪

**核心模型**：
- [sales_fabric_order_handler.rs](file:///workspace/backend/src/handlers/sales_fabric_order_handler.rs) - 面料销售订单处理函数

**前端页面**：
- [fabric_order.rs](file:///workspace/frontend/src/pages/fabric_order.rs) - 面料订单页面

#### 5.3 销售合同管理

**功能描述**：
- 销售合同创建
- 销售合同审批
- 销售合同执行
- 销售合同查询

**核心模型**：
- [sales_contract.rs](file:///workspace/backend/src/models/sales_contract.rs) - 销售合同模型
- [sales_contract_service.rs](file:///workspace/backend/src/services/sales_contract_service.rs) - 销售合同服务
- [sales_contract_handler.rs](file:///workspace/backend/src/handlers/sales_contract_handler.rs) - 销售合同处理函数

**前端页面**：
- [sales_contract.rs](file:///workspace/frontend/src/pages/sales_contract.rs) - 销售合同页面

**API接口**：
- `GET /api/v1/erp/sales/contracts` - 获取销售合同列表
- `POST /api/v1/erp/sales/contracts` - 创建销售合同

#### 5.4 销售价格管理

**功能描述**：
- 销售价格创建
- 销售价格查询
- 销售价格调整
- 销售价格历史

**核心模型**：
- [sales_price.rs](file:///workspace/backend/src/models/sales_price.rs) - 销售价格模型
- [sales_price_service.rs](file:///workspace/backend/src/services/sales_price_service.rs) - 销售价格服务
- [sales_price_handler.rs](file:///workspace/backend/src/handlers/sales_price_handler.rs) - 销售价格处理函数

**前端页面**：
- [sales_price.rs](file:///workspace/frontend/src/pages/sales_price.rs) - 销售价格页面

#### 5.5 销售分析

**功能描述**：
- 销售数据分析
- 销售报表生成
- 销售趋势分析
- 客户销售分析

**核心模型**：
- [sales_analysis.rs](file:///workspace/backend/src/models/sales_analysis.rs) - 销售分析模型
- [sales_analysis_service.rs](file:///workspace/backend/src/services/sales_analysis_service.rs) - 销售分析服务
- [sales_analysis_handler.rs](file:///workspace/backend/src/handlers/sales_analysis_handler.rs) - 销售分析处理函数

**前端页面**：
- [sales_analysis.rs](file:///workspace/frontend/src/pages/sales_analysis.rs) - 销售分析页面

#### 5.6 销售交货管理

**功能描述**：
- 销售交货单创建
- 销售交货单执行
- 销售交货单查询

**核心模型**：
- [sales_delivery.rs](file:///workspace/backend/src/models/sales_delivery.rs) - 销售交货模型
- [sales_delivery_item.rs](file:///workspace/backend/src/models/sales_delivery_item.rs) - 销售交货明细模型

### 6. 采购管理模块

#### 6.1 采购订单管理

**功能描述**：
- 采购订单创建
- 采购订单审批
- 采购订单执行
- 采购订单查询
- 采购订单状态管理
- 供应商产品转换

**核心模型**：
- [purchase_order.rs](file:///workspace/backend/src/models/purchase_order.rs) - 采购订单模型
- [purchase_order_item.rs](file:///workspace/backend/src/models/purchase_order_item.rs) - 采购订单明细模型
- [purchase_order_service.rs](file:///workspace/backend/src/services/purchase_order_service.rs) - 采购订单服务
- [purchase_order_handler.rs](file:///workspace/backend/src/handlers/purchase_order_handler.rs) - 采购订单处理函数

**前端页面**：
- [purchase_order.rs](file:///workspace/frontend/src/pages/purchase_order.rs) - 采购订单页面

**API接口**：
- `GET /api/v1/erp/purchase/orders` - 获取采购订单列表
- `POST /api/v1/erp/purchase/orders` - 创建采购订单
- `GET /api/v1/erp/purchase/orders/:id` - 获取采购订单详情

#### 6.2 采购收货管理

**功能描述**：
- 采购收货单创建
- 采购收货单执行
- 采购收货单查询
- 采购收货单验收

**核心模型**：
- [purchase_receipt.rs](file:///workspace/backend/src/models/purchase_receipt.rs) - 采购收货模型
- [purchase_receipt_item.rs](file:///workspace/backend/src/models/purchase_receipt_item.rs) - 采购收货明细模型
- [purchase_receipt_service.rs](file:///workspace/backend/src/services/purchase_receipt_service.rs) - 采购收货服务
- [purchase_receipt_handler.rs](file:///workspace/backend/src/handlers/purchase_receipt_handler.rs) - 采购收货处理函数

**前端页面**：
- [purchase_receipt.rs](file:///workspace/frontend/src/pages/purchase_receipt.rs) - 采购收货页面

**API接口**：
- `GET /api/v1/erp/purchase/receipts` - 获取采购收货列表
- `POST /api/v1/erp/purchase/receipts` - 创建采购收货单

#### 6.3 采购退货管理

**功能描述**：
- 采购退货单创建
- 采购退货单执行
- 采购退货单查询

**核心模型**：
- [purchase_return.rs](file:///workspace/backend/src/models/purchase_return.rs) - 采购退货模型
- [purchase_return_service.rs](file:///workspace/backend/src/services/purchase_return_service.rs) - 采购退货服务
- [purchase_return_handler.rs](file:///workspace/backend/src/handlers/purchase_return_handler.rs) - 采购退货处理函数

**前端页面**：
- [purchase_return.rs](file:///workspace/frontend/src/pages/purchase_return.rs) - 采购退货页面

#### 6.4 采购合同管理

**功能描述**：
- 采购合同创建
- 采购合同审批
- 采购合同执行
- 采购合同查询

**核心模型**：
- [purchase_contract.rs](file:///workspace/backend/src/models/purchase_contract.rs) - 采购合同模型
- [purchase_contract_execution.rs](file:///workspace/backend/src/models/purchase_contract_execution.rs) - 采购合同执行模型
- [purchase_contract_service.rs](file:///workspace/backend/src/services/purchase_contract_service.rs) - 采购合同服务
- [purchase_contract_handler.rs](file:///workspace/backend/src/handlers/purchase_contract_handler.rs) - 采购合同处理函数

**前端页面**：
- [purchase_contract.rs](file:///workspace/frontend/src/pages/purchase_contract.rs) - 采购合同页面

#### 6.5 采购价格管理

**功能描述**：
- 采购价格创建
- 采购价格查询
- 采购价格调整
- 采购价格历史

**核心模型**：
- [purchase_price.rs](file:///workspace/backend/src/models/purchase_price.rs) - 采购价格模型
- [purchase_price_service.rs](file:///workspace/backend/src/services/purchase_price_service.rs) - 采购价格服务
- [purchase_price_handler.rs](file:///workspace/backend/src/handlers/purchase_price_handler.rs) - 采购价格处理函数

**前端页面**：
- [purchase_price.rs](file:///workspace/frontend/src/pages/purchase_price.rs) - 采购价格页面

#### 6.6 采购检验管理

**功能描述**：
- 采购检验单创建
- 采购检验单执行
- 采购检验单查询
- 不合格品处理

**核心模型**：
- [purchase_inspection.rs](file:///workspace/backend/src/models/purchase_inspection.rs) - 采购检验模型
- [purchase_inspection_service.rs](file:///workspace/backend/src/services/purchase_inspection_service.rs) - 采购检验服务
- [purchase_inspection_handler.rs](file:///workspace/backend/src/handlers/purchase_inspection_handler.rs) - 采购检验处理函数

**前端页面**：
- [purchase_inspection.rs](file:///workspace/frontend/src/pages/purchase_inspection.rs) - 采购检验页面

### 7. 供应商管理模块

#### 7.1 供应商管理

**功能描述**：
- 供应商CRUD操作
- 供应商信息管理
- 供应商状态管理
- 供应商黑名单管理
- 供应商类别管理

**核心模型**：
- [supplier.rs](file:///workspace/backend/src/models/supplier.rs) - 供应商模型
- [supplier_blacklist.rs](file:///workspace/backend/src/models/supplier_blacklist.rs) - 供应商黑名单模型
- [supplier_category.rs](file:///workspace/backend/src/models/supplier_category.rs) - 供应商类别模型
- [supplier_contact.rs](file:///workspace/backend/src/models/supplier_contact.rs) - 供应商联系人模型
- [supplier_qualification.rs](file:///workspace/backend/src/models/supplier_qualification.rs) - 供应商资质模型
- [supplier_service.rs](file:///workspace/backend/src/services/supplier_service.rs) - 供应商服务
- [supplier_handler.rs](file:///workspace/backend/src/handlers/supplier_handler.rs) - 供应商处理函数

**前端页面**：
- [supplier.rs](file:///workspace/frontend/src/pages/supplier.rs) - 供应商页面

**API接口**：
- `GET /api/v1/erp/suppliers` - 获取供应商列表
- `POST /api/v1/erp/suppliers` - 创建供应商
- `GET /api/v1/erp/suppliers/:id` - 获取供应商详情
- `PUT /api/v1/erp/suppliers/:id` - 更新供应商
- `DELETE /api/v1/erp/suppliers/:id` - 删除供应商

#### 7.2 供应商产品管理

**功能描述**：
- 供应商产品管理
- 供应商产品颜色管理
- 供应商产品与内部产品映射
- 供应商产品色号映射

**核心模型**：
- [supplier_product.rs](file:///workspace/backend/src/models/supplier_product.rs) - 供应商产品模型
- [supplier_product_color.rs](file:///workspace/backend/src/models/supplier_product_color.rs) - 供应商产品颜色模型
- [product_supplier_mapping.rs](file:///workspace/backend/src/models/product_supplier_mapping.rs) - 产品供应商映射模型

**数据库迁移**：
- [061_supplier_product_mapping.sql](file:///workspace/backend/database/migration/061_supplier_product_mapping.sql) - 供应商产品映射数据库迁移

#### 7.3 供应商评估管理

**功能描述**：
- 供应商评估创建
- 供应商评估记录
- 供应商等级管理
- 供应商评估查询

**核心模型**：
- [supplier_evaluation.rs](file:///workspace/backend/src/models/supplier_evaluation.rs) - 供应商评估模型
- [supplier_evaluation_record.rs](file:///workspace/backend/src/models/supplier_evaluation_record.rs) - 供应商评估记录模型
- [supplier_grade.rs](file:///workspace/backend/src/models/supplier_grade.rs) - 供应商等级模型
- [supplier_evaluation_service.rs](file:///workspace/backend/src/services/supplier_evaluation_service.rs) - 供应商评估服务
- [supplier_evaluation_handler.rs](file:///workspace/backend/src/handlers/supplier_evaluation_handler.rs) - 供应商评估处理函数

**前端页面**：
- [supplier_evaluation.rs](file:///workspace/frontend/src/pages/supplier_evaluation.rs) - 供应商评估页面

### 8. 客户管理模块

#### 8.1 客户管理

**功能描述**：
- 客户CRUD操作
- 客户信息管理
- 客户状态管理

**核心模型**：
- [customer.rs](file:///workspace/backend/src/models/customer.rs) - 客户模型
- [customer_service.rs](file:///workspace/backend/src/services/customer_service.rs) - 客户服务
- [customer_handler.rs](file:///workspace/backend/src/handlers/customer_handler.rs) - 客户处理函数

**前端页面**：
- [customer.rs](file:///workspace/frontend/src/pages/customer.rs) - 客户页面

**API接口**：
- `GET /api/v1/erp/customers` - 获取客户列表
- `POST /api/v1/erp/customers` - 创建客户
- `GET /api/v1/erp/customers/:id` - 获取客户详情
- `PUT /api/v1/erp/customers/:id` - 更新客户
- `DELETE /api/v1/erp/customers/:id` - 删除客户

#### 8.2 客户信用管理

**功能描述**：
- 客户信用额度管理
- 客户信用状态管理
- 客户信用查询

**核心模型**：
- [customer_credit.rs](file:///workspace/backend/src/models/customer_credit.rs) - 客户信用模型
- [customer_credit_service.rs](file:///workspace/backend/src/services/customer_credit_service.rs) - 客户信用服务
- [customer_credit_handler.rs](file:///workspace/backend/src/handlers/customer_credit_handler.rs) - 客户信用处理函数

**前端页面**：
- [customer_credit.rs](file:///workspace/frontend/src/pages/customer_credit.rs) - 客户信用页面

### 9. 财务管理模块

#### 9.1 总账管理

**功能描述**：
- 会计科目管理
- 会计期间管理
- 财务凭证管理
- 科目余额管理

**核心模型**：
- [account_subject.rs](file:///workspace/backend/src/models/account_subject.rs) - 会计科目模型
- [accounting_period.rs](file:///workspace/backend/src/models/accounting_period.rs) - 会计期间模型
- [voucher.rs](file:///workspace/backend/src/models/voucher.rs) - 财务凭证模型
- [voucher_item.rs](file:///workspace/backend/src/models/voucher_item.rs) - 财务凭证明细模型
- [account_balance.rs](file:///workspace/backend/src/models/account_balance.rs) - 科目余额模型
- [account_subject_service.rs](file:///workspace/backend/src/services/account_subject_service.rs) - 会计科目服务
- [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) - 财务凭证服务
- [account_subject_handler.rs](file:///workspace/backend/src/handlers/account_subject_handler.rs) - 会计科目处理函数
- [voucher_handler.rs](file:///workspace/backend/src/handlers/voucher_handler.rs) - 财务凭证处理函数

**前端页面**：
- [account_subject.rs](file:///workspace/frontend/src/pages/account_subject.rs) - 会计科目页面
- [voucher.rs](file:///workspace/frontend/src/pages/voucher.rs) - 财务凭证页面

**API接口**：
- `GET /api/v1/erp/finance/account-subjects` - 获取会计科目列表
- `POST /api/v1/erp/finance/account-subjects` - 创建会计科目
- `GET /api/v1/erp/finance/vouchers` - 获取财务凭证列表
- `POST /api/v1/erp/finance/vouchers` - 创建财务凭证

#### 9.2 应付账款管理

**功能描述**：
- 应付发票管理
- 应付付款管理
- 应付付款申请管理
- 应付对账管理
- 应付核销管理
- 应付报表管理

**核心模型**：
- [ap_invoice.rs](file:///workspace/backend/src/models/ap_invoice.rs) - 应付发票模型
- [ap_payment.rs](file:///workspace/backend/src/models/ap_payment.rs) - 应付付款模型
- [ap_payment_request.rs](file:///workspace/backend/src/models/ap_payment_request.rs) - 应付付款申请模型
- [ap_payment_request_item.rs](file:///workspace/backend/src/models/ap_payment_request_item.rs) - 应付付款申请明细模型
- [ap_reconciliation.rs](file:///workspace/backend/src/models/ap_reconciliation.rs) - 应付对账模型
- [ap_verification.rs](file:///workspace/backend/src/models/ap_verification.rs) - 应付核销模型
- [ap_verification_item.rs](file:///workspace/backend/src/models/ap_verification_item.rs) - 应付核销明细模型
- [ap_invoice_service.rs](file:///workspace/backend/src/services/ap_invoice_service.rs) - 应付发票服务
- [ap_payment_service.rs](file:///workspace/backend/src/services/ap_payment_service.rs) - 应付付款服务
- [ap_payment_request_service.rs](file:///workspace/backend/src/services/ap_payment_request_service.rs) - 应付付款申请服务
- [ap_reconciliation_service.rs](file:///workspace/backend/src/services/ap_reconciliation_service.rs) - 应付对账服务
- [ap_verification_service.rs](file:///workspace/backend/src/services/ap_verification_service.rs) - 应付核销服务
- [ap_report_service.rs](file:///workspace/backend/src/services/ap_report_service.rs) - 应付报表服务
- [ap_invoice_handler.rs](file:///workspace/backend/src/handlers/ap_invoice_handler.rs) - 应付发票处理函数
- [ap_payment_handler.rs](file:///workspace/backend/src/handlers/ap_payment_handler.rs) - 应付付款处理函数
- [ap_payment_request_handler.rs](file:///workspace/backend/src/handlers/ap_payment_request_handler.rs) - 应付付款申请处理函数
- [ap_reconciliation_handler.rs](file:///workspace/backend/src/handlers/ap_reconciliation_handler.rs) - 应付对账处理函数
- [ap_verification_handler.rs](file:///workspace/backend/src/handlers/ap_verification_handler.rs) - 应付核销处理函数
- [ap_report_handler.rs](file:///workspace/backend/src/handlers/ap_report_handler.rs) - 应付报表处理函数

**前端页面**：
- [ap_invoice.rs](file:///workspace/frontend/src/pages/ap_invoice.rs) - 应付发票页面
- [ap_payment.rs](file:///workspace/frontend/src/pages/ap_payment.rs) - 应付付款页面
- [ap_payment_request.rs](file:///workspace/frontend/src/pages/ap_payment_request.rs) - 应付付款申请页面
- [ap_reconciliation.rs](file:///workspace/frontend/src/pages/ap_reconciliation.rs) - 应付对账页面
- [ap_verification.rs](file:///workspace/frontend/src/pages/ap_verification.rs) - 应付核销页面
- [ap_report.rs](file:///workspace/frontend/src/pages/ap_report.rs) - 应付报表页面

#### 9.3 应收账款管理

**功能描述**：
- 应收发票管理
- 应收收款管理
- 应收对账管理

**核心模型**：
- [ar_invoice.rs](file:///workspace/backend/src/models/ar_invoice.rs) - 应收发票模型
- [ar_collection.rs](file:///workspace/backend/src/models/ar_collection.rs) - 应收收款模型
- [ar_mod.rs](file:///workspace/backend/src/models/ar_mod.rs) - 应收模型
- [ar_invoice_service.rs](file:///workspace/backend/src/services/ar_invoice_service.rs) - 应收发票服务
- [ar_invoice_handler.rs](file:///workspace/backend/src/handlers/ar_invoice_handler.rs) - 应收发票处理函数

**前端页面**：
- [ar_invoice.rs](file:///workspace/frontend/src/pages/ar_invoice.rs) - 应收发票页面

#### 9.4 财务发票管理

**功能描述**：
- 财务发票创建
- 财务发票查询
- 财务发票状态管理

**核心模型**：
- [finance_invoice.rs](file:///workspace/backend/src/models/finance_invoice.rs) - 财务发票模型
- [finance_invoice_service.rs](file:///workspace/backend/src/services/finance_invoice_service.rs) - 财务发票服务
- [finance_invoice_handler.rs](file:///workspace/backend/src/handlers/finance_invoice_handler.rs) - 财务发票处理函数

**前端页面**：
- [finance_invoice.rs](file:///workspace/frontend/src/pages/finance_invoice.rs) - 财务发票页面

#### 9.5 财务付款管理

**功能描述**：
- 财务付款创建
- 财务付款查询
- 财务付款状态管理

**核心模型**：
- [finance_payment.rs](file:///workspace/backend/src/models/finance_payment.rs) - 财务付款模型
- [finance_payment_service.rs](file:///workspace/backend/src/services/finance_payment_service.rs) - 财务付款服务
- [finance_payment_handler.rs](file:///workspace/backend/src/handlers/finance_payment_handler.rs) - 财务付款处理函数

**前端页面**：
- [finance_payment.rs](file:///workspace/frontend/src/pages/finance_payment.rs) - 财务付款页面

#### 9.6 成本管理

**功能描述**：
- 成本归集
- 成本分析
- 成本核算

**核心模型**：
- [cost_collection.rs](file:///workspace/backend/src/models/cost_collection.rs) - 成本归集模型
- [cost_analysis.rs](file:///workspace/backend/src/models/cost_analysis.rs) - 成本分析模型
- [cost_mod.rs](file:///workspace/backend/src/models/cost_mod.rs) - 成本模型
- [cost_collection_service.rs](file:///workspace/backend/src/services/cost_collection_service.rs) - 成本归集服务
- [cost_collection_handler.rs](file:///workspace/backend/src/handlers/cost_collection_handler.rs) - 成本归集处理函数

**前端页面**：
- [cost_collection.rs](file:///workspace/frontend/src/pages/cost_collection.rs) - 成本归集页面

#### 9.7 财务分析

**功能描述**：
- 财务数据分析
- 财务报表生成
- 财务指标分析

**核心模型**：
- [financial_analysis.rs](file:///workspace/backend/src/models/financial_analysis.rs) - 财务分析模型
- [financial_analysis_result.rs](file:///workspace/backend/src/models/financial_analysis_result.rs) - 财务分析结果模型
- [financial_analysis_service.rs](file:///workspace/backend/src/services/financial_analysis_service.rs) - 财务分析服务
- [financial_analysis_handler.rs](file:///workspace/backend/src/handlers/financial_analysis_handler.rs) - 财务分析处理函数

**前端页面**：
- [financial_analysis.rs](file:///workspace/frontend/src/pages/financial_analysis.rs) - 财务分析页面

### 10. 生产管理模块

#### 10.1 染色批次管理

**功能描述**：
- 染色批次创建
- 染色批次执行
- 染色批次查询
- 染色批次状态管理

**核心模型**：
- [dye_batch.rs](file:///workspace/backend/src/models/dye_batch.rs) - 染色批次模型
- [batch_dye_lot.rs](file:///workspace/backend/src/models/batch_dye_lot.rs) - 批次染缸模型
- [dye_lot_mapping.rs](file:///workspace/backend/src/models/dye_lot_mapping.rs) - 染缸映射模型
- [dye_batch_handler.rs](file:///workspace/backend/src/handlers/dye_batch_handler.rs) - 染色批次处理函数

**前端页面**：
- [dye_batch.rs](file:///workspace/frontend/src/pages/dye_batch.rs) - 染色批次页面

#### 10.2 染色配方管理

**功能描述**：
- 染色配方创建
- 染色配方查询
- 染色配方调整

**核心模型**：
- [dye_recipe.rs](file:///workspace/backend/src/models/dye_recipe.rs) - 染色配方模型
- [dye_recipe_handler.rs](file:///workspace/backend/src/handlers/dye_recipe_handler.rs) - 染色配方处理函数

**前端页面**：
- [dye_recipe.rs](file:///workspace/frontend/src/pages/dye_recipe.rs) - 染色配方页面

#### 10.3 坯布管理

**功能描述**：
- 坯布信息管理
- 坯布批次管理
- 坯布查询

**核心模型**：
- [greige_fabric.rs](file:///workspace/backend/src/models/greige_fabric.rs) - 坯布模型
- [greige_fabric_handler.rs](file:///workspace/backend/src/handlers/greige_fabric_handler.rs) - 坯布处理函数

**前端页面**：
- [greige_fabric.rs](file:///workspace/frontend/src/pages/greige_fabric.rs) - 坯布页面

### 11. 辅助核算模块

**功能描述**：
- 辅助核算维度管理
- 辅助核算记录管理
- 辅助核算汇总管理

**核心模型**：
- [assist_accounting_dimension.rs](file:///workspace/backend/src/models/assist_accounting_dimension.rs) - 辅助核算维度模型
- [assist_accounting_record.rs](file:///workspace/backend/src/models/assist_accounting_record.rs) - 辅助核算记录模型
- [assist_accounting_summary.rs](file:///workspace/backend/src/models/assist_accounting_summary.rs) - 辅助核算汇总模型
- [assist_accounting_service.rs](file:///workspace/backend/src/services/assist_accounting_service.rs) - 辅助核算服务
- [assist_accounting_handler.rs](file:///workspace/backend/src/handlers/assist_accounting_handler.rs) - 辅助核算处理函数

**前端页面**：
- [assist_accounting.rs](file:///workspace/frontend/src/pages/assist_accounting.rs) - 辅助核算页面

### 12. 业务追溯模块

**功能描述**：
- 五维度管理（产品、批次、色号、等级、仓库）
- 业务流程追溯
- 数据快照
- 业务追溯链条
- 业务追溯视图

**核心模型**：
- [business_trace.rs](file:///workspace/backend/src/models/business_trace.rs) - 业务追溯模型
- [business_trace_chain.rs](file:///workspace/backend/src/models/business_trace_chain.rs) - 业务追溯链条模型
- [business_trace_snapshot.rs](file:///workspace/backend/src/models/business_trace_snapshot.rs) - 业务追溯快照模型
- [business_trace_view.rs](file:///workspace/backend/src/models/business_trace_view.rs) - 业务追溯视图模型
- [business_trace_assist_link.rs](file:///workspace/backend/src/models/business_trace_assist_link.rs) - 业务追溯辅助链接模型
- [batch_trace_log.rs](file:///workspace/backend/src/models/batch_trace_log.rs) - 批次追溯日志模型
- [business_trace_service.rs](file:///workspace/backend/src/services/business_trace_service.rs) - 业务追溯服务
- [five_dimension_query_service.rs](file:///workspace/backend/src/services/five_dimension_query_service.rs) - 五维度查询服务
- [business_trace_handler.rs](file:///workspace/backend/src/handlers/business_trace_handler.rs) - 业务追溯处理函数

**前端页面**：
- [business_trace.rs](file:///workspace/frontend/src/pages/business_trace.rs) - 业务追溯页面
- [five_dimension.rs](file:///workspace/frontend/src/pages/five_dimension.rs) - 五维度页面

### 13. 预算管理模块

**功能描述**：
- 预算计划管理
- 预算执行管理
- 预算调整管理
- 预算查询

**核心模型**：
- [budget_management.rs](file:///workspace/backend/src/models/budget_management.rs) - 预算管理模型
- [budget_plan.rs](file:///workspace/backend/src/models/budget_plan.rs) - 预算计划模型
- [budget_execution.rs](file:///workspace/backend/src/models/budget_execution.rs) - 预算执行模型
- [budget_adjustment.rs](file:///workspace/backend/src/models/budget_adjustment.rs) - 预算调整模型
- [budget_management_service.rs](file:///workspace/backend/src/services/budget_management_service.rs) - 预算管理服务
- [budget_management_handler.rs](file:///workspace/backend/src/handlers/budget_management_handler.rs) - 预算管理处理函数

**前端页面**：
- [budget_management.rs](file:///workspace/frontend/src/pages/budget_management.rs) - 预算管理页面

### 14. 固定资产管理模块

**功能描述**：
- 固定资产信息管理
- 固定资产折旧管理
- 固定资产处置管理
- 固定资产查询

**核心模型**：
- [fixed_asset.rs](file:///workspace/backend/src/models/fixed_asset.rs) - 固定资产模型
- [fixed_asset_disposal.rs](file:///workspace/backend/src/models/fixed_asset_disposal.rs) - 固定资产处置模型
- [fixed_asset_service.rs](file:///workspace/backend/src/services/fixed_asset_service.rs) - 固定资产服务
- [fixed_asset_handler.rs](file:///workspace/backend/src/handlers/fixed_asset_handler.rs) - 固定资产处理函数

**前端页面**：
- [fixed_asset.rs](file:///workspace/frontend/src/pages/fixed_asset.rs) - 固定资产页面

### 15. 资金管理模块

**功能描述**：
- 资金账户管理
- 资金转账记录管理
- 资金查询
- 资金统计

**核心模型**：
- [fund_management.rs](file:///workspace/backend/src/models/fund_management.rs) - 资金管理模型
- [fund_account.rs](file:///workspace/backend/src/models/fund_account.rs) - 资金账户模型
- [fund_transfer_record.rs](file:///workspace/backend/src/models/fund_transfer_record.rs) - 资金转账记录模型
- [fund_management_service.rs](file:///workspace/backend/src/services/fund_management_service.rs) - 资金管理服务
- [fund_management_handler.rs](file:///workspace/backend/src/handlers/fund_management_handler.rs) - 资金管理处理函数

**前端页面**：
- [fund_management.rs](file:///workspace/frontend/src/pages/fund_management.rs) - 资金管理页面

### 16. 质量管理模块

#### 16.1 质量检验管理

**功能描述**：
- 质量检验单创建
- 质量检验单执行
- 质量检验单查询
- 不合格品管理

**核心模型**：
- [quality_inspection.rs](file:///workspace/backend/src/models/quality_inspection.rs) - 质量检验模型
- [quality_inspection_record.rs](file:///workspace/backend/src/models/quality_inspection_record.rs) - 质量检验记录模型
- [unqualified_product.rs](file:///workspace/backend/src/models/unqualified_product.rs) - 不合格品模型
- [quality_inspection_service.rs](file:///workspace/backend/src/services/quality_inspection_service.rs) - 质量检验服务
- [quality_inspection_handler.rs](file:///workspace/backend/src/handlers/quality_inspection_handler.rs) - 质量检验处理函数

**前端页面**：
- [quality_inspection.rs](file:///workspace/frontend/src/pages/quality_inspection.rs) - 质量检验页面

#### 16.2 质量标准管理

**功能描述**：
- 质量标准创建
- 质量标准查询
- 质量标准调整

**核心模型**：
- [quality_standard.rs](file:///workspace/backend/src/models/quality_standard.rs) - 质量标准模型
- [quality_standard_service.rs](file:///workspace/backend/src/services/quality_standard_service.rs) - 质量标准服务
- [quality_standard_handler.rs](file:///workspace/backend/src/handlers/quality_standard_handler.rs) - 质量标准处理函数

### 17. CRM模块

**功能描述**：
- 客户线索管理
- 客户机会管理

**核心模型**：
- [crm_lead.rs](file:///workspace/backend/src/models/crm_lead.rs) - 客户线索模型
- [crm_opportunity.rs](file:///workspace/backend/src/models/crm_opportunity.rs) - 客户机会模型

### 18. OA模块

**功能描述**：
- 公告管理

**核心模型**：
- [oa_announcement.rs](file:///workspace/backend/src/models/oa_announcement.rs) - 公告模型

### 19. BPM流程模块

**功能描述**：
- 流程定义管理
- 流程实例管理
- 流程任务管理

**核心模型**：
- [bpm_process_definition.rs](file:///workspace/backend/src/models/bpm_process_definition.rs) - 流程定义模型
- [bpm_process_instance.rs](file:///workspace/backend/src/models/bpm_process_instance.rs) - 流程实例模型
- [bpm_task.rs](file:///workspace/backend/src/models/bpm_task.rs) - 流程任务模型

### 20. 报表模块

**功能描述**：
- 报表定义管理

**核心模型**：
- [report_definition.rs](file:///workspace/backend/src/models/report_definition.rs) - 报表定义模型

### 21. 日志模块

**功能描述**：
- API访问日志
- 登录日志
- 系统日志

**核心模型**：
- [log_api_access.rs](file:///workspace/backend/src/models/log_api_access.rs) - API访问日志模型
- [log_login.rs](file:///workspace/backend/src/models/log_login.rs) - 登录日志模型
- [log_system.rs](file:///workspace/backend/src/models/log_system.rs) - 系统日志模型

### 22. 仪表盘模块

**功能描述**：
- 仪表盘数据展示
- 业务数据统计
- 数据可视化

**核心模型**：
- [dashboard_service.rs](file:///workspace/backend/src/services/dashboard_service.rs) - 仪表盘服务
- [dashboard_handler.rs](file:///workspace/backend/src/handlers/dashboard_handler.rs) - 仪表盘处理函数

**前端页面**：
- [dashboard.rs](file:///workspace/frontend/src/pages/dashboard.rs) - 仪表盘页面

### 23. 初始化模块

**功能描述**：
- 系统初始化
- 数据库初始化
- 配置初始化

**核心模型**：
- [init_service.rs](file:///workspace/backend/src/services/init_service.rs) - 初始化服务
- [init_handler.rs](file:///workspace/backend/src/handlers/init_handler.rs) - 初始化处理函数

**前端页面**：
- [init.rs](file:///workspace/frontend/src/pages/init.rs) - 初始化页面

### 24. 系统更新模块

**功能描述**：
- 系统版本管理
- 系统更新检查
- 系统更新下载
- 系统更新执行

**核心模型**：
- [system_version.rs](file:///workspace/backend/src/models/system_version.rs) - 系统版本模型
- [system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs) - 系统更新服务
- [system_update_handler.rs](file:///workspace/backend/src/handlers/system_update_handler.rs) - 系统更新处理函数

### 25. 操作日志模块

**功能描述**：
- 操作日志记录
- 操作日志查询
- 操作日志统计

**核心模型**：
- [operation_log.rs](file:///workspace/backend/src/models/operation_log.rs) - 操作日志模型
- [operation_log_service.rs](file:///workspace/backend/src/services/operation_log_service.rs) - 操作日志服务
- [operation_log.rs](file:///workspace/backend/src/middleware/operation_log.rs) - 操作日志中间件

---

## 数据库设计

### 数据库迁移文件

系统使用数据库迁移文件来管理数据库结构，所有迁移文件位于 [backend/database/migration/](file:///workspace/backend/database/migration/) 目录。

主要迁移文件包括：
- [001_init.sql](file:///workspace/backend/database/migration/001_init.sql) - 初始数据库结构
- [002_inventory_reservation.sql](file:///workspace/backend/database/migration/002_inventory_reservation.sql) - 库存预留
- [004_customers.sql](file:///workspace/backend/database/migration/004_customers.sql) - 客户管理
- [005_fabric_industry_adaptation.sql](file:///workspace/backend/database/migration/005_fabric_industry_adaptation.sql) - 面料行业适配
- [006_gl_module.sql](file:///workspace/backend/database/migration/006_gl_module.sql) - 总账模块
- [007_dual_unit_optimization.sql](file:///workspace/backend/database/migration/007_dual_unit_optimization.sql) - 双计量单位优化
- [008_five_dimension_optimization.sql](file:///workspace/backend/database/migration/008_five_dimension_optimization.sql) - 五维度优化
- [009_business_trace_optimization.sql](file:///workspace/backend/database/migration/009_business_trace_optimization.sql) - 业务追溯优化
- [010_supplier_management.sql](file:///workspace/backend/database/migration/010_supplier_management.sql) - 供应商管理
- [011_purchase_management.sql](file:///workspace/backend/database/migration/011_purchase_management.sql) - 采购管理
- [012_accounts_payable.sql](file:///workspace/backend/database/migration/012_accounts_payable.sql) - 应付账款
- [013_inventory_transfer.sql](file:///workspace/backend/database/migration/013_inventory_transfer.sql) - 库存调拨
- [014_inventory_count.sql](file:///workspace/backend/database/migration/014_inventory_count.sql) - 库存盘点
- [015_inventory_count_items.sql](file:///workspace/backend/database/migration/015_inventory_count_items.sql) - 库存盘点明细
- [020_general_ledger.sql](file:///workspace/backend/database/migration/020_general_ledger.sql) - 总账
- [021_accounts_receivable.sql](file:///workspace/backend/database/migration/021_accounts_receivable.sql) - 应收账款
- [022_cost_management.sql](file:///workspace/backend/database/migration/022_cost_management.sql) - 成本管理
- [030_fixed_assets.sql](file:///workspace/backend/database/migration/030_fixed_assets.sql) - 固定资产
- [031_purchase_contract.sql](file:///workspace/backend/database/migration/031_purchase_contract.sql) - 采购合同
- [032_sales_contract.sql](file:///workspace/backend/database/migration/032_sales_contract.sql) - 销售合同
- [033_customer_credit.sql](file:///workspace/backend/database/migration/033_customer_credit.sql) - 客户信用
- [034_financial_analysis.sql](file:///workspace/backend/database/migration/034_financial_analysis.sql) - 财务分析
- [035_supplier_evaluation.sql](file:///workspace/backend/database/migration/035_supplier_evaluation.sql) - 供应商评估
- [036_fund_management.sql](file:///workspace/backend/database/migration/036_fund_management.sql) - 资金管理
- [037_budget_management.sql](file:///workspace/backend/database/migration/037_budget_management.sql) - 预算管理
- [038_purchase_price.sql](file:///workspace/backend/database/migration/038_purchase_price.sql) - 采购价格
- [039_sales_price.sql](file:///workspace/backend/database/migration/039_sales_price.sql) - 销售价格
- [040_sales_analysis.sql](file:///workspace/backend/database/migration/040_sales_analysis.sql) - 销售分析
- [041_quality_inspection.sql](file:///workspace/backend/database/migration/041_quality_inspection.sql) - 质量检验
- [042_quality_standard.sql](file:///workspace/backend/database/migration/042_quality_standard.sql) - 质量标准
- [050_four_level_batch_management.sql](file:///workspace/backend/database/migration/050_four_level_batch_management.sql) - 四级批次管理
- [051_extend_existing_tables.sql](file:///workspace/backend/database/migration/051_extend_existing_tables.sql) - 扩展现有表
- [052_bpm_process_engine.sql](file:///workspace/backend/database/migration/052_bpm_process_engine.sql) - BPM流程引擎
- [053_bpm_extension.sql](file:///workspace/backend/database/migration/053_bpm_extension.sql) - BPM扩展
- [054_log_management.sql](file:///workspace/backend/database/migration/054_log_management.sql) - 日志管理
- [055_crm_extension.sql](file:///workspace/backend/database/migration/055_crm_extension.sql) - CRM扩展
- [056_oa_collaboration.sql](file:///workspace/backend/database/migration/056_oa_collaboration.sql) - OA协作
- [057_data_visualization.sql](file:///workspace/backend/database/migration/057_data_visualization.sql) - 数据可视化
- [058_test_data.sql](file:///workspace/backend/database/migration/058_test_data.sql) - 测试数据
- [060_assist_accounting.sql](file:///workspace/backend/database/migration/060_assist_accounting.sql) - 辅助核算
- [060_performance_optimization.sql](file:///workspace/backend/database/migration/060_performance_optimization.sql) - 性能优化
- [061_supplier_product_mapping.sql](file:///workspace/backend/database/migration/061_supplier_product_mapping.sql) - 供应商产品映射

---

## API接口文档

### 路由配置

系统的路由配置位于 [backend/src/routes/mod.rs](file:///workspace/backend/src/routes/mod.rs)。

### 主要API接口分类

#### 1. 认证接口
- `POST /api/v1/erp/auth/login` - 用户登录
- `POST /api/v1/erp/auth/logout` - 用户注销
- `POST /api/v1/erp/auth/refresh` - 刷新令牌

#### 2. 用户管理接口
- `GET /api/v1/erp/users` - 获取用户列表
- `POST /api/v1/erp/users` - 创建用户
- `GET /api/v1/erp/users/:id` - 获取用户详情
- `PUT /api/v1/erp/users/:id` - 更新用户
- `DELETE /api/v1/erp/users/:id` - 删除用户

#### 3. 角色管理接口
- `GET /api/v1/erp/roles` - 获取角色列表
- `POST /api/v1/erp/roles` - 创建角色
- `GET /api/v1/erp/roles/:id` - 获取角色详情
- `PUT /api/v1/erp/roles/:id` - 更新角色
- `DELETE /api/v1/erp/roles/:id` - 删除角色

#### 4. 库存管理接口
- `GET /api/v1/erp/inventory/stock` - 获取库存列表
- `POST /api/v1/erp/inventory/stock` - 创建库存记录
- `GET /api/v1/erp/inventory/transfers` - 获取调拨列表
- `POST /api/v1/erp/inventory/transfers` - 创建调拨单
- `GET /api/v1/erp/inventory/counts` - 获取盘点列表
- `POST /api/v1/erp/inventory/counts` - 创建盘点单
- `GET /api/v1/erp/inventory/adjustments` - 获取调整列表
- `POST /api/v1/erp/inventory/adjustments` - 创建调整单

#### 5. 销售管理接口
- `GET /api/v1/erp/sales/orders` - 获取销售订单列表
- `POST /api/v1/erp/sales/orders` - 创建销售订单
- `GET /api/v1/erp/sales/fabric-orders` - 获取面料销售订单列表
- `POST /api/v1/erp/sales/fabric-orders` - 创建面料销售订单
- `GET /api/v1/erp/sales/contracts` - 获取销售合同列表
- `POST /api/v1/erp/sales/contracts` - 创建销售合同

#### 6. 采购管理接口
- `GET /api/v1/erp/purchase/orders` - 获取采购订单列表
- `POST /api/v1/erp/purchase/orders` - 创建采购订单
- `GET /api/v1/erp/purchase/receipts` - 获取采购收货列表
- `POST /api/v1/erp/purchase/receipts` - 创建采购收货单
- `GET /api/v1/erp/purchase/returns` - 获取采购退货列表
- `POST /api/v1/erp/purchase/returns` - 创建采购退货单
- `GET /api/v1/erp/purchase/contracts` - 获取采购合同列表
- `POST /api/v1/erp/purchase/contracts` - 创建采购合同
- `GET /api/v1/erp/purchase/inspections` - 获取采购检验列表
- `POST /api/v1/erp/purchase/inspections` - 创建采购检验单

#### 7. 供应商管理接口
- `GET /api/v1/erp/suppliers` - 获取供应商列表
- `POST /api/v1/erp/suppliers` - 创建供应商
- `GET /api/v1/erp/suppliers/:id` - 获取供应商详情
- `PUT /api/v1/erp/suppliers/:id` - 更新供应商
- `DELETE /api/v1/erp/suppliers/:id` - 删除供应商
- `GET /api/v1/erp/suppliers/evaluations` - 获取供应商评估列表

#### 8. 客户管理接口
- `GET /api/v1/erp/customers` - 获取客户列表
- `POST /api/v1/erp/customers` - 创建客户
- `GET /api/v1/erp/customers/:id` - 获取客户详情
- `PUT /api/v1/erp/customers/:id` - 更新客户
- `DELETE /api/v1/erp/customers/:id` - 删除客户

#### 9. 财务管理接口
- `GET /api/v1/erp/finance/account-subjects` - 获取会计科目列表
- `POST /api/v1/erp/finance/account-subjects` - 创建会计科目
- `GET /api/v1/erp/finance/vouchers` - 获取财务凭证列表
- `POST /api/v1/erp/finance/vouchers` - 创建财务凭证
- `GET /api/v1/erp/finance/invoices` - 获取财务发票列表
- `POST /api/v1/erp/finance/invoices` - 创建财务发票
- `GET /api/v1/erp/finance/payments` - 获取财务付款列表
- `POST /api/v1/erp/finance/payments` - 创建财务付款

#### 10. 应付账款接口
- `GET /api/v1/erp/ap/invoices` - 获取应付发票列表
- `POST /api/v1/erp/ap/invoices` - 创建应付发票
- `GET /api/v1/erp/ap/payments` - 获取应付付款列表
- `POST /api/v1/erp/ap/payments` - 创建应付付款
- `GET /api/v1/erp/ap/payment-requests` - 获取应付付款申请列表
- `POST /api/v1/erp/ap/payment-requests` - 创建应付付款申请
- `GET /api/v1/erp/ap/reconciliations` - 获取应付对账列表
- `POST /api/v1/erp/ap/reconciliations` - 创建应付对账
- `GET /api/v1/erp/ap/verifications` - 获取应付核销列表
- `POST /api/v1/erp/ap/verifications` - 创建应付核销
- `GET /api/v1/erp/ap/reports` - 获取应付报表

#### 11. 应收账款接口
- `GET /api/v1/erp/ar/invoices` - 获取应收发票列表
- `POST /api/v1/erp/ar/invoices` - 创建应收发票

#### 12. 仪表盘接口
- `GET /api/v1/erp/dashboard` - 获取仪表盘数据

#### 13. 健康检查接口
- `GET /health` - 健康检查
- `GET /readiness` - 就绪检查
- `GET /liveness` - 存活检查

---

## 安全机制

### 1. 身份认证

系统使用JWT（JSON Web Token）进行身份认证。

**核心实现**：
- [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) - 认证服务
- [auth.rs](file:///workspace/backend/src/middleware/auth.rs) - 认证中间件
- [auth_context.rs](file:///workspace/backend/src/middleware/auth_context.rs) - 认证上下文

**认证流程**：
1. 用户提交用户名和密码进行登录
2. 系统验证用户凭据
3. 验证成功后生成JWT令牌
4. 前端将令牌存储在localStorage中
5. 后续请求在Authorization头中携带令牌
6. 后端验证令牌的有效性和过期时间

### 2. 权限管理

系统实现了基于角色的访问控制（RBAC）。

**核心实现**：
- [permission.rs](file:///workspace/backend/src/middleware/permission.rs) - 权限中间件
- [role.rs](file:///workspace/backend/src/models/role.rs) - 角色模型
- [role_permission.rs](file:///workspace/backend/src/models/role_permission.rs) - 角色权限模型

**权限管理流程**：
1. 定义系统权限
2. 创建角色并分配权限
3. 为用户分配角色
4. 用户登录时获取角色和权限
5. 访问受保护资源时验证权限

### 3. 请求验证

系统实现了请求验证中间件，确保请求来自前端。

**核心实现**：
- [request_validator.rs](file:///workspace/backend/src/middleware/request_validator.rs) - 请求验证中间件

**验证机制**：
- 检查请求头中的`X-Requested-With`字段
- 验证值是否为`XMLHttpRequest`
- 确保请求是通过前端发起的，防止直接访问后端API

### 4. CORS配置

系统配置了跨域资源共享（CORS），确保前端可以安全访问后端API。

**核心实现**：
- [main.rs](file:///workspace/backend/src/main.rs) - 主入口文件中的CORS配置
- [.env.example](file:///workspace/backend/.env.example) - 环境配置示例

**CORS配置**：
- 配置允许的来源
- 配置允许的HTTP方法
- 配置允许的请求头
- 配置是否允许凭证

### 5. 路由保护

前端实现了路由保护，确保未登录用户只能访问登录页面。

**核心实现**：
- [app/mod.rs](file:///workspace/frontend/src/app/mod.rs) - 应用路由配置

**路由保护机制**：
- 定义公共路由（登录页面）
- 定义受保护路由（需要登录）
- 实现`protected_route`函数保护路由
- 检查用户是否已登录
- 未登录用户重定向到登录页面

### 6. 数据安全

- 密码使用安全的哈希算法存储
- JWT令牌包含过期时间
- 敏感信息不记录在日志中
- 数据库连接使用安全的连接字符串

---

## 部署指南

项目现已支持纯物理机环境的 **一键自动化部署**，无需 Docker 容器。该脚本将自动下载最新代码、配置运行环境、设置 Nginx 反向代理，并注册 Systemd 服务实现开机自启和崩溃保活。

### 1. 一键快速部署

在您的 Linux 服务器（推荐 Ubuntu/Debian/CentOS）上，直接运行以下命令即可完成全自动安装：

```bash
curl -fsSL https://cdn.jsdelivr.net/gh/57231307/1@main/%E5%BF%AB%E9%80%9F%E9%83%A8%E7%BD%B2/install.sh | sudo bash -s install
```

### 2. 一键管理工具 (bingxi)

安装成功后，系统会自动为您在终端注入一个叫做 `bingxi` 的全局命令。后续日常运维非常极简：

```bash
sudo bingxi start    # 启动系统 (后端及Nginx网关)
sudo bingxi stop     # 停止系统
sudo bingxi restart  # 重启系统
sudo bingxi status   # 查看系统运行与保活状态
sudo bingxi update   # 一键平滑升级（自动拉取最新 Release 包并平滑重启）
```

### 3. 环境要求与手动配置

**硬件要求**：
- CPU：至少 2 核
- 内存：至少 4GB
- 磁盘：至少 20GB
- 操作系统：Linux (推荐 Ubuntu 20.04+)

**手动配置 (可选)**：
如果需要修改数据库连接或其它高级配置，请编辑配置文件：
```bash
nano /etc/bingxi/.env
```
修改完成后，重启服务使其生效：
```bash
sudo bingxi restart
```

---

## 开发指南

### 1. 开发环境设置

#### 1.1 安装依赖

**后端依赖**：
```bash
cd backend
cargo build
```

**前端依赖**：
```bash
cd frontend
cargo build
```

#### 1.2 配置开发环境

```bash
cd backend
cp .env.example .env
# 编辑 .env 文件，配置开发环境
```

### 2. 开发工作流

#### 2.1 运行开发服务器

**后端开发服务器**：
```bash
cd backend
cargo run
```

**前端开发服务器**：
```bash
cd frontend
trunk serve
```

#### 2.2 代码规范

**命名约定**：
- 变量和函数：蛇形命名法（snake_case）
- 结构体和枚举：驼峰命名法（CamelCase）
- 常量：全大写蛇形命名法（SNAKE_CASE）

**代码组织**：
- 每个功能模块独立成文件
- 相关代码放在同一目录
- 函数职责单一
- 保持适当的抽象层次

**注释规范**：
- 公共API需要详细文档
- 复杂逻辑需要注释说明
- 注释应该解释为什么，而不是做什么

### 3. 测试指南

#### 3.1 单元测试

```bash
# 运行后端单元测试
cd backend
cargo test

# 运行前端单元测试
cd ../frontend
cargo test
```

#### 3.2 集成测试

```bash
# 运行后端集成测试
cd backend
cargo test --test api_test
cargo test --test auth_integration_test
cargo test --test user_integration_test
```

#### 3.3 性能测试

```bash
# 使用ab进行性能测试
./backend/scripts/performance_test_ab.sh

# 使用wrk进行性能测试
./backend/scripts/performance_test_wrk.sh
```

### 4. 开发流程

#### 4.1 新功能开发

1. 创建分支
2. 实现功能
3. 编写测试
4. 提交代码
5. 进行代码审查
6. 合并到主分支

#### 4.2 Bug修复

1. 复现问题
2. 分析原因
3. 修复代码
4. 编写测试
5. 验证修复
6. 提交代码

---

## 监控与运维

### 1. 监控系统

系统集成了完整的监控体系，包括Prometheus、Grafana和Alertmanager。

#### 1.1 Prometheus配置

**配置文件**：
- [prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml) - Prometheus主配置
- [alert_rules.yml](file:///workspace/monitoring/prometheus/alert_rules.yml) - 告警规则

**监控指标**：
- 系统指标（CPU、内存、磁盘）
- 应用指标（请求数、响应时间、错误率）
- 数据库指标（连接数、查询性能）
- 业务指标（订单量、库存水平）

#### 1.2 Grafana配置

**配置文件**：
- [bingxi-erp-overview.json](file:///workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json) - 仪表盘配置

**仪表盘内容**：
- 系统资源使用情况
- 应用性能指标
- 业务数据统计
- 告警状态

#### 1.3 Alertmanager配置

**配置文件**：
- [alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml) - Alertmanager配置

**告警规则**：
- 系统资源告警（CPU、内存、磁盘）
- 应用性能告警（响应时间、错误率）
- 业务告警（库存预警、订单异常）

### 2. 日志管理

**日志配置**：
- 使用Tracing进行结构化日志记录
- 日志级别：debug、info、warn、error
- 日志文件位置：`backend/logs/`

**日志中间件**：
- [logger_middleware.rs](file:///workspace/backend/src/middleware/logger_middleware.rs) - 日志中间件

**日志模型**：
- [log_api_access.rs](file:///workspace/backend/src/models/log_api_access.rs) - API访问日志
- [log_login.rs](file:///workspace/backend/src/models/log_login.rs) - 登录日志
- [log_system.rs](file:///workspace/backend/src/models/log_system.rs) - 系统日志

### 3. 运维操作

#### 3.1 服务监控

```bash
# 查看后端服务状态
sudo systemctl status bingxi-backend

# 查看后端服务日志
sudo journalctl -u bingxi-backend -f

# 查看Nginx状态
sudo systemctl status nginx

# 查看Nginx日志
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log
```

#### 3.2 数据库维护

```bash
# 备份数据库
pg_dump bingxi > backup_$(date +%Y%m%d).sql

# 恢复数据库
psql bingxi < backup_20260404.sql

# 清理旧日志
find backend/logs/ -name "*.log.*" -mtime +30 -delete
```

#### 3.3 系统更新

使用系统更新模块进行更新：
- [system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs) - 系统更新服务

**更新流程**：
1. 检查更新
2. 下载更新
3. 执行更新
4. 验证更新

---

## 总结

秉羲管理系统是一个功能完整、架构清晰的企业资源规划系统，专为面料行业定制。系统具有以下特点：

- **高性能**：基于Rust语言开发，性能优异
- **可扩展**：模块化设计，易于添加新功能
- **行业特色**：支持面料行业的特殊需求，如批次管理、双计量单位等
- **完整功能**：涵盖企业管理的各个方面，从采购到销售，从库存到财务
- **易于部署**：提供完整的部署脚本和监控方案
- **高安全性**：JWT认证、权限管理、请求验证、CORS配置
- **完整监控**：Prometheus + Grafana + Alertmanager

系统已经具备生产环境运行的条件，可以根据企业的具体需求进行定制和扩展。

---

*本文档由秉羲团队维护，定期更新。*
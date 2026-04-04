# 秉羲管理系统 - 详细项目说明

## 1. 项目简介

秉羲管理系统是一个基于Rust语言开发的企业资源规划（ERP）系统，专为面料行业定制，提供完整的企业管理功能。系统采用现代化的前后端分离架构，具有高性能、可扩展、行业特色明显等特点。

### 1.1 项目定位

- **目标行业**：面料生产与贸易企业
- **系统规模**：中大型企业级应用
- **核心价值**：提供完整的面料行业管理解决方案，优化业务流程，提高管理效率

### 1.2 系统特点

- **高性能**：基于Rust语言开发，性能优异，资源占用低
- **行业特色**：支持面料行业特殊需求，如批次管理、双计量单位自动换算等
- **模块化设计**：清晰的模块划分，易于扩展和维护
- **前后端分离**：现代化的前后端分离架构，提供良好的用户体验
- **完整功能**：涵盖企业管理的各个方面，从采购到销售，从库存到财务
- **多租户支持**：可支持多个企业或部门使用同一系统
- **安全可靠**：完善的认证授权机制，数据安全保障

## 2. 技术架构

### 2.1 技术栈

| 类别 | 技术 | 版本 | 用途 | 来源 |
|------|------|------|------|------|
| **后端** | Rust | 2021 | 主要开发语言 | [backend/Cargo.toml](file:///workspace/backend/Cargo.toml) |
| | Axum | 0.7 | Web框架 | [backend/Cargo.toml](file:///workspace/backend/Cargo.toml) |
| | PostgreSQL | 14+ | 数据库 | [backend/.env.example](file:///workspace/backend/.env.example) |
| | SeaORM | 0.12 | ORM框架 | [backend/Cargo.toml](file:///workspace/backend/Cargo.toml) |
| | JWT | - | 认证机制 | [backend/src/services/auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) |
| | gRPC | - | 服务间通信 | [backend/proto/bingxi.proto](file:///workspace/backend/proto/bingxi.proto) |
| | OpenAPI | - | API文档 | [backend/src/openapi.rs](file:///workspace/backend/src/openapi.rs) |
| **前端** | Yew | 0.21 | Web框架 | [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml) |
| | Rust WebAssembly | - | 前端运行环境 | [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml) |
| | Yew Router | - | 前端路由 | [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml) |
| | Gloo Net | - | HTTP客户端 | [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml) |
| **部署** | Systemd | - | 服务管理 | [deploy/bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service) |
| | Nginx | 1.18+ | 反向代理 | [deploy/nginx.conf](file:///workspace/deploy/nginx.conf) |
| **监控** | Prometheus | - | 指标收集 | [monitoring/prometheus/prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml) |
| | Grafana | - | 指标可视化 | [monitoring/grafana/dashboards/bingxi-erp-overview.json](file:///workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json) |
| | Alertmanager | - | 告警管理 | [monitoring/alertmanager/alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml) |

### 2.2 架构设计

#### 2.2.1 整体架构

```mermaid
flowchart TD
    subgraph ClientLayer[客户端层]
        Browser[浏览器]
        MobileApp[移动应用]
    end

    subgraph FrontendLayer[前端层]
        YewApp[Yew应用]
        WebAssembly[WebAssembly运行时]
        YewRouter[Yew Router]
        YewContext[Yew Context API]
    end

    subgraph BackendLayer[后端层]
        AxumServer[Axum服务器]
        Middleware[中间件]
        Handlers[API处理函数]
        Services[业务逻辑服务]
        Models[数据模型]
        Database[PostgreSQL数据库]
        gRPC[gRPC服务]
    end

    subgraph InfrastructureLayer[基础设施层]
        Systemd[Systemd服务管理]
        Nginx[Nginx反向代理]
        Prometheus[Prometheus监控]
        Grafana[Grafana可视化]
        Alertmanager[Alertmanager告警]
    end

    Browser -->|HTTP请求| Nginx
    MobileApp -->|HTTP请求| Nginx
    Nginx -->|反向代理| AxumServer
    AxumServer -->|路由| Middleware
    Middleware -->|认证授权| Handlers
    Handlers -->|调用| Services
    Services -->|操作| Models
    Models -->|CRUD| Database
    Services -->|远程调用| gRPC
    YewApp -->|API调用| Nginx
    YewApp -->|使用| YewRouter
    YewApp -->|状态管理| YewContext
    YewApp -->|运行于| WebAssembly
    AxumServer -->|暴露指标| Prometheus
    Prometheus -->|数据| Grafana
    Prometheus -->|告警| Alertmanager
    Systemd -->|管理| AxumServer
```

#### 2.2.2 核心模块关系

| 模块 | 主要职责 | 依赖关系 | 调用链 |
|------|---------|----------|--------|
| **认证模块** | 用户登录、注销、令牌管理 | 依赖用户服务 | `auth_handler.rs → auth_service.rs → user_service.rs` |
| **用户模块** | 用户CRUD、角色管理 | 依赖数据库 | `user_handler.rs → user_service.rs → user_model.rs` |
| **库存模块** | 库存管理、调拨、盘点 | 依赖产品模块 | `inventory_stock_handler.rs → inventory_stock_service.rs → product_service.rs` |
| **销售模块** | 销售订单、合同管理 | 依赖客户模块 | `sales_order_handler.rs → sales_service.rs → customer_service.rs` |
| **采购模块** | 采购订单、收货管理 | 依赖供应商模块 | `purchase_order_handler.rs → purchase_order_service.rs → supplier_service.rs` |
| **财务模块** | 凭证、发票管理 | 依赖销售/采购模块 | `voucher_handler.rs → voucher_service.rs → sales_service.rs` |
| **批次模块** | 面料批次、色号管理 | 依赖产品模块 | `batch_handler.rs → batch_service.rs → product_service.rs` |

## 3. 项目结构

### 3.1 目录结构

```
├── backend/             # 后端应用
│   ├── src/             # 源代码
│   │   ├── config/      # 配置管理
│   │   ├── database/    # 数据库连接
│   │   ├── grpc/        # gRPC服务
│   │   ├── handlers/    # API处理函数
│   │   ├── middleware/  # 中间件
│   │   ├── models/      # 数据模型
│   │   ├── routes/      # 路由配置
│   │   ├── services/    # 业务逻辑
│   │   ├── utils/       # 工具函数
│   │   ├── lib.rs       # 库入口
│   │   └── main.rs      # 应用入口
│   ├── database/        # 数据库迁移
│   ├── proto/           # gRPC协议定义
│   ├── Cargo.toml       # 依赖管理
│   └── config.toml      # 配置文件
├── frontend/            # 前端应用
│   ├── src/             # 源代码
│   │   ├── app/         # 应用组件
│   │   ├── components/  # 通用组件
│   │   ├── models/      # 数据模型
│   │   ├── pages/       # 页面组件
│   │   ├── services/    # API服务
│   │   ├── utils/       # 工具函数
│   │   └── main.rs      # 应用入口
│   ├── static/          # 静态资源
│   ├── styles/          # 样式文件
│   ├── Cargo.toml       # 依赖管理
│   └── Trunk.toml       # Trunk配置
├── deploy/              # 部署脚本
├── monitoring/          # 监控配置
└── releases/            # 发布包
```

### 3.2 核心目录说明

| 目录 | 主要职责 | 关键文件 |
|------|---------|----------|
| **backend/src/handlers/** | API处理函数，处理HTTP请求 | [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs), [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) |
| **backend/src/services/** | 业务逻辑层，实现核心功能 | [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs), [inventory_stock_service.rs](file:///workspace/backend/src/services/inventory_stock_service.rs) |
| **backend/src/models/** | 数据模型层，定义数据库表结构 | [user.rs](file:///workspace/backend/src/models/user.rs), [inventory_stock.rs](file:///workspace/backend/src/models/inventory_stock.rs) |
| **backend/src/routes/** | 路由配置，定义API路径 | [mod.rs](file:///workspace/backend/src/routes/mod.rs) |
| **backend/src/middleware/** | 中间件，处理认证、日志等横切关注点 | [auth.rs](file:///workspace/backend/src/middleware/auth.rs), [logger_middleware.rs](file:///workspace/backend/src/middleware/logger_middleware.rs) |
| **frontend/src/pages/** | 前端页面组件 | [login.rs](file:///workspace/frontend/src/pages/login.rs), [dashboard.rs](file:///workspace/frontend/src/pages/dashboard.rs) |
| **frontend/src/services/** | 前端API服务，与后端通信 | [api.rs](file:///workspace/frontend/src/services/api.rs), [auth.rs](file:///workspace/frontend/src/services/auth.rs) |
| **frontend/src/components/** | 前端通用组件 | [navigation.rs](file:///workspace/frontend/src/components/navigation.rs), [main_layout.rs](file:///workspace/frontend/src/components/main_layout.rs) |
| **deploy/** | 部署脚本和配置 | [deploy.sh](file:///workspace/deploy/deploy.sh), [nginx.conf](file:///workspace/deploy/nginx.conf) |
| **monitoring/** | 监控配置 | [prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml), [alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml) |

## 4. 核心功能模块

### 4.1 认证与用户管理

#### 4.1.1 认证服务

**功能说明**：处理用户登录、注销和令牌管理，确保系统安全访问。

**核心功能**：
- JWT令牌生成与验证
- 密码哈希与验证
- 用户认证逻辑
- 令牌刷新机制

**实现文件**：
- [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) - 处理认证相关API请求
- [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) - 实现认证业务逻辑
- [auth.rs](file:///workspace/frontend/src/services/auth.rs) - 前端认证服务

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/auth/login` | POST | 用户登录 | 否 |
| `/api/v1/erp/auth/logout` | POST | 用户注销 | 是 |
| `/api/v1/erp/auth/refresh` | POST | 刷新令牌 | 是 |

#### 4.1.2 用户管理

**功能说明**：管理系统用户，包括创建、查询、更新和删除用户，以及角色分配。

**核心功能**：
- 用户CRUD操作
- 角色分配与管理
- 部门关联
- 权限管理

**实现文件**：
- [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) - 处理用户相关API请求
- [user_service.rs](file:///workspace/backend/src/services/user_service.rs) - 实现用户管理业务逻辑
- [user_list.rs](file:///workspace/frontend/src/pages/user_list.rs) - 前端用户管理页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/users` | GET | 获取用户列表 | 是 |
| `/api/v1/erp/users` | POST | 创建用户 | 是 |
| `/api/v1/erp/users/:id` | GET | 获取用户详情 | 是 |
| `/api/v1/erp/users/:id` | PUT | 更新用户 | 是 |
| `/api/v1/erp/users/:id` | DELETE | 删除用户 | 是 |

### 4.2 库存管理

**功能说明**：管理企业库存，包括库存查询、调拨、盘点和调整，支持面料行业特色功能。

**核心功能**：
- 库存查询与统计
- 库存调拨
- 库存盘点
- 库存调整
- 批次管理
- 色号管理
- 双计量单位（米/公斤）自动换算
- 缸号管理
- 坯布管理

**实现文件**：
- [inventory_stock_handler.rs](file:///workspace/backend/src/handlers/inventory_stock_handler.rs) - 处理库存相关API请求
- [inventory_stock_service.rs](file:///workspace/backend/src/services/inventory_stock_service.rs) - 实现库存管理业务逻辑
- [inventory_stock.rs](file:///workspace/frontend/src/pages/inventory_stock.rs) - 前端库存管理页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/inventory/stock` | GET | 获取库存列表 | 是 |
| `/api/v1/erp/inventory/stock` | POST | 创建库存记录 | 是 |
| `/api/v1/erp/inventory/transfers` | GET | 获取调拨列表 | 是 |
| `/api/v1/erp/inventory/transfers` | POST | 创建调拨单 | 是 |
| `/api/v1/erp/inventory/counts` | GET | 获取盘点列表 | 是 |
| `/api/v1/erp/inventory/counts` | POST | 创建盘点单 | 是 |
| `/api/v1/erp/inventory/adjustments` | GET | 获取调整列表 | 是 |
| `/api/v1/erp/inventory/adjustments` | POST | 创建调整单 | 是 |

### 4.3 销售管理

**功能说明**：管理企业销售业务，包括销售订单创建与管理、销售合同管理、销售价格管理和销售分析。

**核心功能**：
- 销售订单创建与管理
- 销售合同管理
- 销售价格管理
- 销售分析
- 面料销售订单（支持米/公斤双计量单位）
- 色号管理
- 批次追踪

**实现文件**：
- [sales_order_handler.rs](file:///workspace/backend/src/handlers/sales_order_handler.rs) - 处理销售订单API请求
- [sales_service.rs](file:///workspace/backend/src/services/sales_service.rs) - 实现销售管理业务逻辑
- [sales_order.rs](file:///workspace/frontend/src/pages/sales_order.rs) - 前端销售订单页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/sales/orders` | GET | 获取销售订单列表 | 是 |
| `/api/v1/erp/sales/orders` | POST | 创建销售订单 | 是 |
| `/api/v1/erp/sales/orders/:id` | GET | 获取销售订单详情 | 是 |
| `/api/v1/erp/sales/orders/:id` | PUT | 更新销售订单 | 是 |
| `/api/v1/erp/sales/orders/:id` | DELETE | 删除销售订单 | 是 |
| `/api/v1/erp/sales/fabric-orders` | GET | 获取面料销售订单列表 | 是 |
| `/api/v1/erp/sales/fabric-orders` | POST | 创建面料销售订单 | 是 |

### 4.4 采购管理

**功能说明**：管理企业采购业务，包括采购订单创建与管理、采购合同管理、采购收货、采购退货和采购价格管理。

**核心功能**：
- 采购订单创建与管理
- 采购合同管理
- 采购收货
- 采购退货
- 采购价格管理
- 供应商管理

**实现文件**：
- [purchase_order_handler.rs](file:///workspace/backend/src/handlers/purchase_order_handler.rs) - 处理采购订单API请求
- [purchase_order_service.rs](file:///workspace/backend/src/services/purchase_order_service.rs) - 实现采购管理业务逻辑
- [purchase_order.rs](file:///workspace/frontend/src/pages/purchase_order.rs) - 前端采购订单页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/purchase/orders` | GET | 获取采购订单列表 | 是 |
| `/api/v1/erp/purchase/orders` | POST | 创建采购订单 | 是 |
| `/api/v1/erp/purchase/orders/:id` | GET | 获取采购订单详情 | 是 |
| `/api/v1/erp/purchase/orders/:id` | PUT | 更新采购订单 | 是 |
| `/api/v1/erp/purchase/orders/:id` | DELETE | 删除采购订单 | 是 |
| `/api/v1/erp/purchase/receipts` | GET | 获取采购收货列表 | 是 |
| `/api/v1/erp/purchase/receipts` | POST | 创建采购收货 | 是 |
| `/api/v1/erp/purchase/returns` | GET | 获取采购退货列表 | 是 |
| `/api/v1/erp/purchase/returns` | POST | 创建采购退货 | 是 |

### 4.5 财务管理

**功能说明**：管理企业财务业务，包括总账管理、科目管理、凭证管理、应付账款、应收账款和财务分析。

**核心功能**：
- 总账管理
- 科目管理
- 凭证管理
- 应付账款
- 应收账款
- 财务分析
- 发票管理

**实现文件**：
- [voucher_handler.rs](file:///workspace/backend/src/handlers/voucher_handler.rs) - 处理凭证相关API请求
- [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) - 实现财务管理业务逻辑
- [voucher.rs](file:///workspace/frontend/src/pages/voucher.rs) - 前端凭证管理页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/finance/vouchers` | GET | 获取凭证列表 | 是 |
| `/api/v1/erp/finance/vouchers` | POST | 创建凭证 | 是 |
| `/api/v1/erp/finance/vouchers/:id` | GET | 获取凭证详情 | 是 |
| `/api/v1/erp/finance/vouchers/:id` | PUT | 更新凭证 | 是 |
| `/api/v1/erp/finance/vouchers/:id` | DELETE | 删除凭证 | 是 |
| `/api/v1/erp/finance/invoices` | GET | 获取发票列表 | 是 |
| `/api/v1/erp/finance/invoices` | POST | 创建发票 | 是 |

### 4.6 业务追溯

**功能说明**：提供业务流程追溯和数据快照功能，支持五维度管理（产品、批次、色号、等级、仓库）。

**核心功能**：
- 五维度管理（产品、批次、色号、等级、仓库）
- 业务流程追溯
- 数据快照
- 批次追踪

**实现文件**：
- [business_trace_handler.rs](file:///workspace/backend/src/handlers/business_trace_handler.rs) - 处理业务追溯API请求
- [business_trace_service.rs](file:///workspace/backend/src/services/business_trace_service.rs) - 实现业务追溯业务逻辑
- [business_trace.rs](file:///workspace/frontend/src/pages/business_trace.rs) - 前端业务追溯页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/business-trace` | GET | 获取业务追溯列表 | 是 |
| `/api/v1/erp/business-trace/:id` | GET | 获取业务追溯详情 | 是 |
| `/api/v1/erp/business-trace/snapshots` | GET | 获取数据快照列表 | 是 |

### 4.7 面料行业特色功能

**功能说明**：专为面料行业定制的特色功能，满足行业特殊需求。

**核心功能**：
- 批次管理（染色批次、缸号管理）
- 双计量单位（米/公斤）自动换算
- 色号管理
- 坯布管理
- 染色配方管理

**实现文件**：
- [batch_handler.rs](file:///workspace/backend/src/handlers/batch_handler.rs) - 处理批次相关API请求
- [batch_service.rs](file:///workspace/backend/src/services/batch_service.rs) - 实现批次管理业务逻辑
- [dye_batch.rs](file:///workspace/frontend/src/pages/dye_batch.rs) - 前端染色批次页面
- [greige_fabric.rs](file:///workspace/frontend/src/pages/greige_fabric.rs) - 前端坯布管理页面

**API接口**：
| 路径 | 方法 | 功能 | 认证要求 |
|------|------|------|----------|
| `/api/v1/erp/batch/dye-batches` | GET | 获取染色批次列表 | 是 |
| `/api/v1/erp/batch/dye-batches` | POST | 创建染色批次 | 是 |
| `/api/v1/erp/batch/greige-fabrics` | GET | 获取坯布列表 | 是 |
| `/api/v1/erp/batch/greige-fabrics` | POST | 创建坯布记录 | 是 |
| `/api/v1/erp/batch/dye-recipes` | GET | 获取染色配方列表 | 是 |
| `/api/v1/erp/batch/dye-recipes` | POST | 创建染色配方 | 是 |

## 5. 数据库设计

### 5.1 核心表结构

| 表名 | 主要功能 | 关键字段 | 关联表 |
|------|---------|----------|--------|
| **users** | 用户信息 | id, username, password_hash, role_id | roles |
| **roles** | 角色信息 | id, name, permissions | - |
| **departments** | 部门信息 | id, name, parent_id | departments (自关联) |
| **products** | 产品信息 | id, code, name, category_id | product_categories |
| **product_categories** | 产品类别 | id, name, parent_id | product_categories (自关联) |
| **warehouses** | 仓库信息 | id, code, name, status | - |
| **inventory_stocks** | 库存信息 | id, product_id, warehouse_id, batch_no, color_code, quantity_meters, quantity_kg | products, warehouses |
| **inventory_transfers** | 库存调拨 | id, transfer_no, from_warehouse_id, to_warehouse_id | warehouses |
| **inventory_counts** | 库存盘点 | id, count_no, warehouse_id, status | warehouses |
| **inventory_adjustments** | 库存调整 | id, adjustment_no, warehouse_id, reason | warehouses |
| **sales_orders** | 销售订单 | id, order_no, customer_id, status | customers |
| **sales_order_items** | 销售订单明细 | id, order_id, product_id, quantity, price | sales_orders, products |
| **purchase_orders** | 采购订单 | id, order_no, supplier_id, status | suppliers |
| **purchase_order_items** | 采购订单明细 | id, order_id, product_id, quantity, price | purchase_orders, products |
| **suppliers** | 供应商信息 | id, name, contact_person, contact_phone | - |
| **customers** | 客户信息 | id, name, contact_person, contact_phone | - |
| **finance_invoices** | 发票信息 | id, invoice_no, amount, status, type | - |
| **vouchers** | 财务凭证 | id, voucher_no, amount, status, date | - |
| **voucher_items** | 凭证明细 | id, voucher_id, account_subject_id, amount, direction | vouchers, account_subjects |
| **account_subjects** | 会计科目 | id, code, name, type | - |

### 5.2 面料行业特色表

| 表名 | 主要功能 | 关键字段 | 关联表 |
|------|---------|----------|--------|
| **dye_batches** | 染色批次 | id, batch_no, color_code, status, greige_fabric_id | greige_fabrics |
| **greige_fabrics** | 坯布管理 | id, batch_no, supplier_id, quantity, status | suppliers |
| **dye_recipes** | 染色配方 | id, recipe_no, color_code, formula | - |
| **dye_lot_mappings** | 染色缸号映射 | id, dye_batch_id, lot_no, quantity | dye_batches |
| **product_color_mappings** | 产品色号映射 | id, product_id, color_code, name | products |
| **batch_trace_logs** | 批次追踪日志 | id, batch_no, operation, timestamp, user_id | users |

### 5.3 索引设计

| 表名 | 索引字段 | 索引类型 | 目的 |
|------|----------|----------|------|
| **users** | username | 唯一索引 | 加速用户登录查询 |
| **inventory_stocks** | product_id, warehouse_id, batch_no, color_code | 复合索引 | 加速库存查询 |
| **sales_orders** | order_no | 唯一索引 | 加速订单查询 |
| **purchase_orders** | order_no | 唯一索引 | 加速订单查询 |
| **dye_batches** | batch_no | 唯一索引 | 加速批次查询 |
| **greige_fabrics** | batch_no | 唯一索引 | 加速坯布查询 |

## 6. API接口详细说明

### 6.1 认证接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/auth/login` | POST | 用户登录 | `{"username": "admin", "password": "password"}` | `{"token": "...", "user": {...}}` |
| `/api/v1/erp/auth/logout` | POST | 用户注销 | N/A | `{"message": "Logged out successfully"}` |
| `/api/v1/erp/auth/refresh` | POST | 刷新令牌 | `{"refresh_token": "..."}` | `{"token": "..."}` |

### 6.2 用户管理接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/users` | GET | 获取用户列表 | N/A | `[{"id": 1, "username": "admin", ...}]` |
| `/api/v1/erp/users` | POST | 创建用户 | `{"username": "user1", "password": "password", "role_id": 1}` | `{"id": 2, "username": "user1", ...}` |
| `/api/v1/erp/users/:id` | GET | 获取用户详情 | N/A | `{"id": 1, "username": "admin", ...}` |
| `/api/v1/erp/users/:id` | PUT | 更新用户 | `{"username": "admin", "role_id": 1}` | `{"id": 1, "username": "admin", ...}` |
| `/api/v1/erp/users/:id` | DELETE | 删除用户 | N/A | `{"message": "User deleted successfully"}` |

### 6.3 库存管理接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/inventory/stock` | GET | 获取库存列表 | N/A | `[{"id": 1, "product_id": 1, "quantity_meters": 100, ...}]` |
| `/api/v1/erp/inventory/stock` | POST | 创建库存记录 | `{"product_id": 1, "warehouse_id": 1, "quantity_meters": 100, ...}` | `{"id": 1, "product_id": 1, ...}` |
| `/api/v1/erp/inventory/transfers` | GET | 获取调拨列表 | N/A | `[{"id": 1, "transfer_no": "TR-2026-001", ...}]` |
| `/api/v1/erp/inventory/transfers` | POST | 创建调拨单 | `{"from_warehouse_id": 1, "to_warehouse_id": 2, "items": [...]}` | `{"id": 1, "transfer_no": "TR-2026-001", ...}` |
| `/api/v1/erp/inventory/counts` | GET | 获取盘点列表 | N/A | `[{"id": 1, "count_no": "CN-2026-001", ...}]` |
| `/api/v1/erp/inventory/counts` | POST | 创建盘点单 | `{"warehouse_id": 1, "items": [...]}` | `{"id": 1, "count_no": "CN-2026-001", ...}` |

### 6.4 销售管理接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/sales/orders` | GET | 获取销售订单列表 | N/A | `[{"id": 1, "order_no": "SO-2026-001", ...}]` |
| `/api/v1/erp/sales/orders` | POST | 创建销售订单 | `{"customer_id": 1, "items": [...], ...}` | `{"id": 1, "order_no": "SO-2026-001", ...}` |
| `/api/v1/erp/sales/orders/:id` | GET | 获取销售订单详情 | N/A | `{"id": 1, "order_no": "SO-2026-001", "items": [...], ...}` |
| `/api/v1/erp/sales/orders/:id` | PUT | 更新销售订单 | `{"status": "completed", ...}` | `{"id": 1, "order_no": "SO-2026-001", ...}` |
| `/api/v1/erp/sales/orders/:id` | DELETE | 删除销售订单 | N/A | `{"message": "Order deleted successfully"}` |

### 6.5 采购管理接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/purchase/orders` | GET | 获取采购订单列表 | N/A | `[{"id": 1, "order_no": "PO-2026-001", ...}]` |
| `/api/v1/erp/purchase/orders` | POST | 创建采购订单 | `{"supplier_id": 1, "items": [...], ...}` | `{"id": 1, "order_no": "PO-2026-001", ...}` |
| `/api/v1/erp/purchase/orders/:id` | GET | 获取采购订单详情 | N/A | `{"id": 1, "order_no": "PO-2026-001", "items": [...], ...}` |
| `/api/v1/erp/purchase/orders/:id` | PUT | 更新采购订单 | `{"status": "completed", ...}` | `{"id": 1, "order_no": "PO-2026-001", ...}` |
| `/api/v1/erp/purchase/orders/:id` | DELETE | 删除采购订单 | N/A | `{"message": "Order deleted successfully"}` |

### 6.6 财务管理接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/finance/vouchers` | GET | 获取凭证列表 | N/A | `[{"id": 1, "voucher_no": "V-2026-001", ...}]` |
| `/api/v1/erp/finance/vouchers` | POST | 创建凭证 | `{"date": "2026-04-04", "items": [...], ...}` | `{"id": 1, "voucher_no": "V-2026-001", ...}` |
| `/api/v1/erp/finance/vouchers/:id` | GET | 获取凭证详情 | N/A | `{"id": 1, "voucher_no": "V-2026-001", "items": [...], ...}` |
| `/api/v1/erp/finance/vouchers/:id` | PUT | 更新凭证 | `{"status": "approved", ...}` | `{"id": 1, "voucher_no": "V-2026-001", ...}` |
| `/api/v1/erp/finance/vouchers/:id` | DELETE | 删除凭证 | N/A | `{"message": "Voucher deleted successfully"}` |

### 6.7 批次管理接口

| 路径 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|--------|
| `/api/v1/erp/batch/dye-batches` | GET | 获取染色批次列表 | N/A | `[{"id": 1, "batch_no": "DB-2026-001", ...}]` |
| `/api/v1/erp/batch/dye-batches` | POST | 创建染色批次 | `{"batch_no": "DB-2026-001", "color_code": "RED-001", ...}` | `{"id": 1, "batch_no": "DB-2026-001", ...}` |
| `/api/v1/erp/batch/greige-fabrics` | GET | 获取坯布列表 | N/A | `[{"id": 1, "batch_no": "GF-2026-001", ...}]` |
| `/api/v1/erp/batch/greige-fabrics` | POST | 创建坯布记录 | `{"batch_no": "GF-2026-001", "supplier_id": 1, ...}` | `{"id": 1, "batch_no": "GF-2026-001", ...}` |

## 7. 前端应用

### 7.1 前端架构

**技术栈**：
- Yew 0.21 (Rust WebAssembly)
- Yew Router
- Yew Context API
- Gloo Net (HTTP客户端)
- CSS3

**架构特点**：
- 组件化设计
- 响应式布局
- 状态管理
- 路由导航
- API服务层

### 7.2 核心页面

| 页面 | 功能 | 实现文件 | 路由 |
|------|------|----------|------|
| **登录页面** | 用户登录 | [login.rs](file:///workspace/frontend/src/pages/login.rs) | `/login` |
| **仪表盘** | 系统概览 | [dashboard.rs](file:///workspace/frontend/src/pages/dashboard.rs) | `/dashboard` |
| **用户管理** | 用户CRUD | [user_list.rs](file:///workspace/frontend/src/pages/user_list.rs) | `/users` |
| **库存管理** | 库存查询与管理 | [inventory_stock.rs](file:///workspace/frontend/src/pages/inventory_stock.rs) | `/inventory/stock` |
| **销售订单** | 销售订单管理 | [sales_order.rs](file:///workspace/frontend/src/pages/sales_order.rs) | `/sales/orders` |
| **采购订单** | 采购订单管理 | [purchase_order.rs](file:///workspace/frontend/src/pages/purchase_order.rs) | `/purchase/orders` |
| **财务管理** | 财务凭证管理 | [voucher.rs](file:///workspace/frontend/src/pages/voucher.rs) | `/finance/vouchers` |
| **批次管理** | 染色批次管理 | [dye_batch.rs](file:///workspace/frontend/src/pages/dye_batch.rs) | `/batch/dye-batches` |
| **坯布管理** | 坯布记录管理 | [greige_fabric.rs](file:///workspace/frontend/src/pages/greige_fabric.rs) | `/batch/greige-fabrics` |
| **业务追溯** | 业务流程追溯 | [business_trace.rs](file:///workspace/frontend/src/pages/business_trace.rs) | `/business-trace` |

### 7.3 前端组件

| 组件 | 功能 | 实现文件 |
|------|------|----------|
| **导航组件** | 系统导航菜单 | [navigation.rs](file:///workspace/frontend/src/components/navigation.rs) |
| **主布局** | 页面主布局 | [main_layout.rs](file:///workspace/frontend/src/components/main_layout.rs) |
| **表单组件** | 通用表单元素 | 待实现 |
| **表格组件** | 数据表格 | 待实现 |
| **图表组件** | 数据可视化 | 待实现 |

### 7.4 前端服务

| 服务 | 功能 | 实现文件 |
|------|------|----------|
| **API服务** | HTTP请求封装 | [api.rs](file:///workspace/frontend/src/services/api.rs) |
| **认证服务** | 用户认证 | [auth.rs](file:///workspace/frontend/src/services/auth.rs) |
| **用户服务** | 用户相关API | [user_service.rs](file:///workspace/frontend/src/services/user_service.rs) |
| **库存服务** | 库存相关API | [inventory_service.rs](file:///workspace/frontend/src/services/inventory_service.rs) |
| **销售服务** | 销售相关API | [sales_service.rs](file:///workspace/frontend/src/services/sales_service.rs) |
| **采购服务** | 采购相关API | [purchase_order_service.rs](file:///workspace/frontend/src/services/purchase_order_service.rs) |
| **财务服务** | 财务相关API | [voucher_service.rs](file:///workspace/frontend/src/services/voucher_service.rs) |
| **批次服务** | 批次相关API | [batch_service.rs](file:///workspace/frontend/src/services/batch_service.rs) |

## 8. 部署与运维

### 8.1 部署方案

**部署架构**：
- **后端**：Rust编译的可执行文件，通过Systemd管理
- **前端**：WebAssembly静态文件，通过Nginx托管
- **数据库**：PostgreSQL
- **反向代理**：Nginx

**部署流程**：
1. 编译后端应用：`cargo build --release`
2. 编译前端应用：`trunk build --release`
3. 配置环境变量
4. 配置Systemd服务
5. 配置Nginx
6. 启动服务

**部署脚本**：
- [deploy.sh](file:///workspace/deploy/deploy.sh) - 主部署脚本
- [deploy-backend.sh](file:///workspace/deploy/deploy-backend.sh) - 后端部署脚本
- [deploy-frontend.sh](file:///workspace/deploy/deploy-frontend.sh) - 前端部署脚本

### 8.2 配置管理

**环境变量**：
| 变量名 | 描述 | 默认值 | 来源 |
|--------|------|--------|------|
| `DATABASE_URL` | 数据库连接字符串 | - | [.env.example](file:///workspace/backend/.env.example) |
| `JWT_SECRET` | JWT密钥 | - | [.env.example](file:///workspace/backend/.env.example) |
| `SERVER_HOST` | 服务器主机 | 0.0.0.0 | [.env.example](file:///workspace/backend/.env.example) |
| `SERVER_PORT` | 服务器端口 | 8080 | [.env.example](file:///workspace/backend/.env.example) |
| `LOG_LEVEL` | 日志级别 | info | [.env.example](file:///workspace/backend/.env.example) |
| `CORS_ALLOWED_ORIGINS` | CORS允许的来源 | * | [.env.example](file:///workspace/backend/.env.example) |

**配置文件**：
- [config.toml](file:///workspace/backend/config.toml) - 后端配置
- [nginx.conf](file:///workspace/deploy/nginx.conf) - Nginx配置
- [bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service) - Systemd服务配置

### 8.3 监控与告警

**监控组件**：
- **Prometheus**：指标收集
- **Grafana**：指标可视化
- **Alertmanager**：告警管理

**监控指标**：
- 系统指标（CPU、内存、磁盘）
- 应用指标（请求数、响应时间）
- 数据库指标（连接数、查询性能）
- 业务指标（订单量、库存水平）

**告警规则**：
- 服务不可用
- 响应时间过长
- 错误率过高
- 系统资源不足

**监控配置**：
- [prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml) - Prometheus配置
- [alert_rules.yml](file:///workspace/monitoring/prometheus/alert_rules.yml) - 告警规则
- [alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml) - Alertmanager配置
- [bingxi-erp-overview.json](file:///workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json) - Grafana仪表盘

### 8.4 日志管理

**日志系统**：
- 使用Rust的Tracing库
- 支持不同日志级别
- 日志文件轮转

**日志文件**：
- 位置：`backend/logs/`
- 命名格式：`bingxi_backend.log.YYYY-MM-DD`

**日志级别**：
- debug
- info
- warn
- error

## 9. 开发与测试

### 9.1 开发环境

**环境要求**：
- Rust 1.70+
- PostgreSQL 14+
- Trunk (前端构建工具)
- Node.js (可选，用于npm依赖)

**开发流程**：
1. 克隆代码库
2. 配置环境变量
3. 启动数据库
4. 运行后端开发服务器
5. 运行前端开发服务器

**开发命令**：
```bash
# 后端开发
cd backend
cargo run

# 前端开发
cd frontend
trunk serve
```

### 9.2 测试策略

**测试类型**：
- 单元测试
- 集成测试
- 性能测试

**测试命令**：
```bash
# 运行后端单元测试
cd backend
cargo test

# 运行后端集成测试
cd backend
cargo test --test api_test

# 运行前端单元测试
cd frontend
cargo test

# 性能测试
./backend/scripts/performance_test_ab.sh
./backend/scripts/performance_test_wrk.sh
```

### 9.3 代码规范

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

## 10. 系统要求

### 10.1 硬件要求

| 环境 | CPU | 内存 | 磁盘 |
|------|-----|------|------|
| **开发环境** | 至少2核 | 至少4GB | 至少20GB |
| **测试环境** | 至少4核 | 至少8GB | 至少50GB |
| **生产环境** | 至少8核 | 至少16GB | 至少100GB |

### 10.2 软件要求

| 软件 | 版本 | 用途 |
|------|------|------|
| Rust | 1.70+ | 后端开发 |
| PostgreSQL | 14+ | 数据库 |
| Nginx | 1.18+ | 反向代理 |
| Systemd | - | 服务管理 |
| Trunk | 0.18+ | 前端构建 |
| Prometheus | 2.40+ | 监控 |
| Grafana | 9.0+ | 监控可视化 |
| Alertmanager | 0.25+ | 告警管理 |

## 11. 常见问题与解决方案

### 11.1 数据库连接失败

**问题**：后端无法连接到数据库

**解决方案**：
- 检查数据库服务是否运行
- 检查连接字符串是否正确
- 检查数据库用户权限
- 检查网络连接

### 11.2 服务启动失败

**问题**：后端服务启动失败

**解决方案**：
- 检查端口是否被占用
- 检查配置文件是否正确
- 查看日志文件获取详细错误信息
- 检查环境变量是否设置

### 11.3 前端访问404

**问题**：前端页面访问返回404

**解决方案**：
- 检查Nginx配置是否正确
- 检查前端文件是否正确部署
- 检查路由配置是否正确
- 检查前端构建是否成功

### 11.4 性能问题

**问题**：系统响应缓慢

**解决方案**：
- 检查数据库索引
- 优化SQL查询
- 考虑使用缓存
- 检查系统资源使用情况
- 分析性能瓶颈

### 11.5 认证失败

**问题**：用户登录失败

**解决方案**：
- 检查用户名和密码是否正确
- 检查JWT密钥是否配置
- 检查令牌是否过期
- 检查认证中间件配置

## 12. 总结与展望

### 12.1 项目总结

秉羲管理系统是一个功能完整、架构清晰的企业资源规划系统，专为面料行业定制。系统采用现代化的技术栈，具有以下特点：

- **高性能**：基于Rust语言开发，性能优异，资源占用低
- **可扩展**：模块化设计，易于添加新功能
- **行业特色**：支持面料行业的特殊需求，如批次管理、双计量单位等
- **完整功能**：涵盖企业管理的各个方面，从采购到销售，从库存到财务
- **易于部署**：提供完整的部署脚本和监控方案
- **安全可靠**：完善的认证授权机制，数据安全保障

### 12.2 未来展望

- **移动端支持**：开发移动应用，支持随时随地访问系统
- **AI集成**：引入人工智能技术，优化预测和决策
- **区块链集成**：使用区块链技术，增强供应链透明度
- **更丰富的行业功能**：进一步完善面料行业特色功能
- **多语言支持**：支持多语言界面，满足国际化需求
- **更强大的数据分析**：提供更丰富的数据分析和报表功能

## 13. 附录

### 13.1 环境变量配置

**示例 .env 文件**：
```env
# 数据库配置
DATABASE_URL=postgres://admin:password@localhost:5432/bingxi

# 服务器配置
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# 认证配置
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production

# 日志配置
LOG_LEVEL=info

# CORS配置
CORS_ALLOWED_ORIGINS=*
```

### 13.2 系统架构图

![系统架构图](https://trae-api-cn.mchost.guru/api/ide/v1/text_to_image?prompt=ERP%20system%20architecture%20diagram%20with%20frontend%20and%20backend%20components%2C%20database%2C%20and%20monitoring%20systems&image_size=landscape_16_9)

### 13.3 数据流程图

![数据流程图](https://trae-api-cn.mchost.guru/api/ide/v1/text_to_image?prompt=Data%20flow%20diagram%20for%20ERP%20system%20showing%20data%20movement%20between%20modules&image_size=landscape_16_9)

### 13.4 联系方式

**开发团队**：秉羲团队
**邮箱**：contact@bingxi-erp.com
**网站**：https://www.bingxi-erp.com

---

*本文档由秉羲团队维护，定期更新。*

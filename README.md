# 秉羲面料管理

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

秉羲面料管理是一个专为面料行业定制的企业资源规划（ERP）系统，采用现代化的技术栈，提供完整的企业管理功能。系统采用前后端分离架构，支持多租户、模块化设计，具有高性能、高安全性、高可扩展性等特点。

### 核心特性

- **完整的ERP功能**：从采购到销售，从库存到财务，从生产到分析
- **面料行业特色功能**：批次管理、双计量单位、缸号管理、色号管理、坯布管理
- **高性能**：基于Rust语言开发后端，性能优异
- **高安全性**：JWT认证、权限管理、请求验证、CORS配置
- **高可扩展性**：模块化设计，易于添加新功能
- **完整的监控体系**：Prometheus + Grafana + Alertmanager

---

## 系统架构

### 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                         前端层                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Vue 3.4 + TypeScript + Element Plus                │   │
│  │  - 路由管理 (Vue Router)                            │   │
│  │  - 状态管理 (Pinia)                                 │   │
│  │  - HTTP客户端 (Axios)                               │   │
│  └─────────────────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────────────────┘
                       │ HTTP + JSON
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                      Nginx 反向代理                           │
│  - 静态资源托管                                             │
│  - API 反向代理                                             │
│  - 负载均衡                                                 │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────────┐
│                      后端层 (Axum)                           │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │  中间件  │  │  路由层  │  │  处理层  │          │   │
│  │  │  └──┘  └────┬─────┘  └────┬─────┘          │   │
│  │  ┌──────────────────────────────────────────┐  │   │
│  │  │          业务逻辑层                    │  │   │
│  │  │  (Services)                          │  │   │
│  │  └──────────────────────────────────────────┘  │   │
│  │  ┌──────────────────────────────────────────┐  │   │
│  │  │          数据访问层                    │  │   │
│  │  │  (SeaORM)                        │  │   │
│  │  └──────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────────┐
│                      数据层                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  PostgreSQL  │  │   Redis     │  │  Prometheus  │      │
│  │   (主数据库)  │  │   (缓存)    │  │  (监控数据)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### 架构设计原则

1. **前后端分离**：前端使用Vue 3 + TypeScript，后端使用Axum，通过HTTP/JSON通信
2. **分层架构**：中间件层、路由层、处理层、业务逻辑层、数据访问层
3. **模块化设计**：每个功能模块独立，易于扩展和维护
4. **高可用性**：支持负载均衡、缓存、监控等

---

## 技术栈详解

### 后端技术栈

| 技术 | 版本 | 用途 | 文件位置 |
|------|------|------|---------|
| Rust | 2021 | 后端开发语言 | backend/Cargo.toml |
| Axum | 0.7 | Web框架 | backend/src/main.rs |
| SeaORM | - | 数据库ORM | backend/src/database/mod.rs |
| PostgreSQL | 14+ | 关系型数据库 | backend/.env.example |
| Tonic | - | gRPC框架 | backend/proto/bingxi.proto |
| JWT | - | 身份认证 | backend/src/services/auth_service.rs |
| Tracing | - | 日志追踪 | backend/src/middleware/logger_middleware.rs |
| Prometheus | - | 指标收集 | backend/src/middleware/metrics.rs |

### 前端技术栈

| 技术 | 版本 | 用途 | 文件位置 |
|------|------|------|---------|
| Vue | 3.4 | 前端框架 | frontend/package.json |
| TypeScript | 5.4 | 类型系统 | frontend/tsconfig.json |
| Element Plus | 2.6 | UI组件库 | frontend/package.json |
| Vue Router | 4.3 | 路由管理 | frontend/src/router/index.ts |
| Pinia | 2.1 | 状态管理 | frontend/src/store/index.ts |
| Axios | 1.6 | HTTP客户端 | frontend/src/api/request.ts |
| Vite | 5.2 | 构建工具 | frontend/vite.config.ts |

### 部署与监控技术栈

| 技术 | 用途 | 文件位置 |
|------|------|---------|
| Nginx | 反向代理和静态资源托管 | deploy/nginx.conf |
| Systemd | 系统服务管理 | deploy/bingxi-backend.service |
| Prometheus | 指标收集 | monitoring/prometheus/prometheus.yml |
| Grafana | 指标可视化 | monitoring/grafana/dashboards/bingxi-erp-overview.json |
| Alertmanager | 告警管理 | monitoring/alertmanager/alertmanager.yml |

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
- backend/src/models/user.rs - 用户模型
- backend/src/services/user_service.rs - 用户服务
- backend/src/handlers/user_handler.rs - 用户处理函数

**前端页面**：
- frontend/src/views/system/users/index.vue - 用户列表页面

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
- backend/src/models/role.rs - 角色模型
- backend/src/models/role_permission.rs - 角色权限模型
- backend/src/services/role_permission_service.rs - 角色权限服务
- backend/src/handlers/role_handler.rs - 角色处理函数

**前端页面**：
- frontend/src/views/system/roles/index.vue - 角色列表页面

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
- backend/src/models/department.rs - 部门模型
- backend/src/services/department_service.rs - 部门服务
- backend/src/handlers/department_handler.rs - 部门处理函数

**前端页面**：
- frontend/src/views/system/departments/index.vue - 部门列表页面

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
- backend/src/services/auth_service.rs - 认证服务
- backend/src/handlers/auth_handler.rs - 认证处理函数
- backend/src/middleware/auth.rs - 认证中间件
- backend/src/middleware/auth_context.rs - 认证上下文

**前端页面**：
- frontend/src/views/Login.vue - 登录页面

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
- backend/src/models/product.rs - 产品模型
- backend/src/services/product_service.rs - 产品服务
- backend/src/handlers/product_handler.rs - 产品处理函数

**前端页面**：
- frontend/src/views/fabric/index.vue - 面料管理页面

**API接口**：
- `GET /api/v1/erp/products` - 获取产品列表
- `POST /api/v1/erp/products` - 创建产品
- `GET /api/v1/erp/products/:id` - 获取产品详情
- `PUT /api/v1/erp/products/:id` - 更新产品
- `DELETE /api/v1/erp/products/:id` - 删除产品

### 3. 库存管理模块

#### 3.1 库存查询

**功能描述**：
- 库存列表查询
- 库存统计
- 库存明细查询
- 库存预警

**核心模型**：
- backend/src/models/inventory_stock.rs - 库存模型
- backend/src/models/inventory_transaction.rs - 库存交易模型
- backend/src/services/inventory_stock_service.rs - 库存服务
- backend/src/handlers/inventory_stock_handler.rs - 库存处理函数

**前端页面**：
- frontend/src/views/inventory/index.vue - 库存管理页面

**API接口**：
- `GET /api/v1/erp/inventory/stock` - 获取库存列表
- `POST /api/v1/erp/inventory/stock` - 创建库存记录
- `GET /api/v1/erp/inventory/stock/:id` - 获取库存详情

### 4. 销售管理模块

#### 4.1 销售订单管理

**功能描述**：
- 销售订单创建
- 销售订单审批
- 销售订单执行
- 销售订单查询
- 销售订单状态管理

**核心模型**：
- backend/src/models/sales_order.rs - 销售订单模型
- backend/src/models/sales_order_item.rs - 销售订单明细模型
- backend/src/services/sales_service.rs - 销售服务
- backend/src/handlers/sales_order_handler.rs - 销售订单处理函数

**前端页面**：
- frontend/src/views/sales/index.vue - 销售管理页面

**API接口**：
- `GET /api/v1/erp/sales/orders` - 获取销售订单列表
- `POST /api/v1/erp/sales/orders` - 创建销售订单
- `GET /api/v1/erp/sales/orders/:id` - 获取销售订单详情
- `PUT /api/v1/erp/sales/orders/:id` - 更新销售订单
- `DELETE /api/v1/erp/sales/orders/:id` - 删除销售订单

---

## 数据库设计

### 数据库迁移文件

系统使用数据库迁移文件来管理数据库结构，所有迁移文件位于 backend/database/migration/ 目录。

主要迁移文件包括：
- 001_init.sql - 初始数据库结构
- 002_inventory_reservation.sql - 库存预留
- 003_foreign_keys.sql - 数据关联外键
- 004_customers.sql - 客户管理
- 005_fabric_industry_adaptation.sql - 面料行业适配

---

## API接口文档

### 路由配置

系统的路由配置位于 backend/src/routes/mod.rs。

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

#### 3. 库存管理接口
- `GET /api/v1/erp/inventory/stock` - 获取库存列表
- `POST /api/v1/erp/inventory/stock` - 创建库存记录

#### 4. 销售管理接口
- `GET /api/v1/erp/sales/orders` - 获取销售订单列表
- `POST /api/v1/erp/sales/orders` - 创建销售订单

#### 5. 仪表盘接口
- `GET /api/v1/erp/dashboard` - 获取仪表盘数据

#### 6. 健康检查接口
- `GET /health` - 健康检查
- `GET /readiness` - 就绪检查
- `GET /liveness` - 存活检查

---

## 部署指南

项目支持纯物理机环境的一键自动化部署，无需Docker容器。该脚本将自动下载最新代码、配置运行环境、设置Nginx反向代理，并注册Systemd服务实现开机自启和崩溃保活。

### 1. 一键快速部署

在您的Linux服务器（推荐Ubuntu/Debian/CentOS）上，直接运行以下命令即可完成全自动安装：

```bash
curl -fsSL https://cdn.jsdelivr.net/gh/57231307/1@main/快速部署/install.sh | sudo bash -s install
```

### 2. 一键管理工具 (bingxi)

安装成功后，系统会自动为您在终端注入一个叫做 `bingxi` 的全局命令。后续日常运维非常极简：

```bash
sudo bingxi start    # 启动系统（后端及Nginx网关）
sudo bingxi stop     # 停止系统
sudo bingxi restart  # 重启系统
sudo bingxi status   # 查看系统运行与保活状态
sudo bingxi update   # 一键平滑升级（自动拉取最新Release包并平滑重启）
```

### 3. 环境要求与手动配置

**硬件要求**：
- CPU：至少2核
- 内存：至少4GB
- 磁盘：至少20GB
- 操作系统：Linux（推荐Ubuntu 20.04+）

**手动配置（可选）**：
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
npm install
```

#### 1.2 配置开发环境

```bash
cd backend
cp .env.example .env
# 编辑.env文件，配置开发环境
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
npm run dev
```

前端开发服务器默认运行在 http://localhost:3000

#### 2.2 代码规范

**命名约定**：
- 变量和函数：蛇形命名法（snake_case）- Rust后端
- 变量和函数：驼峰命名法（camelCase）- TypeScript前端
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

# 运行前端类型检查
cd frontend
npm run build
```

#### 3.2 集成测试

```bash
# 运行后端集成测试
cd backend
cargo test --test api_test
cargo test --test auth_integration_test
```

---

## 安全机制

### 1. 身份认证

系统使用JWT（JSON Web Token）进行身份认证。

**核心实现**：
- backend/src/services/auth_service.rs - 认证服务
- backend/src/middleware/auth.rs - 认证中间件
- backend/src/middleware/auth_context.rs - 认证上下文

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
- backend/src/middleware/permission.rs - 权限中间件
- backend/src/models/role.rs - 角色模型
- backend/src/models/role_permission.rs - 角色权限模型

**前端权限实现**：
- frontend/src/store/permission.ts - 权限状态管理
- frontend/src/router/index.ts - 路由守卫

### 3. CORS配置

系统配置了跨域资源共享（CORS），确保前端可以安全访问后端API。

**核心实现**：
- backend/src/routes/mod.rs - CORS配置

### 4. 路由保护

前端实现了路由保护，确保未登录用户只能访问登录页面。

**核心实现**：
- frontend/src/router/index.ts - 路由守卫

---

## 监控与运维

### 1. 监控系统

系统集成了完整的监控体系，包括Prometheus、Grafana和Alertmanager。

#### 1.1 Prometheus配置

**配置文件**：
- monitoring/prometheus/prometheus.yml - Prometheus主配置
- monitoring/prometheus/alert_rules.yml - 告警规则

**监控指标**：
- 系统指标（CPU、内存、磁盘）
- 应用指标（请求数、响应时间、错误率）
- 数据库指标（连接数、查询性能）
- 业务指标（订单量、库存水平）

#### 1.2 Grafana配置

**配置文件**：
- monitoring/grafana/dashboards/bingxi-erp-overview.json - 仪表盘配置

#### 1.3 Alertmanager配置

**配置文件**：
- monitoring/alertmanager/alertmanager.yml - Alertmanager配置

### 2. 日志管理

**日志配置**：
- 使用Tracing进行结构化日志记录
- 日志级别：debug、info、warn、error

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
```

---

## 总结

秉羲面料管理是一个功能完整、架构清晰的企业资源规划系统，专为面料行业定制。系统具有以下特点：

- **高性能**：后端基于Rust语言开发，性能优异
- **现代化前端**：Vue 3.4 + TypeScript + Element Plus
- **可扩展**：模块化设计，易于添加新功能
- **行业特色**：支持面料行业的特殊需求，如批次管理、双计量单位等
- **完整功能**：涵盖企业管理的各个方面，从采购到销售，从库存到财务
- **易于部署**：提供完整的部署脚本和监控方案
- **高安全性**：JWT认证、权限管理、CORS配置
- **完整监控**：Prometheus + Grafana + Alertmanager

系统已经具备生产环境运行的条件，可以根据企业的具体需求进行定制和扩展。

---

*本文档由秉羲团队维护，定期更新。*

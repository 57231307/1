# 秉羲管理系统 - Code Wiki

## 1. 项目概述

秉羲管理系统是一个基于Rust语言开发的企业资源规划（ERP）系统，专为面料行业定制，提供完整的企业管理功能。

**主要功能模块**：
- 用户与权限管理
- 产品管理
- 仓库管理
- 库存管理（支持面料行业特色功能）
- 销售管理
- 采购管理
- 财务管理
- 生产管理
- 数据分析与报表

**系统特点**：
- 基于Rust语言开发，性能优异
- 前后端分离架构
- 支持多租户
- 模块化设计，易于扩展
- 完整的面料行业特色功能

## 2. 技术栈

### 后端技术
- **语言**：Rust 2021
- **Web框架**：Axum 0.7
- **数据库**：PostgreSQL
- **ORM**：SeaORM
- **认证**：JWT
- **gRPC**：Tonic
- **API文档**：OpenAPI/Swagger

### 前端技术
- **框架**：Yew 0.21 (Rust WebAssembly)
- **路由**：Yew Router
- **状态管理**：Yew Context API
- **HTTP客户端**：Gloo Net

### 部署与监控
- **部署**：Systemd + Nginx
- **监控**：Prometheus + Grafana
- **日志**：Tracing

## 3. 项目结构

### 目录结构

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

### 核心模块职责

| 模块 | 主要职责 | 文件位置 |
|------|---------|----------|
| 认证模块 | 用户登录、注销、令牌刷新 | [backend/src/services/auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) |
| 用户管理 | 用户CRUD操作 | [backend/src/services/user_service.rs](file:///workspace/backend/src/services/user_service.rs) |
| 库存管理 | 库存查询、调整、盘点 | [backend/src/services/inventory_stock_service.rs](file:///workspace/backend/src/services/inventory_stock_service.rs) |
| 销售管理 | 销售订单处理 | [backend/src/services/sales_service.rs](file:///workspace/backend/src/services/sales_service.rs) |
| 采购管理 | 采购订单处理 | [backend/src/services/purchase_order_service.rs](file:///workspace/backend/src/services/purchase_order_service.rs) |
| 财务管理 | 财务凭证、发票处理 | [backend/src/services/voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) |
| 面料行业特色 | 批次管理、双计量单位 | [backend/src/services/batch_service.rs](file:///workspace/backend/src/services/batch_service.rs) |

## 4. 核心功能模块

### 4.1 认证与用户管理

**认证服务**：
- JWT令牌生成与验证
- 密码哈希与验证
- 用户认证逻辑

**用户管理**：
- 用户CRUD操作
- 角色分配
- 部门关联
- 权限管理

### 4.2 库存管理

**核心功能**：
- 库存查询与统计
- 库存调拨
- 库存盘点
- 库存调整

**面料行业特色**：
- 批次管理
- 色号管理
- 双计量单位（米/公斤）自动换算
- 缸号管理
- 坯布管理

### 4.3 销售管理

**核心功能**：
- 销售订单创建与管理
- 销售合同管理
- 销售价格管理
- 销售分析

**面料行业特色**：
- 面料销售订单（支持米/公斤双计量单位）
- 色号管理
- 批次追踪

### 4.4 采购管理

**核心功能**：
- 采购订单创建与管理
- 采购合同管理
- 采购收货
- 采购退货
- 采购价格管理

### 4.5 财务管理

**核心功能**：
- 总账管理
- 科目管理
- 凭证管理
- 应付账款
- 应收账款
- 财务分析

### 4.6 业务追溯

**核心功能**：
- 五维度管理（产品、批次、色号、等级、仓库）
- 业务流程追溯
- 数据快照

## 5. 数据库结构

### 核心表结构

| 表名 | 主要功能 | 关键字段 |
|------|---------|----------|
| users | 用户信息 | id, username, password_hash, role_id |
| roles | 角色信息 | id, name, permissions |
| departments | 部门信息 | id, name, parent_id |
| products | 产品信息 | id, code, name, category_id |
| product_categories | 产品类别 | id, name, parent_id |
| warehouses | 仓库信息 | id, code, name, status |
| inventory_stocks | 库存信息 | id, product_id, warehouse_id, batch_no, color_code |
| inventory_transfers | 库存调拨 | id, transfer_no, from_warehouse_id, to_warehouse_id |
| sales_orders | 销售订单 | id, order_no, customer_name, status |
| purchase_orders | 采购订单 | id, order_no, supplier_id, status |
| finance_invoices | 发票信息 | id, invoice_no, amount, status |
| vouchers | 财务凭证 | id, voucher_no, amount, status |

### 面料行业特色表

| 表名 | 主要功能 | 关键字段 |
|------|---------|----------|
| dye_batches | 染色批次 | id, batch_no, color_code, status |
| greige_fabrics | 坯布管理 | id, batch_no, supplier_id, quantity |
| dye_recipes | 染色配方 | id, recipe_no, color_code, formula |
| inventory_stocks | 库存信息（扩展） | batch_no, color_code, quantity_meters, quantity_kg |

## 6. API接口

### 6.1 认证接口

| 路径 | 方法 | 功能 | 模块 |
|------|------|------|------|
| /api/v1/erp/auth/login | POST | 用户登录 | [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) |
| /api/v1/erp/auth/logout | POST | 用户注销 | [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) |
| /api/v1/erp/auth/refresh | POST | 刷新令牌 | [auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) |

### 6.2 用户管理接口

| 路径 | 方法 | 功能 | 模块 |
|------|------|------|------|
| /api/v1/erp/users | GET | 获取用户列表 | [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) |
| /api/v1/erp/users | POST | 创建用户 | [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) |
| /api/v1/erp/users/:id | GET | 获取用户详情 | [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) |
| /api/v1/erp/users/:id | PUT | 更新用户 | [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) |
| /api/v1/erp/users/:id | DELETE | 删除用户 | [user_handler.rs](file:///workspace/backend/src/handlers/user_handler.rs) |

### 6.3 库存管理接口

| 路径 | 方法 | 功能 | 模块 |
|------|------|------|------|
| /api/v1/erp/inventory/stock | GET | 获取库存列表 | [inventory_stock_handler.rs](file:///workspace/backend/src/handlers/inventory_stock_handler.rs) |
| /api/v1/erp/inventory/stock | POST | 创建库存记录 | [inventory_stock_handler.rs](file:///workspace/backend/src/handlers/inventory_stock_handler.rs) |
| /api/v1/erp/inventory/transfers | GET | 获取调拨列表 | [inventory_transfer_handler.rs](file:///workspace/backend/src/handlers/inventory_transfer_handler.rs) |
| /api/v1/erp/inventory/transfers | POST | 创建调拨单 | [inventory_transfer_handler.rs](file:///workspace/backend/src/handlers/inventory_transfer_handler.rs) |
| /api/v1/erp/inventory/counts | GET | 获取盘点列表 | [inventory_count_handler.rs](file:///workspace/backend/src/handlers/inventory_count_handler.rs) |
| /api/v1/erp/inventory/counts | POST | 创建盘点单 | [inventory_count_handler.rs](file:///workspace/backend/src/handlers/inventory_count_handler.rs) |

### 6.4 销售管理接口

| 路径 | 方法 | 功能 | 模块 |
|------|------|------|------|
| /api/v1/erp/sales/orders | GET | 获取销售订单列表 | [sales_order_handler.rs](file:///workspace/backend/src/handlers/sales_order_handler.rs) |
| /api/v1/erp/sales/orders | POST | 创建销售订单 | [sales_order_handler.rs](file:///workspace/backend/src/handlers/sales_order_handler.rs) |
| /api/v1/erp/sales/fabric-orders | GET | 获取面料销售订单列表 | [sales_fabric_order_handler.rs](file:///workspace/backend/src/handlers/sales_fabric_order_handler.rs) |
| /api/v1/erp/sales/fabric-orders | POST | 创建面料销售订单 | [sales_fabric_order_handler.rs](file:///workspace/backend/src/handlers/sales_fabric_order_handler.rs) |

## 7. 部署与监控

### 7.1 部署方案

**部署流程**：
1. 解压发布包到部署目录
2. 配置系统服务（systemd）
3. 配置Nginx反向代理
4. 启动服务

**部署脚本**：
- [deploy.sh](file:///workspace/deploy/deploy.sh) - 主部署脚本
- [deploy-backend.sh](file:///workspace/deploy/deploy-backend.sh) - 后端部署脚本
- [deploy-frontend.sh](file:///workspace/deploy/deploy-frontend.sh) - 前端部署脚本

**系统服务配置**：
- [bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service) - 后端服务配置

**Nginx配置**：
- [nginx.conf](file:///workspace/deploy/nginx.conf) - Nginx主配置
- [nginx-simple.conf](file:///workspace/deploy/nginx-simple.conf) - 简化版配置

### 7.2 监控方案

**监控组件**：
- **Prometheus** - 指标收集
- **Grafana** - 指标可视化
- **Alertmanager** - 告警管理

**监控配置**：
- [prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml) - Prometheus配置
- [alert_rules.yml](file:///workspace/monitoring/prometheus/alert_rules.yml) - 告警规则
- [alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml) - Alertmanager配置
- [bingxi-erp-overview.json](file:///workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json) - Grafana仪表盘

**监控指标**：
- 系统指标（CPU、内存、磁盘）
- 应用指标（请求数、响应时间）
- 数据库指标（连接数、查询性能）
- 业务指标（订单量、库存水平）

## 8. 运行方式

### 8.1 开发环境

**后端运行**：
```bash
# 进入后端目录
cd backend

# 安装依赖
cargo build

# 运行开发服务器
cargo run
```

**前端运行**：
```bash
# 进入前端目录
cd frontend

# 安装依赖
cargo build

# 运行开发服务器
trunk serve
```

### 8.2 生产环境

**构建发布包**：
```bash
# 构建后端
cd backend
cargo build --release

# 构建前端
cd ../frontend
trunk build --release
```

**部署**：
```bash
# 使用部署脚本
./deploy/deploy.sh
```

**服务管理**：
```bash
# 启动服务
sudo systemctl start bingxi-backend

# 停止服务
sudo systemctl stop bingxi-backend

# 重启服务
sudo systemctl restart bingxi-backend

# 查看服务状态
sudo systemctl status bingxi-backend
```

## 9. 开发指南

### 9.1 代码规范

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

### 9.2 开发流程

**新功能开发**：
1. 创建分支
2. 实现功能
3. 编写测试
4. 提交代码
5. 进行代码审查
6. 合并到主分支

**Bug修复**：
1. 复现问题
2. 分析原因
3. 修复代码
4. 编写测试
5. 验证修复
6. 提交代码

### 9.3 测试指南

**单元测试**：
```bash
# 运行后端单元测试
cd backend
cargo test

# 运行前端单元测试
cd ../frontend
cargo test
```

**集成测试**：
```bash
# 运行后端集成测试
cd backend
cargo test --test api_test
```

**性能测试**：
```bash
# 使用ab进行性能测试
./backend/scripts/performance_test_ab.sh

# 使用wrk进行性能测试
./backend/scripts/performance_test_wrk.sh
```

## 10. 总结

秉羲管理系统是一个功能完整、架构清晰的企业资源规划系统，专为面料行业定制。系统采用现代化的技术栈，具有以下特点：

- **高性能**：基于Rust语言开发，性能优异
- **可扩展**：模块化设计，易于添加新功能
- **行业特色**：支持面料行业的特殊需求，如批次管理、双计量单位等
- **完整功能**：涵盖企业管理的各个方面，从采购到销售，从库存到财务
- **易于部署**：提供完整的部署脚本和监控方案

系统已经具备生产环境运行的条件，可以根据企业的具体需求进行定制和扩展。

## 11. 附录

### 11.1 环境变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| DATABASE_URL | 数据库连接字符串 | - |
| JWT_SECRET | JWT密钥 | - |
| SERVER_HOST | 服务器主机 | 0.0.0.0 |
| SERVER_PORT | 服务器端口 | 8080 |
| LOG_LEVEL | 日志级别 | info |
| CORS_ALLOWED_ORIGINS | CORS允许的来源 | * |

### 11.2 系统要求

**硬件要求**：
- CPU：至少2核
- 内存：至少2GB
- 磁盘：至少20GB

**软件要求**：
- Rust 1.70+ 
- PostgreSQL 14+
- Nginx 1.18+
- Systemd
- Trunk (前端构建工具)

### 11.3 常见问题

**数据库连接失败**：
- 检查数据库服务是否运行
- 检查连接字符串是否正确
- 检查数据库用户权限

**服务启动失败**：
- 检查端口是否被占用
- 检查配置文件是否正确
- 查看日志文件获取详细错误信息

**前端访问404**：
- 检查Nginx配置是否正确
- 检查前端文件是否正确部署
- 检查路由配置是否正确

**性能问题**：
- 检查数据库索引
- 检查SQL查询优化
- 考虑使用缓存

## 12. 联系方式

**开发团队**：秉羲团队
**邮箱**：contact@bingxi-erp.com
**网站**：https://www.bingxi-erp.com

---

*本文档由秉羲团队维护，定期更新。*
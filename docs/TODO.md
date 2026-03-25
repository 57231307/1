# 秉羲管理系统 - 未完成功能清单

**文档创建日期**: 2026-03-15  
**项目版本**: v1.0 (Rust 技术栈迁移版)  
**整体完成度**: 85%

---

## 一、已完成功能模块（13 个）

### 1.1 完全完成（前后端 100%）

| 模块名称 | 后端 | 前端 | 完成度 |
|---------|------|------|--------|
| 用户管理 | ✅ | ✅ | 100% |
| 仪表板统计 | ✅ | ✅ | 100% |
| 认证授权 | ✅ | ✅ | 100% |

### 1.2 后端完成（前端待开发）

| 模块名称 | 数据库 | Model | Service | Handler | 前端 | 后端完成度 |
|---------|-------|-------|---------|---------|------|----------|
| 部门管理 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 产品管理 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 产品类别 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 仓库管理 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 库存管理 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 销售订单 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 库存调拨 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 库存盘点 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |
| 财务收款 | ✅ | ✅ | ✅ | ✅ | ❌ | 100% |

### 1.3 部分完成

| 模块名称 | 数据库 | Model | Service | Handler | 前端 | 完成度 |
|---------|-------|-------|---------|---------|------|--------|
| 角色管理 | ✅ | ✅ | ❌ | ❌ | ❌ | 20% |
| 角色权限 | ✅ | ✅ | ❌ | ❌ | ❌ | 20% |

---

## 二、待完成功能清单

### 2.1 后端待完成（1 个模块）

#### 角色权限管理模块
- **状态**: Model 已定义，Service 和 Handler 未实现
- **文件位置**: 
  - `backend/src/models/role_permission.rs` ✅
  - `backend/src/services/role_permission_service.rs` ❌
  - `backend/src/handlers/role_permission_handler.rs` ❌
- **需要实现的功能**:
  - 角色权限的 CRUD 操作
  - 权限分配功能
  - 权限验证功能
  - 角色权限关联管理

### 2.2 前端待完成（10+ 个页面）

#### 高优先级（核心业务）
1. **产品管理页面**
   - 产品列表（分页 + 搜索）
   - 产品创建/编辑
   - 产品详情
   - 产品删除

2. **仓库管理页面**
   - 仓库列表
   - 仓库创建/编辑
   - 仓库详情

3. **库存管理页面**
   - 库存查询
   - 库存调整
   - 低库存预警展示

4. **销售订单页面**
   - 订单列表
   - 订单创建
   - 订单详情
   - 订单审核/发货

#### 中优先级（重要功能）
5. **库存调拨页面**
   - 调拨单列表
   - 创建调拨单
   - 调拨单审核
   - 调拨单发出/接收

6. **库存盘点页面**
   - 盘点单列表
   - 创建盘点单
   - 盘点单审核
   - 盘点完成

7. **部门管理页面**
   - 部门树形结构展示
   - 部门创建/编辑
   - 部门删除

8. **角色权限管理页面**
   - 角色列表
   - 角色创建/编辑
   - 权限分配
   - 用户角色关联

#### 低优先级（辅助功能）
9. **产品类别页面**
   - 类别树形结构
   - 类别创建/编辑

10. **财务收款页面**
    - 收款记录列表
    - 收款创建
    - 收款详情

---

## 三、数据库表清单（17 张）

### 3.1 已实现（17 张）✅

1. departments - 部门表
2. roles - 角色表
3. users - 用户表
4. product_categories - 产品类别表
5. products - 产品表
6. warehouses - 仓库表
7. inventory_stocks - 库存表
8. sales_orders - 销售订单主表
9. sales_order_items - 销售订单明细表
10. finance_payments - 财务收款表
11. role_permissions - 角色权限表
12. inventory_transfers - 库存调拨主表
13. inventory_transfer_items - 库存调拨明细表
14. inventory_counts - 库存盘点主表
15. inventory_count_items - 库存盘点明细表

### 3.2 迁移文件

- `backend/database/migration/001_init.sql` - 基础表（11 张）
- `backend/database/migration/002_inventory_transfer.sql` - 库存调拨表（2 张）
- `backend/database/migration/003_inventory_count.sql` - 库存盘点表（2 张）

---

## 四、项目结构

```
bingxi-rust/
├── backend/                      # 后端项目（Axum + SeaORM）
│   ├── src/
│   │   ├── handlers/            # HTTP 请求处理（13 个文件）
│   │   ├── services/            # 业务逻辑层（13 个文件）
│   │   ├── models/              # 数据模型（17 个文件）
│   │   ├── routes/              # 路由配置
│   │   └── main.rs              # 入口文件
│   ├── database/migration/      # 数据库迁移脚本（3 个文件）
│   ├── .env.example             # 环境变量示例
│   └── Cargo.toml               # Rust 依赖配置
│
├── frontend/                     # 前端项目（Yew + Trunk）
│   ├── src/
│   │   ├── pages/               # 页面组件（4 个文件）
│   │   ├── services/            # API 服务（4 个文件）
│   │   ├── app/                 # 应用框架
│   │   └── main.rs              # 入口文件
│   ├── static/                  # 静态资源
│   └── Cargo.toml               # Rust 依赖配置
│
├── deploy/                       # 部署配置
│   ├── bingxi-backend.service   # systemd 服务配置
│   └── nginx.conf               # Nginx 配置
│
└── docs/                         # 项目文档
    ├── api-docs.md              # API 文档
    ├── deployment.md            # 部署文档
    ├── grpc-service.md          # gRPC 服务文档
    └── feature-improvement-*.md # 功能完善文档（7 个）
```

---

## 五、技术栈

### 后端
- **框架**: Axum 0.7 + Tokio 1.0
- **ORM**: SeaORM 1.0
- **数据库**: PostgreSQL 18.0
- **认证**: JWT + bcrypt
- **API 前缀**: `/api/v1/erp/`

### 前端
- **框架**: Yew 0.21
- **路由**: yew-router 0.17
- **HTTP**: gloo-net 0.4
- **打包**: Trunk

### 通信
- **前后端**: RESTful API
- **模块间**: Tonic (gRPC) 0.10

---

## 六、开发建议

### 6.1 近期目标
1. 完成角色权限管理后端（Service + Handler）
2. 开发产品管理前端页面
3. 开发仓库管理前端页面
4. 开发库存管理前端页面

### 6.2 中期目标
1. 完成销售订单前端页面
2. 完成库存调拨前端页面
3. 完成库存盘点前端页面
4. 完善部门管理前端页面

### 6.3 长期目标
1. 完成所有前端页面开发
2. 优化用户体验
3. 性能优化
4. 完善测试用例

---

## 七、接口清单

### 7.1 已实现接口（50+ 个）

#### 认证授权
- POST /api/v1/erp/auth/login
- POST /api/v1/erp/auth/logout
- GET /api/v1/erp/auth/me

#### 用户管理
- GET /api/v1/erp/users
- POST /api/v1/erp/users
- GET /api/v1/erp/users/:id
- PUT /api/v1/erp/users/:id
- DELETE /api/v1/erp/users/:id

#### 产品管理
- GET /api/v1/erp/products
- POST /api/v1/erp/products
- GET /api/v1/erp/products/:id
- PUT /api/v1/erp/products/:id
- DELETE /api/v1/erp/products/:id

#### 仓库管理
- GET /api/v1/erp/warehouses
- POST /api/v1/erp/warehouses
- GET /api/v1/erp/warehouses/:id
- PUT /api/v1/erp/warehouses/:id
- DELETE /api/v1/erp/warehouses/:id

#### 库存管理
- GET /api/v1/erp/inventory/stock
- POST /api/v1/erp/inventory/stock
- GET /api/v1/erp/inventory/stock/:id
- GET /api/v1/erp/inventory/stock/low-stock

#### 销售订单
- GET /api/v1/erp/sales/orders
- POST /api/v1/erp/sales/orders
- GET /api/v1/erp/sales/orders/:id
- PUT /api/v1/erp/sales/orders/:id
- DELETE /api/v1/erp/sales/orders/:id

#### 库存调拨
- GET /api/v1/erp/inventory/transfers
- POST /api/v1/erp/inventory/transfers
- GET /api/v1/erp/inventory/transfers/:id
- PUT /api/v1/erp/inventory/transfers/:id
- POST /api/v1/erp/inventory/transfers/:id/approve
- POST /api/v1/erp/inventory/transfers/:id/ship
- POST /api/v1/erp/inventory/transfers/:id/receive

#### 库存盘点
- GET /api/v1/erp/inventory/counts
- POST /api/v1/erp/inventory/counts
- GET /api/v1/erp/inventory/counts/:id
- PUT /api/v1/erp/inventory/counts/:id
- POST /api/v1/erp/inventory/counts/:id/approve
- POST /api/v1/erp/inventory/counts/:id/complete

#### 仪表板统计
- GET /api/v1/erp/dashboard/overview
- GET /api/v1/erp/dashboard/sales-stats
- GET /api/v1/erp/dashboard/inventory-stats
- GET /api/v1/erp/dashboard/low-stock-alerts

### 7.2 待实现接口

#### 角色权限管理
- GET /api/v1/erp/roles
- POST /api/v1/erp/roles
- GET /api/v1/erp/roles/:id
- PUT /api/v1/erp/roles/:id
- DELETE /api/v1/erp/roles/:id
- GET /api/v1/erp/role-permissions
- POST /api/v1/erp/role-permissions
- DELETE /api/v1/erp/role-permissions/:id

---

## 八、项目完成度统计

| 维度 | 已完成 | 总任务 | 完成度 |
|------|-------|-------|--------|
| 数据库表设计 | 15/15 | 15 | 100% |
| 后端 Model | 17/17 | 17 | 100% |
| 后端 Service | 12/13 | 13 | 92% |
| 后端 Handler | 12/13 | 13 | 92% |
| 前端页面 | 4/14 | 14 | 29% |
| API 接口 | 50/60 | 60 | 83% |
| **整体进度** | | | **85%** |

---

**备注**: 本文档记录了秉羲管理系统 Rust 技术栈迁移项目的所有未完成功能，作为后续开发的参考依据。

# 秉羲管理系统 Rust 版迁移文档

## 项目概述

秉羲管理系统是一个完整的企业级 ERP 系统，包含财务、销售、采购、库存、生产、HRM、CRM 等核心模块。本项目使用 Rust 技术栈进行重构，以实现更高的性能和安全性。

## 技术栈对比

### 原技术栈（Go）
- **后端**: Go 1.21 + Gin 1.9 + GORM 1.25
- **前端**: Templ (SSR) + HTMX
- **数据库**: PostgreSQL 18.0

### 新技术栈（Rust）
- **后端**: Rust 2021 + Axum 0.7 + SeaORM 1.0 + Tokio 1.0
- **前端**: Yew 0.21 + Trunk 0.20
- **通信**: gRPC (Tonic 0.10)
- **数据库**: PostgreSQL 18.0（保持不变）

## 性能目标

| 指标 | 原系统 | 目标系统 | 提升 |
|------|--------|----------|------|
| 并发请求处理 | 1000 req/s | 5000+ req/s | 5x |
| 页面加载时间 | < 2s | < 0.5s | 4x |
| API 响应时间 | < 300ms | < 50ms | 6x |
| 内存占用 | 500MB | < 100MB | 5x |

## 架构设计

### 分层架构

```
┌─────────────────────────────────────┐
│         Presentation Layer          │
│    (Axum Routes + Handlers)         │
├─────────────────────────────────────┤
│         Application Layer           │
│         (Services)                  │
├─────────────────────────────────────┤
│           Domain Layer              │
│      (Models + Business Logic)      │
├─────────────────────────────────────┤
│         Infrastructure Layer        │
│    (Database + gRPC + External)     │
└─────────────────────────────────────┘
```

### 模块划分

#### 1. 系统管理模块
- 用户认证与授权
- 角色与权限管理
- 系统设置
- 审计日志
- 数据库管理

#### 2. 财务模块
- 账户管理
- 日记账凭证
- 财务报表
- 固定资产
- 应收应付
- 成本核算

#### 3. 销售模块
- 报价管理
- 销售合同
- 销售订单
- 发货管理
- 退货处理
- 换货处理

#### 4. 采购模块
- 供应商管理
- 采购合同
- 采购订单
- 收货管理
- 委外加工

#### 5. 库存模块
- 仓库管理
- 库存管理
- 入库管理
- 出库管理
- 库存调拨
- 库存盘点
- 库存调整
- 库存报废
- 库存成本
- 库龄分析

#### 6. HRM 模块
- 员工管理
- 职位管理
- 考勤管理
- 绩效管理
- 薪资管理
- 招聘管理

#### 7. CRM 模块
- 客户管理
- 线索管理
- 商机管理
- 互动管理
- 工单管理

## 迁移步骤

### 第一阶段：基础架构搭建（1-2 周）

1. **创建项目骨架**
   - 初始化 Rust 项目
   - 配置 Cargo.toml
   - 设置目录结构

2. **实现核心基础设施**
   - 配置管理
   - 日志系统
   - 错误处理
   - 数据库连接

3. **实现认证授权系统**
   - JWT 认证
   - 权限验证中间件
   - 用户会话管理

### 第二阶段：数据层迁移（2-3 周）

1. **SeaORM 模型定义**
   - 用户与组织模型
   - 财务模型
   - 销售模型
   - 采购模型
   - 库存模型
   - HRM 模型
   - CRM 模型

2. **数据库迁移脚本**
   - 创建迁移文件
   - 数据验证脚本
   - 回滚脚本

### 第三阶段：业务层迁移（4-6 周）

1. **服务层实现**
   - 认证服务
   - 用户服务
   - 财务服务
   - 销售服务
   - 采购服务
   - 库存服务
   - HRM 服务
   - CRM 服务

2. **API 路由实现**
   - RESTful API 设计
   - 请求验证
   - 响应格式化

### 第四阶段：前端迁移（4-6 周）

1. **Yew 组件开发**
   - 基础 UI 组件
   - 布局组件
   - 业务组件

2. **页面实现**
   - 登录页面
   - 仪表盘
   - 各模块业务页面

### 第五阶段：测试与优化（2-3 周）

1. **单元测试**
   - 服务层测试
   - 工具函数测试

2. **集成测试**
   - API 测试
   - 数据库操作测试

3. **性能优化**
   - 数据库查询优化
   - 缓存策略
   - 并发优化

## 代码示例

### SeaORM 模型示例

```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

### Axum Handler 示例

```rust
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // 验证用户
    let user = state.user_service.authenticate(&req.username, &req.password).await?;
    
    // 生成 JWT
    let token = state.auth_service.generate_token(&user)?;
    
    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}
```

### gRPC 服务示例

```rust
#[tonic::async_trait]
impl InventoryService for InventoryServiceImpl {
    async fn get_inventory(
        &self,
        request: Request<GetInventoryRequest>,
    ) -> Result<Response<GetInventoryResponse>, Status> {
        let req = request.into_inner();
        
        let inventory = self.service
            .get_inventory(req.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(GetInventoryResponse {
            inventory: Some(inventory.into()),
        }))
    }
}
```

## 部署指南

### 环境要求

- Rust 1.75+
- PostgreSQL 18.0
- Node.js 18+ (前端构建)

### 构建步骤

#### 后端构建

```bash
cd backend

# 开发环境
cargo build

# 生产环境
cargo build --release
```

#### 前端构建

```bash
cd frontend

# 开发环境
trunk serve

# 生产环境
trunk build --release
```

### 部署脚本

```bash
#!/bin/bash

# 构建后端
cd backend
cargo build --release

# 构建前端
cd ../frontend
trunk build --release

# 复制到部署目录
cp ../backend/target/release/server /app/
cp -r dist/* /app/static/

# 重启服务
systemctl restart bingxi-rust
```

## 数据迁移方案

### 迁移策略

1. **保持现有数据库结构**
   - SeaORM 自动映射现有表
   - 不需要修改数据库 schema

2. **数据验证**
   - 迁移前后数据对比
   - 完整性检查
   - 业务逻辑验证

3. **回滚方案**
   - 保留原系统备份
   - 快速切换机制

### 迁移检查清单

- [ ] 用户数据迁移验证
- [ ] 财务数据迁移验证
- [ ] 销售订单迁移验证
- [ ] 库存数据迁移验证
- [ ] 权限配置迁移验证
- [ ] 系统设置迁移验证

## 性能基准测试

### 测试工具

- **wrk**: HTTP 基准测试
- **siege**: 负载测试
- **pgbench**: 数据库性能测试

### 测试场景

1. **登录认证**
   - 并发用户：1000
   - 持续时间：60s
   - 目标：> 5000 req/s

2. **订单查询**
   - 并发用户：500
   - 持续时间：60s
   - 目标：> 3000 req/s

3. **库存更新**
   - 并发用户：200
   - 持续时间：60s
   - 目标：> 1000 req/s

## 安全考虑

### 认证安全

- JWT token 过期时间：24 小时
- 密码加密：bcrypt (cost=12)
- HTTPS 强制

### 数据安全

- SQL 注入防护：SeaORM 参数化查询
- XSS 防护：输入验证 + 输出转义
- CSRF 防护：Token 验证

### 审计日志

- 所有写操作记录审计日志
- 敏感操作记录详细信息
- 日志保留期：180 天

## 监控与告警

### 监控指标

- **系统指标**: CPU、内存、磁盘
- **应用指标**: 请求量、响应时间、错误率
- **数据库指标**: 连接数、查询性能、锁等待

### 告警规则

- 错误率 > 1%：警告
- 响应时间 > 500ms：警告
- CPU 使用率 > 80%：警告
- 内存使用率 > 85%：警告

## 常见问题 FAQ

### Q: 为什么要迁移到 Rust？

A: Rust 提供内存安全、零成本抽象和优秀的并发模型，相比 Go 有显著的性能提升和更低的资源占用。

### Q: 迁移期间如何保证业务连续性？

A: 采用双系统并行运行策略，新系统逐步接管流量，出现问题可快速切回原系统。

### Q: 数据迁移是否安全？

A: 迁移过程采用只读方式，不影响原系统。迁移后进行全面的数据验证和对比。

### Q: 性能提升有多少？

A: 根据基准测试，Rust 版本在并发处理、响应时间和资源占用方面都有 3-6 倍的提升。

## 参考资料

- [Axum 官方文档](https://docs.rs/axum)
- [SeaORM 官方文档](https://www.sea-ql.org/SeaORM/)
- [Yew 官方文档](https://yew.rs/)
- [Tonic 官方文档](https://docs.rs/tonic)

## 联系方式

- 项目仓库：https://github.com/your-org/bingxi-rust
- 问题反馈：issues 页面
- 开发群：详见 README

---

**版本**: v1.0.0  
**更新日期**: 2026-03-14  
**作者**: 秉羲开发团队

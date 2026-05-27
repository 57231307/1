# 冰溪 ERP 文档

本文档提供了冰溪 ERP 系统的完整技术文档，包括系统架构、接口规范、开发指南和核心概念。文档面向开发人员、系统管理员和运维工程师。

**快速链接**: [架构](./ARCHITECTURE.md) | [接口](./INTERFACES.md) | [开发者指南](./DEVELOPER_GUIDE.md)

---

## 核心文档

### [架构](./ARCHITECTURE.md)
系统设计、技术栈、组件结构和数据流程。从这里开始了解系统如何运作。

### [接口](./INTERFACES.md)
公开 API、CLI 命令、事件和契约。集成或使用此系统的参考。

### [开发者指南](./DEVELOPER_GUIDE.md)
环境搭建、开发工作流、编码规范和常见任务。贡献者必读。

---

## 模块

| 模块 | 描述 | README |
|------|------|--------|
| `backend/` | Rust 后端服务，提供 REST API 和 gRPC 接口 | [README](../backend/README.md) |
| `frontend/` | Vue 3 前端应用，提供管理界面 | [README](../frontend/README.md) |
| `backend/src/handlers/` | HTTP 处理器层，处理请求和响应 | [README](./模块/handlers.md) |
| `backend/src/services/` | 业务逻辑层，实现核心业务功能 | [README](./模块/services.md) |
| `backend/src/models/` | 数据模型层，定义数据库实体 | [README](./模块/models.md) |
| `backend/src/middleware/` | 中间件层，处理认证、权限等 | [README](./模块/middleware.md) |
| `frontend/src/views/` | 页面组件，提供用户界面 | [README](./模块/views.md) |
| `frontend/src/api/` | API 调用层，与后端通信 | [README](./模块/api.md) |
| `frontend/src/store/` | 状态管理，管理应用状态 | [README](./模块/store.md) |

---

## 核心概念

理解这些领域概念有助于导航代码库：

| 概念 | 描述 |
|------|------|
| [用户](./专有概念/User.md) | 系统用户，包含认证和权限 |
| [产品](./专有概念/Product.md) | 面料产品，包含五维管理 |
| [销售订单](./专有概念/SalesOrder.md) | 销售交易及其生命周期 |
| [采购订单](./专有概念/PurchaseOrder.md) | 采购交易及其生命周期 |
| [库存](./专有概念/Inventory.md) | 库存管理和批次追踪 |
| [财务凭证](./专有概念/Voucher.md) | 会计凭证和账务处理 |
| [BPM 流程](./专有概念/BPM.md) | 审批工作流引擎 |
| [多租户](./专有概念/Tenant.md) | SaaS 多租户架构 |

---

## 入门指南

### 项目新人？

按此路径学习：
1. **[架构](./ARCHITECTURE.md)** - 了解全局
2. **[核心概念](#核心概念)** - 学习领域术语
3. **[开发者指南](./DEVELOPER_GUIDE.md)** - 搭建环境
4. **[接口](./INTERFACES.md)** - 探索公开 API

### 需要集成？

1. **[接口](./INTERFACES.md)** - API 契约和认证
2. **[架构](./ARCHITECTURE.md)** - 系统边界和数据流

### 首次贡献？

1. **[开发者指南](./DEVELOPER_GUIDE.md)** - 搭建和工作流
2. **[安全起步点](./DEVELOPER_GUIDE.md#常见任务)** - 低风险区域
3. **[常见任务](./DEVELOPER_GUIDE.md#常见任务)** - 分步指南

---

## 快速参考

### 命令

#### 后端 (Rust)

```bash
# 开发服务器
cargo run

# 运行测试
cargo test

# 代码检查
cargo clippy

# 代码格式化
cargo fmt

# 构建生产版本
cargo build --release
```

#### 前端 (Vue)

```bash
# 开发服务器
npm run dev

# 运行测试
npm run test

# 代码检查
npm run lint

# 代码格式化
npm run format

# 生产构建
npm run build
```

#### Docker

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down
```

### 重要文件

| 文件 | 目的 |
|------|------|
| `backend/src/main.rs` | 后端入口 |
| `frontend/src/main.ts` | 前端入口 |
| `backend/.env.example` | 后端环境变量模板 |
| `frontend/.env.development` | 前端开发环境变量 |
| `backend/Cargo.toml` | 后端依赖配置 |
| `frontend/package.json` | 前端依赖配置 |
| `docker-compose.yml` | Docker 编排配置 |

### 常见端口

| 服务 | 端口 | 用途 |
|------|------|------|
| 前端开发服务器 | 5173 | Vue 开发服务器 |
| 后端 API 服务器 | 8080 | REST API 和 gRPC |
| Swagger UI | 8080/swagger-ui | API 文档 |
| Prometheus | 9090 | 监控指标 |
| Grafana | 3000 | 监控面板 |
| PostgreSQL | 5432 | 数据库 |
| Redis | 6379 | 缓存 |

### 环境变量

#### 必需变量

```bash
# 数据库
DATABASE_URL=postgresql://user:password@localhost:5432/bingxi_erp

# Redis
REDIS_URL=redis://localhost:6379

# JWT
JWT_SECRET=your-secret-key
```

#### 可选变量

```bash
# 日志级别
RUST_LOG=info

# 服务器端口
SERVER_PORT=8080

# 邮件配置
SMTP_HOST=smtp.example.com
SMTP_PORT=587
```

---

## 业务模块

### 核心业务

| 模块 | 描述 | 状态 |
|------|------|------|
| 采购管理 | 供应商、采购订单、合同、价格、入库、退货 | ✅ 完成 |
| 销售管理 | 客户、销售订单、合同、价格、出库、退货 | ✅ 完成 |
| 库存管理 | 盘点、调拨、调整、批次管理、预警 | ✅ 完成 |
| 生产管理 | 生产订单、MRP、工序、质量控制 | ✅ 完成 |
| 财务管理 | 总账、应收应付、固定资产、资金、成本 | ✅ 完成 |
| CRM | 客户管理、商机、信用评估 | ✅ 完成 |

### 高级功能

| 功能 | 描述 | 状态 |
|------|------|------|
| AI 智能分析 | 销售预测、库存优化、异常检测 | ✅ 完成 |
| BPM 审批流 | 可配置工作流引擎 | ✅ 完成 |
| 报表引擎 | 自定义报表模板 | ✅ 完成 |
| 多租户 SaaS | 数据隔离、计费管理 | ✅ 完成 |
| 消息通知 | 站内信、邮件、短信 | ✅ 完成 |
| 移动端支持 | 响应式设计 | ✅ 完成 |

---

## 行业特性

### 面料行业专属功能

| 功能 | 描述 |
|------|------|
| 五维管理 | 产品-批次-色号-缸号-等级 |
| 双计量单位 | 米/公斤 自动换算 |
| 缸号管理 | 染色批次追踪 |
| 坯布管理 | 原料库存管理 |
| 染色配方 | 工艺配方管理 |
| 匹号管理 | 最小单位追踪 |

---

## 安全特性

### 认证与授权

- JWT + Cookie 双认证
- TOTP 两步验证
- 角色权限管理
- 字段级数据权限
- 数据范围控制

### 安全防护

- CSRF 防护
- XSS 防护
- SQL 注入防护
- 速率限制
- 暴力破解防护
- 安全响应头

### 审计日志

- 操作日志记录
- 登录日志追踪
- API 访问日志
- 财务审计日志

---

## 性能指标

### 系统性能

- **并发支持**: 1000+ 并发用户
- **响应时间**: < 200ms (95th percentile)
- **可用性**: 99.9%+
- **数据备份**: 每日自动备份

### 代码质量

- **测试覆盖率**: 70%+
- **代码审查**: 所有 PR 必须审查
- **静态分析**: Clippy + ESLint
- **文档覆盖**: 核心模块 100%

---

## 版本信息

- **当前版本**: 2.0.0
- **最后更新**: 2026-05-15
- **质量评分**: 7.8/10
- **许可证**: 专有软件

---

## 联系方式

- **项目地址**: https://github.com/57231307/1
- **问题反馈**: https://github.com/57231307/1/issues
- **技术支持**: support@bingxi.com
- **文档反馈**: docs@bingxi.com

---

## 更新日志

### v2.0.0 (2026-05-15)

- ✅ Trading 交易管理模块
- ✅ Advanced AI 分析模块
- ✅ 成本审核功能
- ✅ BPM 任务管理
- ✅ 固定资产折旧计算
- ✅ 客户信用评估模型
- ✅ 资金管理流水记录
- ✅ 库存盘点审批流程
- ✅ 路由配置完善 (96% 覆盖率)
- ✅ 单元测试覆盖 (26+ 个用例)

### v1.0.0 (2026-01-01)

- 初始发布版本
- 核心业务模块实现
- 基础架构搭建
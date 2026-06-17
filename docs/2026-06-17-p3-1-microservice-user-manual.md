# P3-1 微服务拆分 - 用户手册

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-1
> **关联**：Spec + Plan

---

## 一、什么是微服务拆分

冰溪 ERP 当前为**单体型架构**：所有业务模块（用户、库存、销售、生产、工艺、通知）打包在同一个 Rust 后端进程中。

随着业务复杂化，单体架构面临：
- **代码膨胀**：Rust 编译慢（5.8GB 沙箱 OOM 频发）
- **耦合严重**：改一处可能影响其他模块
- **伸缩困难**：高频模块（通知）和低频模块（工艺）无法独立伸缩
- **故障扩散**：单个模块崩溃影响整个系统

P3-1 微服务拆分的目标是**逐步**将单体拆分为独立部署的微服务。

## 二、P3-1 范围

P3-1 是**设计 + 关键路径 demo**阶段，**不**真的拆分所有模块：

✅ **已交付**：
- 完整微服务架构设计 spec
- 完整实施 plan
- 1 个**可独立运行**的微服务 demo：notifications
- 4 个 gRPC 端点（send_notification / batch_send / list / mark_as_read）
- Docker Compose 多服务编排

❌ **未交付**（P4+ 后续阶段）：
- 拆分剩余 5 个微服务（user / inventory / sales / production / process）
- API Gateway（Kong / APISIX）
- Kubernetes 部署
- 服务网格（Istio）
- 分布式追踪（Jaeger）

## 三、6 个微服务职责（设计）

| 微服务 | 端口 | 业务职责 |
|--------|------|----------|
| user | 50051 | 认证、用户、角色、权限、租户 |
| inventory | 50052 | 库存、出入库、SKU、仓库 |
| sales | 50053 | 客户、订单、报价单、合同 |
| production | 50054 | 工单、BOM、生产计划 |
| process | 50055 | 工艺配方、配方优化 |
| **notifications** | **50056** | **通知、日志、消息推送**（P3-1 demo）|

P3-1 仅实现 notifications，其他 5 个模块保留在主项目。

## 四、notifications 微服务本地启动

### 4.1 前置依赖

- Rust 1.94+
- PostgreSQL 15+
- protobuf-compiler（仅编译时需要）
- Docker + docker-compose（容器化启动）

### 4.2 启动 PostgreSQL

```bash
# 创建数据库
createdb notifications_db

# 执行 migration
psql notifications_db < microservices/notifications/migrations/001_init.sql
```

### 4.3 配置环境变量

```bash
export DATABASE_URL=postgres://erp:erp@localhost:5432/notifications_db
export GRPC_PORT=50056
export RUST_LOG=info
```

### 4.4 编译 + 启动

```bash
cd microservices/notifications
cargo build --release
./target/release/notifications-service
```

启动成功日志：
```
启动 notifications 微服务（端口 50056）
数据库连接成功
gRPC server 监听 0.0.0.0:50056
```

## 五、Docker Compose 一键启动

```bash
cd microservices
docker-compose up -d
```

启动 3 个 service：
- `postgres` - 共享数据库（端口 5432）
- `erp-backend` - 主项目（端口 8080）
- `notifications-service` - notifications 微服务（端口 50056）

查看日志：
```bash
docker-compose logs -f notifications-service
```

停止：
```bash
docker-compose down
```

## 六、gRPC 客户端调用示例

### 6.1 grpcurl 调用

```bash
# 发送通知
grpcurl -plaintext -d '{
  "tenant_id": 1,
  "user_id": 100,
  "title": "订单已创建",
  "content": "您的订单 #123 已创建",
  "category": "order",
  "priority": 5
}' localhost:50056 notifications.NotificationService/SendNotification

# 列出通知
grpcurl -plaintext -d '{
  "tenant_id": 1,
  "user_id": 100,
  "limit": 20,
  "offset": 0
}' localhost:50056 notifications.NotificationService/ListUserNotifications
```

### 6.2 响应示例

```json
// SendNotification 响应
{
  "id": "42",
  "status": "success"
}

// ListUserNotifications 响应
{
  "items": [
    {
      "id": "42",
      "tenantId": "1",
      "userId": "100",
      "title": "订单已创建",
      "content": "您的订单 #123 已创建",
      "category": "order",
      "priority": 5,
      "isRead": false,
      "createdAt": "2026-06-17T10:30:00Z"
    }
  ],
  "total": 1
}
```

## 七、与主项目关系

### 7.1 向后兼容

P3-1 **不修改**主项目任何代码：
- 主项目 `backend/src/services/notification_service.rs` 保留所有 HTTP REST 端点
- 主项目 `backend/src/routes/` 通知相关路由不变
- 前端 `frontend/src/api/` 通知相关 API 客户端不变

主项目用户**无感知**升级。

### 7.2 并行运行

通过 `microservices/docker-compose.yml`，主项目 + notifications 微服务**同时运行**：
- 主项目：HTTP REST 端点（端口 8080）
- 微服务：gRPC 端点（端口 50056）

未来通过 API Gateway 统一入口。

### 7.3 数据双写（Phase 1）

P3-1 阶段数据双写（feature flag 控制）：
- 主项目写主库（必需）
- 主项目**可选**写微服务（默认关闭）
- 微服务**仅**接收写入，不回读

## 八、架构图

```
┌──────────────────────────────────────────────────────────────┐
│                     API Gateway (未来)                        │
└──────────────────────────────────────────────────────────────┘
           │              │            │            │
           ▼              ▼            ▼            ▼
    ┌──────────┐    ┌──────────┐  ┌──────────┐  ┌──────────┐
    │  user    │    │ inventory│  │   sales  │  │production│
    │  :50051  │    │  :50052  │  │  :50053  │  │  :50054  │
    └──────────┘    └──────────┘  └──────────┘  └──────────┘
           │              │            │            │
           ▼              ▼            ▼            ▼
    ┌──────────┐    ┌──────────┐
    │ process  │    │  notif   │  ← P3-1 关键路径 demo
    │  :50055  │    │  :50056  │
    └──────────┘    └──────────┘
           │              │
           ▼              ▼
    ┌──────────────────────────────────┐
    │  PostgreSQL (共享实例，schema 隔离)│
    │  user_db / inventory_db / ...    │
    └──────────────────────────────────┘
```

## 九、故障排查

| 现象 | 原因 | 解决 |
|------|------|------|
| 启动报 "数据库连接失败" | DATABASE_URL 错误 | 检查环境变量 + PostgreSQL 状态 |
| gRPC 客户端连不上 | 端口被占用 | `lsof -i :50056` + 改 GRPC_PORT |
| 编译报 "protoc not found" | 缺 protobuf-compiler | `apt install protobuf-compiler` |
| tenant_id 无效错误 | 客户端传 0 或负数 | 传 > 0 的有效值 |
| 沙箱 OOM 编译失败 | 内存不足 | 改用 `cargo check --lib` 验证，或 CI 编译 |
| Docker Compose 启动失败 | 端口冲突 | 检查 8080/50056/5432 是否被占用 |

## 十、CI 验证

主项目 CI 流程不变：
- `cd backend && cargo check --lib`
- `cd frontend && pnpm build`（如有前端变更）

微服务 CI 流程（新增）：
- `cd microservices/notifications && cargo check --lib`
- Docker Compose：`docker-compose config` 验证语法

CI 工作流：`.github/workflows/ci-cd.yml` 保持不变（向后兼容）。

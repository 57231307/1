# P3-1 微服务拆分设计 Spec

> **设计日期**：2026-06-17
> **任务编号**：P3 / P3-1
> **关联**：P0/P1/P2 已完成（92-95/100）
> **设计基线**：test @ b457aa4（含 P2-4 AI 深化）

---

## 一、目标与背景

### 1.1 业务目标

将单体型 ERP 后端（`backend/`）按业务领域拆分为独立部署的微服务，提升：
- **可维护性**：单一代码仓库体积大、编译慢（5.8GB 沙箱 OOM 频发）
- **可扩展性**：不同模块可独立伸缩（如通知服务高频，工艺服务低频）
- **技术异构**：不同模块可选用最适合的技术栈
- **故障隔离**：单个微服务崩溃不影响主系统

### 1.2 技术目标

- 提炼出 1 个**可独立运行**的微服务 demo（notifications 微服务）
- 1 套完整的微服务架构设计 spec（gRPC + Docker Compose + 服务发现）
- 1 套完整的实施 plan（剩余 5 个微服务的拆分顺序）
- 不破坏现有 P0/P1/P2 已合入的功能

### 1.3 范围

**包含**：
1. 完整微服务拆分设计 spec（本文件）
2. 完整实施 plan（`docs/superpowers/plans/2026-06-17-p3-1-microservice.md`）
3. 关键路径 demo：notifications 微服务
   - 独立 `microservices/notifications/` 目录
   - 独立 `Cargo.toml`（独立编译）
   - 独立 migration（notifications 业务表）
   - gRPC 接口定义（protobuf）
   - 1 个端点（`send_notification`）
   - 1 个集成测试
   - Docker Compose 多服务启动
4. 用户手册：微服务架构 + 启动说明

**不包含**（P4+ 后续阶段）：
- 真正拆分所有 5+ 个微服务（用户中心 / 库存 / 销售 / 生产 / 工艺 / 通知）
- Kubernetes 部署
- 服务网格（Istio / Linkerd）
- 分布式追踪（Jaeger / Zipkin）
- 熔断限流（Sentinel / Hystrix）
- API 网关（Kong / APISIX）

---

## 二、决策记录（Q1-Q7 + 矛盾解决）

### 2.1 7 个澄清问题

| 编号 | 问题 | 决策 |
|------|------|------|
| Q1 | 微服务拆分粒度 | 6 个微服务（用户 / 库存 / 销售 / 生产 / 工艺 / 通知） |
| Q2 | 通信协议 | gRPC（内部）+ REST（外部 API Gateway 转换） |
| Q3 | 服务发现 | Docker Compose 内置 DNS（demo 阶段） |
| Q4 | 数据隔离 | 每服务独立数据库（schema 隔离 + 未来物理隔离） |
| Q5 | 关键路径 demo | 提取 notifications 微服务（业务独立、无强依赖） |
| Q6 | CI 影响 | 微服务独立 `cargo check` + 主项目保留 `cargo check --lib` |
| Q7 | 是否合到 main | 不合到 main（仅合到 test） |

### 2.2 矛盾解决

**矛盾 1**：微服务拆分需要重构 80% 现有代码 vs 沙箱内存限制无法完整测试
- **决策**：仅提取 1 个 demo 微服务（notifications），主项目保留通知模块代码，新增微服务以"独立项目"形式存在
- **理由**：保留向后兼容；演示微服务拆分架构；不破坏 P0/P1/P2 已合入功能

**矛盾 2**：gRPC 配置 vs TypeScript 客户端复杂度
- **决策**：gRPC 接口定义 + 暂时仅 Rust 客户端（TypeScript 客户端后续 P4 引入）
- **理由**：演示拆分架构优先，客户端生成后续阶段

---

## 三、架构设计

### 3.1 整体架构图

```
┌──────────────────────────────────────────────────────────────┐
│                     API Gateway (axum)                       │
│  - 路由分发                                                  │
│  - 认证授权（JWT）                                           │
│  - 限流熔断                                                  │
│  - REST → gRPC 转换                                          │
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

### 3.2 技术选型

| 维度 | 选型 | 理由 |
|------|------|------|
| 微服务框架 | tonic（gRPC） | 异步、性能、生态成熟 |
| 服务发现 | Docker Compose DNS | demo 阶段足够 |
| 数据库 | PostgreSQL 15 | 现有项目使用 |
| 序列化 | protobuf | gRPC 标准 |
| 容器化 | Docker + docker-compose | 简单、标准化 |
| 监控 | 结构化日志（tracing） | 后续接入 OpenTelemetry |

### 3.3 6 个微服务职责

| 微服务 | 端口 | 数据库 schema | 业务职责 |
|--------|------|---------------|----------|
| user | 50051 | user_db | 认证、用户、角色、权限、租户 |
| inventory | 50052 | inventory_db | 库存、出入库、SKU、仓库 |
| sales | 50053 | sales_db | 客户、订单、报价单、合同 |
| production | 50054 | production_db | 工单、BOM、生产计划 |
| process | 50055 | process_db | 工艺配方、配方优化 |
| notifications | 50056 | notifications_db | 通知、日志、消息推送 |

### 3.4 关键路径：notifications 微服务设计

**目录结构**：
```
microservices/
└── notifications/
    ├── Cargo.toml
    ├── build.rs               # 编译 proto
    ├── proto/
    │   └── notification.proto
    ├── src/
    │   ├── main.rs
    │   ├── service.rs         # gRPC service 实现
    │   ├── repository.rs      # 数据库访问
    │   └── model.rs           # 数据模型
    ├── migrations/
    │   └── 001_init.sql
    ├── tests/
    │   └── integration_test.rs
    ├── Dockerfile
    └── docker-compose.yml     # 多服务启动
```

**数据库表（notifications 独立）**：
```sql
CREATE TABLE notification_messages (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    category VARCHAR(50) NOT NULL,  -- 订单/库存/生产/系统
    priority SMALLINT NOT NULL DEFAULT 5,  -- 1-10
    is_read BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_notif_tenant_user ON notification_messages(tenant_id, user_id, created_at DESC);
```

**gRPC 接口定义（notification.proto）**：
```proto
syntax = "proto3";
package notifications;

service NotificationService {
  // 发送单条通知
  rpc SendNotification(SendNotificationRequest) returns (SendNotificationResponse);
  // 批量发送
  rpc BatchSend(BatchSendRequest) returns (BatchSendResponse);
  // 列出用户通知
  rpc ListUserNotifications(ListRequest) returns (ListResponse);
  // 标记已读
  rpc MarkAsRead(MarkAsReadRequest) returns (MarkAsReadResponse);
}

message SendNotificationRequest {
  int64 tenant_id = 1;
  int64 user_id = 2;
  string title = 3;
  string content = 4;
  string category = 5;
  int32 priority = 6;
}

message SendNotificationResponse {
  int64 id = 1;
  string status = 2;
}
```

### 3.5 现有主项目兼容

**核心原则**：P3-1 **不破坏** 现有 P0/P1/P2 功能。
- 主项目 `backend/` 保留所有通知逻辑（routes/services/handlers）
- 微服务 `microservices/notifications/` 作为**独立项目**新增
- 主项目 `backend/src/services/notification_service.rs` 保留 HTTP REST 端点
- 微服务仅作为**参考实现** + **未来迁移目标**
- Docker Compose 提供**多服务并行**启动（主项目 + 微服务 demo）

---

## 四、数据迁移策略

### 4.1 渐进式迁移

**Phase 1（P3-1 当前）**：双写（dual-write）
- 主项目写主库
- 主项目**可选**写微服务（feature flag 控制）
- 微服务**仅**接收写入，不回读

**Phase 2（未来 P4+）**：微服务为主
- 微服务是真实写入点
- 主项目通过 gRPC 客户端调用微服务
- 数据一致性通过事务消息保证

**Phase 3（最终）**：主项目下线相关模块
- 主项目通知模块转为薄壳
- 数据完全在微服务

### 4.2 数据一致性

- 强一致：同一服务内事务保证
- 最终一致：跨服务通过事务消息（outbox pattern）
- 沙箱限制：P3-1 demo 仅展示架构，不做实际跨服务事务

---

## 五、CI 验证策略

### 5.1 编译验证

- 主项目：`cd backend && cargo check --lib`（保留现有 P2-3 配置）
- 微服务：`cd microservices/notifications && cargo check --lib`
- Docker Compose：`docker-compose config`（语法验证）
- 沙箱 OOM 限制下，仅验证编译，不跑 `cargo test`

### 5.2 测试验证

- 微服务集成测试 1 个：gRPC 客户端发送 → 服务端接收 → 数据库落地
- 测试用本地 SQLite 或 in-memory（避开 PostgreSQL 依赖）

---

## 六、用户验收标准

| 编号 | 验收项 | 验证方法 |
|------|--------|----------|
| AC-1 | spec + plan 完整 | 文档存在 + 含本文件全部章节 |
| AC-2 | notifications 微服务独立编译 | `cargo check --lib` 通过 |
| AC-3 | gRPC proto 定义完整 | 4 个 RPC + 7 个 message |
| AC-4 | 1 个端点 + 1 个测试 | `send_notification` + integration test |
| AC-5 | Docker Compose 多服务启动 | 2 个 service 启动成功（主项目 + notifications） |
| AC-6 | 不破坏 P0/P1/P2 | 主项目 `cargo check --lib` 通过 |
| AC-7 | 用户手册完整 | 含启动说明 + 架构图 + 故障排查 |

---

## 七、风险与回滚

### 7.1 风险

| 风险 | 等级 | 缓解 |
|------|------|------|
| 沙箱 OOM 无法编译微服务 | 高 | 仅 `cargo check --lib`，不跑 `cargo build --release` |
| gRPC 依赖编译慢 | 中 | 使用预编译 `tonic` 依赖（已稳定） |
| Docker Compose 不可用 | 中 | 提供 `docker-compose.yml` 文档 + 手动启动说明 |
| 主项目兼容性破坏 | 低 | 主项目代码不修改，仅新增 `microservices/` |

### 7.2 回滚

- 微服务 demo 是新增目录，删除 `microservices/` 即可完全回滚
- 不修改 `backend/` 任何文件
- 不修改 `frontend/` 任何文件

---

## 八、关联

- Plan：`docs/superpowers/plans/2026-06-17-p3-1-microservice.md`
- 用户手册：`docs/2026-06-17-p3-1-microservice-user-manual.md`
- API 文档：`docs/2026-06-17-p3-1-microservice-api.md`
- CHANGELOG：`CHANGELOG.md`
- MEMORY：`MEMORY.md`

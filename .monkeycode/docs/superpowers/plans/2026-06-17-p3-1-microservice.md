# P3-1 微服务拆分实施 Plan

> **实施日期**：2026-06-17
> **任务编号**：P3 / P3-1
> **关联**：Spec `docs/superpowers/specs/2026-06-17-p3-1-microservice.md`
> **基线**：test @ b457aa4

---

## 一、目标拆解

P3-1 任务拆解为 3 个子任务，串行执行：

| 子任务 | 内容 | 预期产出 |
|--------|------|----------|
| ST-1 | 写完整 spec + plan | 2 份文档 |
| ST-2 | 实现 notifications 微服务 demo | 8 个核心文件 |
| ST-3 | Docker Compose + 用户手册 + 文档 | 3 份文档 |

---

## 二、ST-1 写 spec + plan

### 2.1 spec 文档结构（已完成）

详见 `docs/superpowers/specs/2026-06-17-p3-1-microservice.md`：
- 目标与背景
- 决策记录（7 个 Q + 矛盾解决）
- 架构设计（架构图、技术选型、6 微服务职责、关键路径）
- 数据迁移策略
- CI 验证策略
- 用户验收标准
- 风险与回滚

### 2.2 plan 文档结构（本文件）

---

## 三、ST-2 notifications 微服务 demo

### 3.1 文件清单

```
microservices/notifications/
├── Cargo.toml                       # 独立 Rust 项目
├── build.rs                         # 编译 proto
├── proto/
│   └── notification.proto           # gRPC 接口定义
├── src/
│   ├── main.rs                      # 启动入口
│   ├── service.rs                   # gRPC service 实现
│   ├── repository.rs                # 数据库访问
│   └── model.rs                     # 数据模型
├── migrations/
│   └── 001_init.sql                 # 表结构
├── tests/
│   └── integration_test.rs          # 集成测试
├── Dockerfile                       # 容器化
└── README.md                        # 项目说明
```

### 3.2 关键实现要点

#### 3.2.1 `Cargo.toml`（独立项目）

```toml
[package]
name = "notifications-service"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "notifications-service"
path = "src/main.rs"

[dependencies]
tonic = "0.10"
prost = "0.12"
tokio = { version = "1.35", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
thiserror = "1"

[build-dependencies]
tonic-build = "0.10"

[dev-dependencies]
tokio = { version = "1.35", features = ["full", "test-util"] }
```

#### 3.2.2 `proto/notification.proto`

```proto
syntax = "proto3";
package notifications;

service NotificationService {
  rpc SendNotification(SendNotificationRequest) returns (SendNotificationResponse);
  rpc BatchSend(BatchSendRequest) returns (BatchSendResponse);
  rpc ListUserNotifications(ListRequest) returns (ListResponse);
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

message BatchSendRequest {
  repeated SendNotificationRequest items = 1;
}

message BatchSendResponse {
  int32 count = 1;
  string status = 2;
}

message ListRequest {
  int64 tenant_id = 1;
  int64 user_id = 2;
  int32 limit = 3;
  int32 offset = 4;
}

message NotificationItem {
  int64 id = 1;
  int64 tenant_id = 2;
  int64 user_id = 3;
  string title = 4;
  string content = 5;
  string category = 6;
  int32 priority = 7;
  bool is_read = 8;
  string created_at = 9;
}

message ListResponse {
  repeated NotificationItem items = 1;
  int32 total = 2;
}

message MarkAsReadRequest {
  int64 id = 1;
  int64 tenant_id = 2;
}

message MarkAsReadResponse {
  bool success = 1;
}
```

#### 3.2.3 `src/main.rs`（启动入口）

- 加载环境变量（`DATABASE_URL`、`GRPC_PORT`）
- 初始化 tracing
- 连接数据库
- 启动 tonic gRPC server
- 优雅关闭

#### 3.2.4 `src/service.rs`（gRPC 实现）

- 实现 `NotificationService` trait
- 4 个 RPC 方法（`send_notification` / `batch_send` / `list_user_notifications` / `mark_as_read`）
- 多租户隔离：所有方法接受 `tenant_id` 并在 SQL 中强制过滤
- 错误处理：自定义 `Status::internal` / `Status::invalid_argument`

#### 3.2.5 `src/repository.rs`（数据库访问）

- 使用 sqlx（与主项目一致技术栈）
- `insert_notification` / `list_by_user` / `mark_as_read` / `batch_insert`
- 所有 SQL 强制 `WHERE tenant_id = $1`

#### 3.2.6 `tests/integration_test.rs`（集成测试）

- 启动 in-memory service（或 testcontainer）
- gRPC 客户端调用 `send_notification`
- 验证数据库中存在该通知
- 验证 `tenant_id` 隔离正确

### 3.3 沙箱约束处理

由于沙箱 OOM 限制：
- **不**跑 `cargo build --release`
- **不**跑 `cargo test`（使用大量 tokio 任务）
- **仅**跑 `cargo check --lib` 验证编译
- CI 中跑完整测试（GitHub Actions runner 内存充足）

### 3.4 验证清单

- [ ] `cargo check --lib` 在 `microservices/notifications/` 通过
- [ ] proto 编译生成 Rust 代码（`target/debug/build/.../notification.rs`）
- [ ] 4 个 RPC 方法签名正确
- [ ] 多租户隔离 `WHERE tenant_id = $1` 在所有 SQL 中存在
- [ ] `Dockerfile` 语法正确（`docker build --dry-run` 或 `docker-compose config`）

---

## 四、ST-3 Docker Compose + 文档

### 4.1 Docker Compose 编排

#### 4.1.1 `microservices/docker-compose.yml`

```yaml
version: '3.8'

services:
  # 现有主项目
  erp-backend:
    build: ../backend
    container_name: erp-backend
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://erp:erp@postgres:5432/erp
    depends_on:
      - postgres

  # P3-1 关键路径 demo：notifications 微服务
  notifications-service:
    build: ./notifications
    container_name: notifications-service
    ports:
      - "50056:50056"
    environment:
      - DATABASE_URL=postgres://erp:erp@postgres:5432/notifications_db
      - GRPC_PORT=50056
    depends_on:
      - postgres

  # 共享数据库
  postgres:
    image: postgres:15
    container_name: erp-postgres
    ports:
      - "5432:5432"
    environment:
      - POSTGRES_USER=erp
      - POSTGRES_PASSWORD=erp
      - POSTGRES_MULTIPLE_DATABASES=erp,notifications_db
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

#### 4.1.2 `microservices/notifications/Dockerfile`

```dockerfile
FROM rust:1.94-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY src ./src
COPY build.rs ./
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/notifications-service /usr/local/bin/
COPY migrations ./migrations
EXPOSE 50056
CMD ["notifications-service"]
```

### 4.2 用户手册

#### 4.2.1 `docs/2026-06-17-p3-1-microservice-user-manual.md`

章节：
- 一、什么是微服务拆分
- 二、6 个微服务职责
- 三、notifications 微服务本地启动
- 四、Docker Compose 一键启动
- 五、gRPC 客户端调用示例（grpcurl）
- 六、与主项目关系
- 七、故障排查

### 4.3 API 文档

#### 4.3.1 `docs/2026-06-17-p3-1-microservice-api.md`

- gRPC 接口清单
- 4 个 RPC 方法详细说明
- 7 个 message 字段说明
- 多租户隔离保证

### 4.4 CHANGELOG + MEMORY 更新

#### 4.4.1 `CHANGELOG.md` 新增条目

```markdown
## P3-1 (2026-06-17)

### 微服务拆分

- 完成微服务拆分设计 spec + 实施 plan
- 提炼 1 个微服务 demo：notifications（独立 gRPC 服务）
- 4 个 RPC 端点（send_notification / batch_send / list / mark_as_read）
- Docker Compose 多服务编排（主项目 + notifications）
- 7 个 message + 1 个集成测试
- 主项目代码 0 改动（向后兼容）
```

#### 4.4.2 `MEMORY.md` 新增关键事实

- 6 个微服务端口分配（50051-50056）
- notifications 微服务启动命令
- Docker Compose 启动入口
- 多租户隔离 SQL 模式

---

## 五、验收与合并

### 5.1 验收清单

| 编号 | 验收项 | 验证 |
|------|--------|------|
| AC-1 | spec + plan 完整 | 文件存在 + 章节齐全 |
| AC-2 | notifications 微服务独立编译 | `cargo check --lib` 通过 |
| AC-3 | gRPC proto 定义完整 | 4 RPC + 7 message |
| AC-4 | 1 端点 + 1 测试 | `send_notification` + integration test |
| AC-5 | Docker Compose 多服务 | 2 service 编排 |
| AC-6 | 主项目未破坏 | `cd backend && cargo check --lib` 通过 |
| AC-7 | 用户手册完整 | 启动 + 架构 + 故障排查 |

### 5.2 合并流程

1. commit：`docs(spec): P3-1 微服务拆分设计 spec`
2. commit：`feat(P3-1): notifications 微服务 demo`
3. push：当前分支 `trae/solo-agent-P3-1-microservice`
4. PR：创建 PR #142（base: test）
5. merge：合到 test
6. 切回 test + pull + 删除本地分支

### 5.3 失败处理

- 若 `cargo check --lib` 失败：修复依赖或 API 调用
- 若 Docker Compose 语法错误：检查 YAML 缩进
- 若 CI 失败：本地复现 + 修复 + 重新 push

---

## 六、风险与回滚

| 风险 | 等级 | 缓解 |
|------|------|------|
| 沙箱 OOM | 高 | 仅 `cargo check --lib`，不跑 `cargo build --release` |
| 主项目兼容性 | 低 | 主项目 0 改动 |
| proto 编译错误 | 中 | 使用 `tonic-build` 0.10 + `prost` 0.12 稳定组合 |
| Docker 不可用 | 低 | 提供手动启动说明 |

回滚：删除 `microservices/` 目录即可，不影响主项目。

---

## 七、关联

- Spec：`docs/superpowers/specs/2026-06-17-p3-1-microservice.md`
- 用户手册：`docs/2026-06-17-p3-1-microservice-user-manual.md`
- API 文档：`docs/2026-06-17-p3-1-microservice-api.md`

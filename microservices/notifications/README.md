# notifications-service

> **P3-1 微服务拆分关键路径 demo**
> notifications 独立 gRPC 微服务
> 端口：50056

## 一、项目概述

这是 P3-1 微服务拆分任务的**关键路径 demo**。

从单体型 ERP 后端（`backend/`）提取 1 个独立微服务，验证微服务拆分架构。

## 二、目录结构

```
notifications/
├── Cargo.toml              # 独立 Rust 项目
├── build.rs                # 编译 proto
├── proto/
│   └── notification.proto  # gRPC 接口定义
├── src/
│   ├── main.rs             # 启动入口
│   ├── service.rs          # gRPC service 实现
│   ├── repository.rs       # 数据库访问
│   └── model.rs            # 数据模型
├── migrations/
│   └── 001_init.sql        # 表结构
├── tests/
│   └── integration_test.rs # 集成测试
├── Dockerfile
└── README.md               # 本文件
```

## 三、本地启动

### 3.1 前置依赖

- Rust 1.94+
- PostgreSQL 15+
- protobuf-compiler（仅编译时需要）

### 3.2 启动数据库

```bash
# 创建数据库
createdb notifications_db

# 执行 migration
psql notifications_db < migrations/001_init.sql
```

### 3.3 配置环境变量

```bash
export DATABASE_URL=postgres://erp:erp@localhost:5432/notifications_db
export GRPC_PORT=50056
export RUST_LOG=info
```

### 3.4 编译 + 启动

```bash
cargo build --release
./target/release/notifications-service
```

启动成功日志：
```
启动 notifications 微服务（端口 50056）
数据库连接成功
gRPC server 监听 0.0.0.0:50056
```

## 四、Docker 启动

```bash
docker build -t notifications-service .
docker run -p 50056:50056 \
  -e DATABASE_URL=postgres://erp:erp@host.docker.internal:5432/notifications_db \
  notifications-service
```

## 五、gRPC 客户端调用示例

### 5.1 grpcurl 调用

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

### 5.2 Rust 客户端示例

```rust
use notifications_service::proto::{
    notification_service_client::NotificationServiceClient,
    SendNotificationRequest,
};

let mut client = NotificationServiceClient::connect("http://localhost:50056").await?;
let request = tonic::Request::new(SendNotificationRequest {
    tenant_id: 1,
    user_id: 100,
    title: "测试".to_string(),
    content: "内容".to_string(),
    category: "system".to_string(),
    priority: 5,
});
let response = client.send_notification(request).await?;
println!("发送结果：{:?}", response);
```

## 六、gRPC 接口

| RPC | 描述 |
|-----|------|
| SendNotification | 发送单条通知 |
| BatchSend | 批量发送 |
| ListUserNotifications | 列出用户通知（分页） |
| MarkAsRead | 标记已读 |

详见 `proto/notification.proto`。

## 七、多租户隔离

- 所有 RPC 方法接受 `tenant_id` 参数
- 所有 SQL 强制 `WHERE tenant_id = $1`
- 标记已读：`WHERE id = $1 AND tenant_id = $2`（双条件防跨租户）
- 与主项目 `extract_tenant_id` 等价

## 八、与主项目关系

- **不破坏主项目**：主项目 `backend/src/services/notification_service.rs` 保留所有 HTTP REST 端点
- **独立运行**：本服务作为参考实现 + 未来迁移目标
- **Docker Compose 并行**：通过 `microservices/docker-compose.yml` 启动主项目 + 本服务

## 九、测试

```bash
# 沙箱 OOM 限制下仅跑单元测试
cargo test --lib

# CI 环境跑集成测试（需 PostgreSQL）
cargo test -- --ignored
```

## 十、故障排查

| 现象 | 原因 | 解决 |
|------|------|------|
| 启动报 "数据库连接失败" | DATABASE_URL 错误 | 检查环境变量 + PostgreSQL 状态 |
| gRPC 客户端连不上 | 端口被占用 | `lsof -i :50056` + 改 GRPC_PORT |
| 编译报 "protoc not found" | 缺 protobuf-compiler | `apt install protobuf-compiler` |
| tenant_id 无效错误 | 客户端传 0 或负数 | 传 > 0 的有效值 |

## 十一、后续演进

- P4+：API Gateway 自动 REST → gRPC 转换
- P4+：服务发现集成 Consul / etcd
- P4+：Kubernetes 部署
- P4+：服务间链路追踪（OpenTelemetry）

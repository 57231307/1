# microservices/ - 微服务目录

> P3-1 微服务拆分关键路径 demo 集合

## 目录结构

```
microservices/
├── docker-compose.yml             # 多服务编排
├── notifications/                 # P3-1 关键路径 demo
│   ├── Cargo.toml
│   ├── proto/
│   ├── src/
│   ├── migrations/
│   ├── tests/
│   ├── Dockerfile
│   └── README.md
└── README.md                      # 本文件
```

## 启动方式

### 方式 1：Docker Compose（推荐）

```bash
cd microservices
docker-compose up -d
```

启动 3 个 service：
- `postgres` - 共享数据库
- `erp-backend` - 主项目（端口 8080）
- `notifications-service` - notifications 微服务（端口 50056）

### 方式 2：单独启动 notifications

```bash
cd microservices/notifications
cargo build --release
DATABASE_URL=postgres://erp:erp@localhost:5432/notifications_db \
  ./target/release/notifications-service
```

## 验证

```bash
# 主项目健康检查
curl http://localhost:8080/health

# notifications 微服务（gRPC 反射）
grpcurl -plaintext localhost:50056 list
```

## 后续演进

P3-1 仅完成 1 个微服务 demo。后续 P4+ 阶段将：

1. **拆分更多微服务**：user / inventory / sales / production / process
2. **API Gateway**：统一入口（Kong / APISIX）
3. **服务发现**：Consul / etcd
4. **链路追踪**：Jaeger + OpenTelemetry
5. **Kubernetes 部署**

详见 `docs/superpowers/specs/2026-06-17-p3-1-microservice.md`。

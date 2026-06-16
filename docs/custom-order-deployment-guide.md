# 定制订单全流程跟踪 - 部署指南

> **版本**: v1.0
> **时间**: 2026-06-17
> **关联测试版本**: `dist/test-version-P0-3/`

---

## 1. 系统要求

| 组件 | 版本 | 用途 |
|------|------|------|
| PostgreSQL | 16.x | 主数据库 |
| Rust | 1.94+ | 后端编译 |
| Node.js | 20.x | 前端构建 |
| Docker | 24+ | 容器化部署（可选） |

---

## 2. 数据库迁移

### 2.1 新增 5 张表

按顺序执行 5 个 migration：

```bash
cd backend
psql -h <host> -U <user> -d <dbname> -f migrations/20260617000001_create_custom_orders/up.sql
psql -h <host> -U <user> -d <dbname> -f migrations/20260617000002_create_process_nodes/up.sql
psql -h <host> -U <user> -d <dbname> -f migrations/20260617000003_create_process_logs/up.sql
psql -h <host> -U <user> -d <dbname> -f migrations/20260617000004_create_quality_issues/up.sql
psql -h <host> -U <user> -d <dbname> -f migrations/20260617000005_create_after_sales/up.sql
```

### 2.2 自动迁移（使用 sea-orm-migration）

```bash
cd backend
cargo run --bin migration up
```

### 2.3 回滚

```bash
psql -h <host> -U <user> -d <dbname> -f migrations/20260617000005_create_after_sales/down.sql
# 倒序执行 down.sql
```

---

## 3. 后端部署

### 3.1 编译

```bash
cd backend
cargo build --release
```

### 3.2 配置

创建 `config/custom-order.toml`：

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgres://user:password@localhost:5432/bingxi_erp"
max_connections = 32

[cache]
redis_url = "redis://localhost:6379"
ttl_seconds = 3600

[custom_order]
# 5 阶段工艺节点是否自动创建（创建订单时）
auto_create_process_nodes = true
# 质量异常色差 ΔE 警告阈值
color_delta_e_warning_threshold = 5.0
# 售后工单超时（小时）
after_sales_timeout_hours = 72
```

### 3.3 启动

```bash
./target/release/bingxi-erp --config config/custom-order.toml
```

---

## 4. 前端部署

### 4.1 构建

```bash
cd frontend
npm install
npm run build
```

### 4.2 Nginx 配置

```nginx
server {
    listen 80;
    server_name custom-order.bingxi-erp.com;

    root /opt/bingxi-erp/dist;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## 5. Docker 部署（推荐）

参考 `dist/test-version-P0-3/Dockerfile` 和 `docker-compose.yml`。

```bash
cd dist/test-version-P0-3
docker-compose up -d
```

---

## 6. 验证清单

- [ ] 5 张表创建成功
- [ ] 16 个 API 端点可访问
- [ ] 状态机推进测试通过
- [ ] 工艺节点自动生成
- [ ] 质检上报（含色差/色牢度校验）
- [ ] 售后工单 4 类型
- [ ] 多租户隔离验证
- [ ] 前端 4 页面可访问
- [ ] 3 组件渲染正常
- [ ] E2E 测试通过

---

## 7. 监控指标

| 指标 | 含义 | 阈值 |
|------|------|------|
| `custom_order_create_total` | 定制订单创建计数 | - |
| `custom_order_advance_total` | 状态推进计数 | - |
| `custom_order_quality_issue_total` | 异常上报计数 | - |
| `custom_order_after_sales_total` | 售后工单计数 | - |
| `custom_order_state_latency_seconds` | 状态推进延迟 | < 1s |

---

## 8. 故障排除

| 问题 | 解决方案 |
|------|---------|
| 启动报 `column "custom_orders" does not exist` | 检查 migration 是否全部执行 |
| 推进状态返回 409 | 检查当前状态是否允许推进 |
| 上报异常返回 400 | 校验色差/色牢度字段是否填写 |
| 跨租户访问返回 403 | 检查 JWT token 的 tenant_id |

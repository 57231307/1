# 面料多色号定价扩展部署指南

> 冰溪 ERP P0-5 行业功能 - 部署与运维
> 适用版本: P0-5 之后
> 更新日期: 2026-06-18

---

## 1. 部署模式

| 模式 | 适用场景 | 命令 |
|------|----------|------|
| Docker Compose | 测试 / 演示 / 1 台服务器 | `docker-compose up -d` |
| Kubernetes | 生产环境 | `kubectl apply -f k8s/` |
| 手动部署 | 自定义环境 | 见 §4 |

---

## 2. Docker Compose 部署（推荐）

### 2.1 准备

- Docker 20.10+
- Docker Compose 2.0+
- 2GB+ 可用内存
- 10GB+ 可用磁盘

### 2.2 启动

```bash
cd dist/test-version-P0-5

# 1. 复制环境变量
cp .env.example .env

# 2. 启动所有服务
docker-compose up -d

# 3. 等待启动（约 30 秒）
docker-compose logs -f

# 4. 检查状态
docker-compose ps
```

### 2.3 服务组件

| 服务 | 端口 | 说明 |
|------|------|------|
| frontend | 8080 | 前端 Vue 应用（nginx） |
| backend | 8081 | 后端 Rust API |
| postgres | 5432 | PostgreSQL 15 |

### 2.4 访问

- 前端：http://localhost:8080
- 后端 API：http://localhost:8081
- PostgreSQL：localhost:5432

### 2.5 默认账号

- 管理员：`admin` / `admin123`
- 销售员：`sales` / `sales123`

### 2.6 停止

```bash
docker-compose down           # 停止
docker-compose down -v        # 停止并删除数据卷
```

---

## 3. Kubernetes 部署

### 3.1 命名空间

```bash
kubectl create namespace color-price
```

### 3.2 配置 Secret

```bash
kubectl create secret generic db-credentials \
  --from-literal=username=bingxi \
  --from-literal=password=<your-password> \
  -n color-price
```

### 3.3 部署

```bash
kubectl apply -f k8s/postgres.yaml -n color-price
kubectl apply -f k8s/backend.yaml -n color-price
kubectl apply -f k8s/frontend.yaml -n color-price
```

### 3.4 Ingress

```bash
kubectl apply -f k8s/ingress.yaml -n color-price
```

---

## 4. 手动部署

### 4.1 数据库

```bash
# 1. 安装 PostgreSQL 15
sudo apt install postgresql-15

# 2. 创建数据库
sudo -u postgres psql <<EOF
CREATE DATABASE bingxi;
CREATE USER bingxi WITH PASSWORD '<your-password>';
GRANT ALL PRIVILEGES ON DATABASE bingxi TO bingxi;
EOF

# 3. 运行 migration
cd backend
sqlx migrate run
```

### 4.2 后端

```bash
# 1. 安装 Rust 1.94+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install 1.94

# 2. 编译
cd backend
cargo build --release

# 3. 启动
DATABASE_URL=postgres://bingxi:<password>@localhost/bingxi \
RUST_LOG=info \
./target/release/bingxi-backend
```

### 4.3 前端

```bash
# 1. 安装 Node 20+
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# 2. 编译
cd frontend
npm install
npm run build

# 3. 启动（nginx）
sudo cp -r dist/* /var/www/html/
sudo systemctl restart nginx
```

---

## 5. 环境变量

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `DATABASE_URL` | 是 | - | PostgreSQL 连接字符串 |
| `RUST_LOG` | 否 | `info` | 日志级别 |
| `JWT_SECRET` | 是 | - | JWT 签名密钥 |
| `BIND_ADDR` | 否 | `0.0.0.0:8081` | 后端监听地址 |
| `TENANT_MODE` | 否 | `multi` | 多租户模式（multi / single） |
| `PRICE_APPROVAL_THRESHOLD` | 否 | `0.10` | 调价审批阈值（10%） |
| `EXCHANGE_RATE_PROVIDER` | 否 | `default` | 汇率服务提供商 |

---

## 6. 数据库 Migration

P0-5 新增 5 张表 + 1 张扩展：

| Migration | 描述 |
|-----------|------|
| 20260618000001_extend_product_color_prices | 扩展现有表（添加 max_quantity / customer_id / season / is_active / priority / 审批字段） |
| 20260618000002_create_color_price_history | 价格历史表 |
| 20260618000003_create_color_price_tiers | 阶梯价表 |
| 20260618000004_create_customer_color_prices | 客户专属价表 |
| 20260618000005_create_seasonal_price_rules | 季节调价规则表 |

**回滚**：

```bash
sqlx migrate revert
```

---

## 7. 监控

### 7.1 健康检查

```bash
curl http://localhost:8081/health
# {"status":"ok","version":"P0-5"}
```

### 7.2 关键指标

| 指标 | 告警阈值 |
|------|----------|
| CPU 使用率 | > 80% |
| 内存使用率 | > 85% |
| 数据库连接数 | > 80% |
| API P99 延迟 | > 500ms |
| 价格计算 QPS | < 100 |
| 待审批调价数 | > 50 |

### 7.3 日志

```bash
# 后端日志
docker-compose logs -f backend

# 前端日志
docker-compose logs -f frontend

# 数据库日志
docker-compose logs -f postgres
```

---

## 8. 备份与恢复

### 8.1 备份

```bash
# 数据库
docker-compose exec postgres pg_dump -U bingxi bingxi > backup.sql

# 上传至对象存储
aws s3 cp backup.sql s3://bingxi-backups/
```

### 8.2 恢复

```bash
# 停止服务
docker-compose down

# 恢复数据库
cat backup.sql | docker-compose exec -T postgres psql -U bingxi bingxi

# 启动服务
docker-compose up -d
```

---

## 9. 故障排除

### 9.1 后端启动失败

```bash
# 1. 检查数据库连接
docker-compose exec backend psql $DATABASE_URL -c "SELECT 1"

# 2. 检查端口
netstat -tlnp | grep 8081

# 3. 查看详细日志
docker-compose logs backend
```

### 9.2 前端无法访问 API

```bash
# 1. 检查 nginx 配置
docker-compose exec frontend cat /etc/nginx/conf.d/default.conf

# 2. 测试 API
curl http://localhost:8081/health
```

### 9.3 价格计算错误

```bash
# 1. 检查阶梯价配置
curl -H "Authorization: Bearer <token>" \
  http://localhost:8081/api/v1/erp/color-prices/tiers/1

# 2. 检查季节规则
curl -H "Authorization: Bearer <token>" \
  'http://localhost:8081/api/v1/erp/color-prices/seasonal-rules?is_active=true'

# 3. 测试价格计算
curl -H "Authorization: Bearer <token>" \
  'http://localhost:8081/api/v1/erp/color-prices/calculate?product_id=1&color_id=1&quantity=100'
```

### 9.4 调价审批未生效

```bash
# 1. 检查 PENDING 状态
curl -H "Authorization: Bearer <token>" \
  'http://localhost:8081/api/v1/erp/color-prices?approval_status=PENDING'

# 2. 审批
curl -X POST -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"decision":"APPROVED"}' \
  http://localhost:8081/api/v1/erp/color-prices/1/approve
```

---

## 10. 性能优化

### 10.1 数据库索引

P0-5 新增索引：

```sql
CREATE INDEX idx_color_prices_tenant ON product_color_prices(tenant_id);
CREATE INDEX idx_color_prices_customer ON product_color_prices(customer_id);
CREATE INDEX idx_color_prices_season ON product_color_prices(season);
CREATE INDEX idx_color_prices_active ON product_color_prices(is_active);
CREATE INDEX idx_color_prices_approval ON product_color_prices(approval_status);
CREATE INDEX idx_price_history_price ON color_price_history(product_color_price_id);
CREATE INDEX idx_price_history_tenant ON color_price_history(tenant_id);
CREATE INDEX idx_cust_color_price_customer ON customer_color_prices(customer_id);
CREATE INDEX idx_seasonal_tenant_active ON seasonal_price_rules(tenant_id, is_active);
```

### 10.2 缓存策略

- 价格计算结果：Redis 缓存 5 分钟
- 阶梯价：启动时加载到内存
- 季节规则：启动时加载到内存
- 客户专属价：按 customer_id 缓存

### 10.3 并发控制

- 批量调价：行锁（`FOR UPDATE`）
- 价格计算：乐观锁（version 字段）
- 审批：事务 + 唯一约束

---

## 11. 安全建议

1. 修改默认密码（`admin` / `admin123`）
2. 使用 HTTPS（生产环境）
3. 配置防火墙（仅开放 80/443）
4. 启用数据库 SSL
5. 定期备份（每日）
6. 审计日志（保留 90 天）
7. 调价权限分级（销售 / 销售经理 / 总经理）
8. 客户专属价需总经理审批

---

## 12. 联系支持

- GitHub Issues: https://github.com/57231307/1/issues
- 文档：`docs/superpowers/specs/2026-06-16-color-price-extension-design.md`
- Spec：`docs/superpowers/plans/2026-06-16-color-price-extension-plan.md`

# 色卡仓储管理 - 部署指南

> P0-4 色卡仓储管理模块部署指南
> 创建时间: 2026-06-17
> 关联 spec: `docs/superpowers/specs/2026-06-16-color-card-design.md`

---

## 一、部署架构

```
┌──────────────────────────────────────────────┐
│  前端 (Nginx) - 端口 3000                    │
│  - Vue 3.4 + Element Plus                    │
│  - 4 页面 + 3 组件                           │
└──────────────┬───────────────────────────────┘
               │ HTTP
┌──────────────▼───────────────────────────────┐
│  后端 (Rust 1.94 + Axum) - 端口 8080         │
│  - 16 API 端点                               │
│  - 4 service + 13 handler                    │
│  - CIELab 色彩空间转换                       │
└──────────────┬───────────────────────────────┘
               │ SQL
┌──────────────▼───────────────────────────────┐
│  PostgreSQL 15                               │
│  - 3 新表：color_cards / color_card_items /  │
│           color_card_borrow_records          │
└──────────────────────────────────────────────┘
```

---

## 二、部署前置条件

### 2.1 硬件要求
| 角色 | CPU | 内存 | 磁盘 |
|------|------|------|------|
| 后端 | 2 核 | 4GB | 20GB |
| 前端 | 1 核 | 512MB | 5GB |
| 数据库 | 2 核 | 4GB | 50GB |

### 2.2 软件依赖
- Docker 20+
- Docker Compose 1.28+
- PostgreSQL 15+
- Rust 1.94（编译时）
- Node.js 20+（前端构建时）

---

## 三、数据库迁移

### 3.1 自动迁移（推荐）

后端启动时自动执行迁移。确保 `migrations/` 目录包含：
- `20260617000006_create_color_cards/`
- `20260617000007_create_color_card_items/`
- `20260617000008_create_color_card_borrow_records/`

### 3.2 手动迁移

```bash
# 在 backend 目录
cargo run --bin migrate up
```

### 3.3 回滚

```bash
# 回滚所有色卡相关迁移
cargo run --bin migrate down \
  --step 3 \
  --migration-dir migrations
```

### 3.4 表结构说明

#### color_cards（色卡主表）
- 主键：id (BIGSERIAL)
- 唯一约束：card_no
- 索引：tenant_id, status, card_type, season

#### color_card_items（色卡明细）
- 主键：id (BIGSERIAL)
- 外键：color_card_id → color_cards(id)
- 唯一约束：(color_card_id, color_code)
- 索引：color_card_id, color_code, pantone_code, cncs_code, tenant_id, dye_recipe_id, product_color_price_id

#### color_card_borrow_records（借出记录）
- 主键：id (BIGSERIAL)
- 外键：color_card_id → color_cards(id), customer_id → customers(id), borrowed_by → users(id)
- 索引：color_card_id, customer_id, status, tenant_id, borrowed_at, borrowed_by

---

## 四、配置说明

### 4.1 后端环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| DATABASE_URL | PostgreSQL 连接 URL | postgres://bingxi:password@localhost:5432/bingxi_erp |
| JWT_SECRET | JWT 签名密钥 | （必填，32+ 字符）|
| REDIS_URL | Redis 连接 URL | redis://localhost:6379 |
| RUST_LOG | 日志级别 | info |

### 4.2 色卡模块配置（可选）

创建 `config/color-card.toml`：

```toml
[color_card]
default_tenant_id = 1

[color_card.import]
max_items_per_batch = 1000
max_file_size_mb = 10
auto_compute_color_space = true

[color_card.borrow]
max_borrow_days = 30
lost_compensation_requires_approval = true
damaged_compensation_approval_threshold = 1000.0

[color_card.industry]
delta_e_threshold = 3.0
```

---

## 五、Docker Compose 部署（推荐）

### 5.1 启动

```bash
cd dist/test-version-P0-4
cp config/color-card.toml.example config/color-card.toml

# 可选：修改 docker-compose.yml 中的密码
export DB_PASSWORD=your-secure-password
export JWT_SECRET=$(openssl rand -hex 32)

./start.sh
```

### 5.2 验证

```bash
# 检查后端健康
curl http://localhost:8080/health

# 检查前端
curl http://localhost:3000

# 验证色卡端点
curl -X POST http://localhost:8080/api/v1/erp/color-cards \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"card_no":"TEST","card_name":"测试","card_type":"CUSTOM"}'
```

### 5.3 停止

```bash
cd dist/test-version-P0-4
./stop.sh
```

---

## 六、手动部署

### 6.1 启动 PostgreSQL

```bash
docker run -d --name postgres \
  -e POSTGRES_USER=bingxi \
  -e POSTGRES_PASSWORD=bingxi_test \
  -e POSTGRES_DB=bingxi_erp \
  -p 5432:5432 \
  -v pgdata:/var/lib/postgresql/data \
  postgres:15-alpine
```

### 6.2 编译并启动后端

```bash
cd backend
cargo build --release

export DATABASE_URL=postgres://bingxi:bingxi_test@localhost:5432/bingxi_erp
export JWT_SECRET=$(openssl rand -hex 32)
export REDIS_URL=redis://localhost:6379

./target/release/bingxi-erp
```

### 6.3 构建并启动前端

```bash
cd frontend
npm install
npm run build

# 用 nginx 托管 dist/
# 或使用：npx serve dist -p 3000
```

---

## 七、生产环境检查清单

### 7.1 安全
- [ ] 修改默认数据库密码
- [ ] 使用强 JWT 密钥（32+ 字符）
- [ ] 启用 HTTPS
- [ ] 配置防火墙规则
- [ ] 启用数据库连接加密（SSL）

### 7.2 性能
- [ ] 启用 PostgreSQL 慢查询日志
- [ ] 配置数据库连接池（min: 5, max: 50）
- [ ] 启用 Redis 缓存
- [ ] 配置前端 CDN
- [ ] 启用 gzip 压缩

### 7.3 监控
- [ ] 配置 Prometheus 指标抓取
- [ ] 配置 Grafana 仪表板
- [ ] 配置告警规则：
  - 数据库连接失败
  - API 响应时间 > 1s
  - 批量导入失败率 > 10%

### 7.4 备份
- [ ] 数据库每日自动备份
- [ ] 色卡文件（如有）单独备份
- [ ] 备份保留期：30 天

---

## 八、故障排查

### 8.1 数据库连接失败
```
ERROR: connection to server at "localhost", port 5432 failed
```
- 检查 PostgreSQL 是否启动
- 检查 `DATABASE_URL` 是否正确
- 检查防火墙规则

### 8.2 迁移失败
```
ERROR: relation "color_cards" does not exist
```
- 确认 migrations 目录存在 3 个新迁移目录
- 手动执行 `cargo run --bin migrate up`

### 8.3 色号导入失败
```
ERROR: duplicate key value violates unique constraint "uq_color_card_items_card_code"
```
- 同一色卡内色号编码重复，请去重后再导入

### 8.4 借出失败
```
ERROR: 预计归还时间不能超过借出时间 + 30 天
```
- 调整预计归还时间在 30 天内

### 8.5 扫码查询无结果
- 确认 color_code 拼写正确
- 确认色号未被删除
- 检查多租户隔离是否误判

---

## 九、升级与回滚

### 9.1 升级
```bash
# 1. 拉取新代码
git pull origin trae/solo-agent-P0-4-color-card

# 2. 重新构建
docker-compose build

# 3. 重启
docker-compose up -d
```

### 9.2 回滚
```bash
# 1. 停止当前服务
./stop.sh

# 2. 切换到上一个版本
git checkout <previous-commit>

# 3. 回滚数据库（谨慎！）
cargo run --bin migrate down --step 3

# 4. 重新启动
./start.sh
```

---

## 十、技术支持

- **文档**：`docs/superpowers/specs/2026-06-16-color-card-design.md`
- **API 文档**：`docs/color-card-api.md`
- **用户手册**：`docs/color-card-user-manual.md`
- **测试场景**：`dist/test-version-P0-4/test-scenarios.md`

---

> 文档版本：v1.0 | 2026-06-17 | P0-4 色卡仓储管理部署指南

# 冰溪 ERP P0-2 主备隔离 TEST 测试版本

> **版本**: v1.0
> **更新日期**: 2026-06-16
> **目标**: 提供 P0-2 主备隔离模块的可测试部署

---

## 快速开始

```bash
# 1. 复制配置模板
cp config/failover.toml.example config/failover.toml

# 2. 设置环境变量（或编辑 .env）
export POSTGRES_PRIMARY_PASSWORD=your_password
export POSTGRES_BACKUP_PASSWORD=your_password
export DATABASE_URL_PRIMARY=postgresql://user:password@postgres-primary:5432/bingxi
export DATABASE_URL_BACKUP=postgresql://user:password@postgres-backup:5432/bingxi
export REDIS_URL=redis://redis-primary:6379
export JWT_SECRET=$(openssl rand -hex 32)

# 3. 启动
./start.sh

# 4. 验证
curl http://localhost:8080/api/v1/erp/admin/failover/status
```

---

## 服务清单

| 服务 | 端口 | 说明 |
|------|------|------|
| app | 8080 | 冰溪 ERP 后端 |
| postgres-primary | 5432 | PostgreSQL 主库 |
| postgres-backup | 5432 | PostgreSQL 备库 |
| redis-primary | 6379 | Redis 主缓存 |

---

## API 端点

- `GET /api/v1/erp/admin/failover/status` — 主备实时状态
- `GET /api/v1/erp/admin/failover/metrics` — Prometheus 指标
- `POST /api/v1/erp/admin/failover/test/switch` — 手动触发切换
- `GET /api/v1/erp/admin/failover/health` — 健康检查

## 监控

- 访问 Grafana: `http://localhost:3000`（如果启用）
- 访问 Prometheus: `http://localhost:9090`（如果启用）

## 故障注入

参见 `chaos-test-scenarios.md`

---

## 目录结构

```
test-version-P0-2/
├── Dockerfile                    # 应用镜像（多阶段构建）
├── docker-compose.yml            # 服务编排
├── start.sh                      # 一键启动脚本
├── stop.sh                       # 停止脚本
├── README.md                     # 本文档
├── chaos-test-scenarios.md       # 故障注入测试
├── monitoring-dashboard.json     # Grafana dashboard
├── config/
│   └── failover.toml.example     # 主备配置示例
└── .env.example                  # 环境变量示例
```

---

## 详细文档

- [部署指南](../../docs/failover-deployment-guide.md)
- [故障注入测试场景](../../docs/chaos-test-scenarios.md)
- [设计 spec](../../docs/superpowers/specs/2026-06-16-failover-isolation-design.md)
- [实施 plan](../../docs/superpowers/plans/2026-06-16-failover-isolation-plan.md)
- [设计报告](../../docs/superpowers/reports/2026-06-16-failover-design.md)

---

**版本**: v1.0
**最后更新**: 2026-06-16

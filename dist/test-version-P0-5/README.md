# P0-5 面料多色号定价扩展 - TEST 测试版本

> 冰溪 ERP P0-5 行业功能 - 一键启动的 Docker 测试版本
> 创建时间: 2026-06-18
> 关联文档: docs/superpowers/specs/2026-06-16-color-price-extension-design.md

## 快速启动

```bash
# 1. 启动所有服务
./start.sh

# 2. 访问
# 前端: http://localhost:8080
# 后端: http://localhost:8081
# 数据库: localhost:5432

# 3. 停止
./stop.sh
```

## 包含内容

- **后端**：完整 Rust 1.94 + Axum + SeaORM
- **前端**：Vue 3 + Element Plus + ECharts
- **数据库**：PostgreSQL 15
- **5 张表**：1 扩展 + 4 新建（color_price_history / color_price_tiers / customer_color_prices / seasonal_price_rules）
- **16 API 端点**
- **13 handler + 5 service + 1 价格计算引擎**
- **3 前端页面 + 2 组件**
- **5 集成测试 + 5 E2E**

## 默认账号

| 账号 | 密码 | 角色 |
|------|------|------|
| admin | admin123 | 管理员 |
| sales | sales123 | 销售员 |

## 端口

| 服务 | 端口 |
|------|------|
| 前端 | 8080 |
| 后端 | 8081 |
| PostgreSQL | 5432 |

## 测试场景

参见 `test-scenarios.md`（10 个测试场景）

## 文档

- 用户手册：`docs/color-price-user-manual.md`
- API 文档：`docs/color-price-api.md`
- 部署指南：`docs/color-price-deployment-guide.md`
- Spec：`docs/superpowers/specs/2026-06-16-color-price-extension-design.md`
- Plan：`docs/superpowers/plans/2026-06-16-color-price-extension-plan.md`

## 颜色价格功能核心特性

- ✅ 批量调价（百分比 / 固定金额 / 阶梯价 3 种模式）
- ✅ 调价审批（>10% 自动转审批）
- ✅ 价格历史（完整审计与回溯）
- ✅ 阶梯定价（4 档 + 客户等级叠加）
- ✅ 季节性调价（SS / AW / HOLIDAY）
- ✅ 客户专属价（最高优先级）
- ✅ VIP 95 折
- ✅ 价格计算引擎（统一计算最优价格）
- ✅ 多租户隔离
- ✅ 多币种支持（CNY / USD / EUR）

## 故障排除

```bash
# 查看日志
docker-compose logs -f

# 重新构建
docker-compose build --no-cache

# 完全重置
docker-compose down -v
./start.sh
```

## 联系

- GitHub: https://github.com/57231307/1
- Issue: https://github.com/57231307/1/issues

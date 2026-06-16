# 冰溪 ERP 定制订单全流程跟踪 P0-3 测试版本

> **版本**: v1.0
> **时间**: 2026-06-17
> **关联模块**: P0-3 定制订单全流程跟踪（5 张表 + 16 API + 4 页面 + 3 组件 + 集成测试）

## 1. 快速开始

### 1.1 启动

```bash
# 进入测试版本目录
cd dist/test-version-P0-3

# 启动所有服务（PostgreSQL + Redis + 后端 + 前端）
docker-compose up -d

# 查看启动日志
docker-compose logs -f backend
```

### 1.2 验证

```bash
# 健康检查
curl http://localhost:8080/health

# 测试定制订单 API
curl -X POST http://localhost:8080/api/v1/erp/custom-orders \
  -H "Authorization: Bearer <test-jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": 1,
    "product_id": 1,
    "spec": "100% 棉 200g/m²",
    "quantity": 100.00,
    "unit": "m"
  }'
```

## 2. 测试场景

详见 [test-scenarios.md](./test-scenarios.md)。

## 3. 配置说明

复制 `config/custom-order.toml.example` 为 `config/custom-order.toml` 并修改实际值。

## 4. 文档

- 用户手册：[docs/custom-order-user-manual.md](../../docs/custom-order-user-manual.md)
- API 文档：[docs/custom-order-api.md](../../docs/custom-order-api.md)
- 部署指南：[docs/custom-order-deployment-guide.md](../../docs/custom-order-deployment-guide.md)
- 设计 spec：[docs/superpowers/specs/2026-06-16-custom-order-design.md](../../docs/superpowers/specs/2026-06-16-custom-order-design.md)

## 5. 故障排除

### 5.1 启动失败

```bash
# 查看详细日志
docker-compose logs --tail=100 backend
```

### 5.2 数据库 migration 失败

```bash
# 手动执行 migration
docker-compose exec postgres psql -U bingxi -d bingxi_erp -f /docker-entrypoint-initdb.d/20260617000001_create_custom_orders/up.sql
```

### 5.3 端口冲突

修改 `docker-compose.yml` 中的端口映射，例如 `"8081:8080"`。

## 6. 验收清单

- [ ] Docker 镜像构建成功
- [ ] PostgreSQL + Redis 启动正常
- [ ] 5 张表 migration 全部成功
- [ ] 后端 16 API 端点可访问
- [ ] 工艺流程测试用例通过
- [ ] 行业规则校验测试通过
- [ ] 多租户隔离验证通过
- [ ] 性能测试（创建订单 < 100ms）

## 7. 联系方式

- 技术支持：tech@bingxi-erp.com

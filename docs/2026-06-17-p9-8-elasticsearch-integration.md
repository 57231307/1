# P9-8 Elasticsearch 集成报告

> 创建日期：2026-06-17
> 范围：业务搜索能力（PG 主数据 + ES 搜索副本）
> 3 索引 + 3 搜索 API + 数据同步

## 一、目标

P9-8 任务为冰溪 ERP 后端引入 **Elasticsearch（ES）** 作为搜索副本，实现：

1. **3 个核心索引**（按业务域拆分）
2. **3 个搜索 API**（HTTP GET）
3. **数据同步机制**（业务写入 → ES）
4. **PG + ES 双轨制**（PG 事务、ES 搜索）

## 二、索引设计

| 索引 | 业务域 | 关键字段 | 分词器 |
|------|--------|----------|--------|
| `sales_orders` | 销售订单 | order_no / customer_name / status / total_amount | ik_smart |
| `customers` | 客户 | code / name / phone / email / tier | ik_smart |
| `products` | 产品 | code / name / color_no / pantone_code / price | ik_smart |

### 字段映射（mapping）

**sales_orders**：
```json
{
  "order_no": "keyword",      // 精确匹配
  "customer_id": "integer",
  "customer_name": "text/ik_smart",  // 中文分词
  "total_amount": "double",
  "status": "keyword",
  "created_at": "date",
  "tenant_id": "keyword"      // 多租户隔离
}
```

**customers**：
```json
{
  "id": "integer",
  "code": "keyword",
  "name": "text/ik_smart",
  "phone": "keyword",
  "email": "keyword",
  "tier": "keyword",
  "tenant_id": "keyword"
}
```

**products**：
```json
{
  "id": "integer",
  "code": "keyword",
  "name": "text/ik_smart",
  "color_no": "keyword",
  "pantone_code": "keyword",
  "price": "double",
  "tenant_id": "keyword"
}
```

## 三、模块设计

### 1. `backend/src/search/elastic.rs` — ES 核心

提供：
- `indices` — 3 个索引常量
- `DocType` — 文档类型枚举
- 文档结构：`SalesOrderDoc` / `CustomerDoc` / `ProductDoc`
- 查询结构：`SearchQuery`（keyword/filters/pagination/highlight）
- 结果结构：`SearchResult<T>` / `SearchHit<T>`
- 客户端 trait：`SearchClient`
- 客户端实现：`ElasticClient`（mock + real）
- 同步器：`SearchSyncer`（业务层 API）

### 2. `backend/src/search/mod.rs` — 模块导出

### 3. `backend/src/routes/search_api.rs` — HTTP 搜索 API

提供 3 个端点：

| Method | Path | 用途 |
|--------|------|------|
| GET | `/api/v1/erp/search/sales-orders?q=...` | 销售订单搜索 |
| GET | `/api/v1/erp/search/customers?q=...` | 客户搜索 |
| GET | `/api/v1/erp/search/products?q=...` | 产品搜索 |

**查询参数**：
- `q` — 关键字
- `from` / `size` — 分页
- `status` / `tier` / `category` — 过滤

## 四、PG + ES 双轨制

| 维度 | PostgreSQL | Elasticsearch |
|------|------------|---------------|
| 角色 | **主数据源** | **搜索副本** |
| 写入 | 业务事务 | 异步同步 |
| 读取 | 关联查询 | 全文搜索 |
| 一致性 | 强一致 | 最终一致（5s 延迟） |
| 容量 | 受限于 DB | 横向扩展 |

### 同步策略

1. **业务写入 PG**（事务）
2. **同步写入 ES**（失败重试 3 次）
3. **异步补偿**：定时任务（每 5 分钟）扫描 5 分钟内变更的记录，修复 ES 缺失
4. **删除策略**：业务删除时同步删除 ES 文档

## 五、搜索 API 设计

### 1. 销售订单搜索

```
GET /api/v1/erp/search/sales-orders?q=ACME&status=approved&from=0&size=20
```

**响应**：
```json
{
  "total": 156,
  "took_ms": 12,
  "hits": [
    {
      "order_no": "SO-20260617-0001",
      "customer_name": "ACME Corp",
      "total_amount": 5000.00,
      "status": "approved",
      "created_at": "2026-06-17T10:00:00Z",
      "tenant_id": "tenant-001"
    }
  ]
}
```

### 2. 客户搜索

```
GET /api/v1/erp/search/customers?q=ACME&tier=A
```

### 3. 产品搜索

```
GET /api/v1/erp/search/products?q=纯棉&color_no=CN-001
```

## 六、单元测试覆盖

| 模块 | 测试数 |
|------|--------|
| `elastic::tests` | 14 |
| `search_api::tests` | 2 |
| **合计** | **16 测试** |

覆盖：
- 索引常量 + 文档类型映射
- 文档结构序列化
- SearchQuery 构造（keyword/filter/pagination/highlight）
- ElasticClient mock（index/search/delete/bulk）
- SearchSyncer（3 文档类型同步）
- SearchParams → SearchQuery 转换

## 七、部署：`deploy/elasticsearch/docker-compose.yml`

包含 3 个服务：

| 服务 | 端口 | 用途 |
|------|------|------|
| **Elasticsearch** | 9200 / 9300 | ES 节点（单节点模式） |
| **Kibana** | 5601 | 可视化 |
| **es-init** | — | 自动创建 3 索引 + mapping |

启动：
```bash
cd deploy/elasticsearch
docker-compose up -d

# 访问
# - ES:    http://localhost:9200
# - Kibana: http://localhost:5601

# 验证索引
curl http://localhost:9200/_cat/indices?v
# 应看到 sales_orders / customers / products
```

## 八、启用 elasticsearch crate（可选）

默认情况下，本模块提供 **mock 实现**。

要启用真实 ES，添加：

```toml
# backend/Cargo.toml
elasticsearch = "8.5.0-alpha.1"
```

然后在 `main.rs` 中替换 `ElasticClient::mock()` 为 `ElasticClient::real(url)`。

## 九、约束遵守

- ✅ **零硬编码**：ES URL 从环境变量读
- ✅ **多租户隔离**：每条文档携带 `tenant_id` 字段
- ✅ **无破坏性变更**：search 模块为新增
- ✅ **可选依赖**：elasticsearch crate 不在默认依赖
- ✅ **数据一致性**：同步重试 + 异步补偿
- ✅ **资源属性**：与 OpenTelemetry trace 关联

## 十、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| ES 部署失败 | 中 | docker-compose 一键启动 |
| elasticsearch crate 编译失败 | 中 | 默认不引入，提供 mock |
| ES 索引同步失败 | 中 | 重试 3 次 + 定时补偿 |
| 搜索结果与 PG 不一致 | 低 | 5 分钟内同步，可接受 |
| 内存占用高 | 中 | ES 默认 512MB 堆，可调 |

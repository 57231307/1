# 冰溪 ERP OpenAPI 3.0 规范

本目录包含冰溪 ERP 项目的完整 OpenAPI 3.0 规范文档。

## 📚 文档清单

| 文档 | 说明 | 行数 |
|------|------|------|
| [openapi.yaml](./openapi.yaml) | 完整 OpenAPI 3.0 规范（v2026.617.0001） | 6764 行 |

## 📊 openapi.yaml 统计

| 指标 | 数值 |
|------|------|
| OpenAPI 版本 | 3.0.3 |
| 文件行数 | 6,764 |
| 路径数 | 191 |
| 端点数 | 288 |
| Tags 数 | 65 |
| Schema 数 | 73 |
| Parameters 数 | 10 |
| Responses 数 | 5 |

## 🔧 使用方式

### Swagger UI 渲染

```bash
# 使用 docker 启动 swagger-ui
docker run -p 8080:8080 \
  -e SWAGGER_JSON=/api/openapi.yaml \
  -v ${PWD}:/api \
  swaggerapi/swagger-ui
```

访问 http://localhost:8080 查看 API 文档。

### Stoplight Elements 渲染

```html
<script src="https://unpkg.com/@stoplight/elements/web-components.min.js"></script>
<elements-api apiDescriptionUrl="openapi.yaml" router="hash" />
```

### Redoc 渲染

```bash
npx redoc-cli serve openapi.yaml
# 访问 http://localhost:8080
```

### 代码生成

```bash
# 生成 TypeScript 客户端
npx @openapitools/openapi-generator-cli generate \
  -i openapi.yaml \
  -g typescript-axios \
  -o ./client/typescript

# 生成 Python 客户端
npx @openapitools/openapi-generator-cli generate \
  -i openapi.yaml \
  -g python \
  -o ./client/python
```

## 📋 业务域分类

OpenAPI 规范按 65 个业务域 tag 组织 API：

- **基础**：认证、用户、角色、租户、部门
- **业务基础**：商品、客户、供应商
- **销售**：销售订单、合同、价格、发货、退货、报价、客户定制
- **采购**：采购订单、收货、退货、合同、价格、检验
- **库存**：库存、调整、调拨、盘点、仓库
- **财务**：凭证、科目、应收、应付、成本、固定资产、财务分析、资金、预算、辅助核算、币种
- **生产**：生产订单、BOM、MRP、工作中心
- **质量**：质量标准、检验、不合格品
- **流程**：BPM
- **通知**：通知、邮件、OA
- **审计日志**：审计、操作日志、API访问、登录、性能
- **报表 BI**：报表、BI
- **业务追溯**
- **AI**：AI分析、AI工艺优化、AI质量预测
- **行业专用**：染整工艺、色卡、物流
- **API 网关**：API密钥、Webhook
- **监控**
- **导入导出**
- **高级分析**
- **灾备**

## 🔗 相关资源

- **GitHub 仓库**：https://github.com/57231307/1
- **API 文档站点**：https://api.bingxi.com/docs
- **OpenAPI 规范**：https://spec.openapis.org/oas/v3.0.3

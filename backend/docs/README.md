# 秉羲管理系统 - 开发文档

## 目录结构

```
backend/
├── src/
│   ├── main.rs              # 应用程序入口
│   ├── openapi.rs           # OpenAPI/Swagger 文档配置
│   ├── lib.rs               # 库根
│   ├── handlers/            # HTTP 请求处理器
│   │   ├── mod.rs
│   │   ├── auth_handler.rs  # 认证处理器
│   │   ├── user_handler.rs  # 用户处理器
│   │   ├── purchase_contract_handler.rs  # 采购合同处理器
│   │   ├── sales_contract_handler.rs     # 销售合同处理器
│   │   ├── fixed_asset_handler.rs        # 固定资产处理器
│   │   └── budget_management_handler.rs  # 预算管理处理器
│   ├── services/            # 业务逻辑层
│   │   ├── mod.rs
│   │   ├── auth_service.rs
│   │   ├── user_service.rs
│   │   ├── purchase_contract_service.rs
│   │   ├── sales_contract_service.rs
│   │   ├── fixed_asset_service.rs
│   │   └── budget_management_service.rs
│   ├── models/              # 数据模型 (SeaORM Entity)
│   ├── routes/              # 路由配置
│   ├── middleware/          # 中间件
│   ├── grpc/                # gRPC 服务
│   └── utils/               # 工具函数
├── tests/                   # 集成测试
├── docs/                    # 文档
│   ├── API.md               # API 文档
│   └── postman_collection.json  # Postman 集合
├── scripts/                 # 脚本
│   ├── performance_test_wrk.sh  # wrk 性能测试
│   └── performance_test_ab.sh   # ab 性能测试
├── proto/                   # Protocol Buffers 定义
└── database/                # 数据库迁移
```

## 生成文档

### 生成 Rust API 文档

```bash
# 生成 HTML 文档
cargo doc --no-deps

# 生成文档并自动打开
cargo doc --no-deps --open

# 包含私有项的文档
cargo doc --no-deps --document-private-items
```

生成的文档位于 `target/doc/bingxi_backend/`

### 生成 Swagger/OpenAPI 文档

启动服务后访问:
- Swagger UI: `http://localhost:8080/swagger-ui/`
- OpenAPI JSON: `http://localhost:8080/api-docs/openapi.json`

### 查看 API 文档

```bash
# Markdown 格式
cat docs/API.md

# Postman 集合
# 导入 docs/postman_collection.json 到 Postman
```

## 运行性能测试

### 使用 wrk

```bash
# 安装 wrk
# Windows: choco install wrk
# macOS: brew install wrk
# Linux: apt-get install wrk

# 运行测试
cd scripts
chmod +x performance_test_wrk.sh
./performance_test_wrk.sh
```

### 使用 Apache Bench

```bash
# 安装 ab
# Windows: choco install apache
# macOS: brew install httpd
# Linux: apt-get install apache2-utils

# 运行测试
cd scripts
chmod +x performance_test_ab.sh
./performance_test_ab.sh
```

## 运行测试

```bash
# 运行所有测试
cargo test

# 运行集成测试
cargo test --test management_services_integration_test

# 运行特定测试
cargo test test_get_purchase_contracts_unauthorized

# 生成测试覆盖率报告 (需要 cargo-tarpaulin)
cargo tarpaulin --out Html
```

## 代码质量检查

```bash
# 格式化代码
cargo fmt

# 检查代码风格
cargo clippy

# 修复自动修复的问题
cargo clippy --fix
```

## 构建和运行

```bash
# 开发模式
cargo run

# 发布模式
cargo build --release
./target/release/server

# 检查编译
cargo check
```

## 配置说明

配置文件位于 `config/settings.yaml`:

```yaml
server:
  http:
    host: "0.0.0.0"
    port: 8080
  grpc:
    host: "0.0.0.0"
    port: 50051

database:
  url: "postgres://user:password@localhost:5432/bingxi"

auth:
  jwt_secret: "your-secret-key"
  token_expiry_hours: 24
```

## 环境变量

创建 `.env` 文件:

```env
DATABASE_URL=postgres://user:password@localhost:5432/bingxi
JWT_SECRET=your-secret-key
RUST_LOG=info,bingxi_backend=debug
```

## API 端点

### 认证
- POST `/api/auth/login` - 用户登录
- POST `/api/auth/verify-token` - 验证 Token

### 采购合同
- GET `/api/v1/erp/purchase-contracts` - 获取列表
- GET `/api/v1/erp/purchase-contracts/{id}` - 获取详情
- POST `/api/v1/erp/purchase-contracts` - 创建合同
- POST `/api/v1/erp/purchase-contracts/{id}/approve` - 审核
- POST `/api/v1/erp/purchase-contracts/{id}/execute` - 执行
- POST `/api/v1/erp/purchase-contracts/{id}/cancel` - 取消
- DELETE `/api/v1/erp/purchase-contracts/{id}` - 删除

### 销售合同
- GET `/api/v1/erp/sales-contracts` - 获取列表
- GET `/api/v1/erp/sales-contracts/{id}` - 获取详情
- POST `/api/v1/erp/sales-contracts` - 创建合同
- POST `/api/v1/erp/sales-contracts/{id}/approve` - 审核
- POST `/api/v1/erp/sales-contracts/{id}/execute` - 执行
- POST `/api/v1/erp/sales-contracts/{id}/cancel` - 取消
- DELETE `/api/v1/erp/sales-contracts/{id}` - 删除

### 固定资产
- GET `/api/v1/erp/fixed-assets` - 获取列表
- GET `/api/v1/erp/fixed-assets/{id}` - 获取详情
- POST `/api/v1/erp/fixed-assets` - 创建资产
- POST `/api/v1/erp/fixed-assets/{id}/depreciate` - 计提折旧
- POST `/api/v1/erp/fixed-assets/{id}/dispose` - 处置资产
- DELETE `/api/v1/erp/fixed-assets/{id}` - 删除资产

### 预算管理
- GET `/api/v1/erp/budget-items` - 获取预算科目列表
- GET `/api/v1/erp/budget-items/{id}` - 获取详情
- POST `/api/v1/erp/budget-items` - 创建预算科目
- PUT `/api/v1/erp/budget-items/{id}` - 更新预算科目
- DELETE `/api/v1/erp/budget-items/{id}` - 删除预算科目
- GET `/api/v1/erp/budget-plans` - 获取预算方案列表
- POST `/api/v1/erp/budget-plans` - 创建预算方案

## 技术栈

- **Web 框架**: Axum 0.7
- **数据库 ORM**: SeaORM 1.0
- **数据库**: PostgreSQL 18
- **认证**: JWT (jsonwebtoken 9.0)
- **gRPC**: Tonic 0.10
- **异步运行时**: Tokio 1.0
- **验证**: validator 0.16
- **日志**: tracing 0.1
- **文档**: utoipa 4 + Swagger UI

## 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 许可证

MIT License

## 联系方式

- 团队：秉羲团队
- 邮箱：support@bingxi.com

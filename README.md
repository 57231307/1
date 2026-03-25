# 秉羲管理系统 - Rust 技术栈版本

秉羲管理系统的全栈 Rust 实现，提供高性能、高可靠性的企业级 ERP 解决方案。

## 技术栈

### 后端
- **框架**: Axum 0.7 + Tokio 1.0
- **ORM**: SeaORM 1.0
- **数据库**: PostgreSQL 18.0
- **认证**: JWT (jsonwebtoken 9.0)
- **gRPC**: Tonic 0.10
- **日志**: tracing + tracing-subscriber

### 前端
- **框架**: Yew 0.21
- **构建工具**: Trunk 0.20
- **路由**: yew-router 0.17
- **HTTP 客户端**: gloo-net 0.4

## 项目结构

```
bingxi-rust/
├── backend/                 # 后端服务
│   ├── src/
│   │   ├── config/         # 配置管理
│   │   ├── database/       # 数据库连接
│   │   ├── handlers/       # API 处理器
│   │   ├── middleware/     # 中间件
│   │   ├── models/         # SeaORM 模型
│   │   ├── routes/         # 路由定义
│   │   ├── services/       # 业务逻辑层
│   │   ├── utils/          # 工具函数
│   │   ├── grpc/           # gRPC 服务
│   │   ├── lib.rs          # 库根
│   │   └── main.rs         # 入口
│   ├── Cargo.toml
│   ├── .env.example        # 配置示例
│   ├── build.sh            # 构建脚本 (Unix)
│   └── build.bat           # 构建脚本 (Windows)
├── frontend/                # 前端应用
│   ├── src/
│   │   ├── app/            # 应用根组件
│   │   ├── components/     # UI 组件
│   │   ├── pages/          # 页面组件
│   │   ├── services/       # API 服务
│   │   ├── models/         # 数据模型
│   │   ├── utils/          # 工具函数
│   │   └── main.rs         # 入口
│   ├── styles/             # 样式文件
│   ├── index.html          # HTML 模板
│   ├── Cargo.toml
│   ├── Trunk.toml          # Trunk 配置
│   ├── build.sh            # 构建脚本 (Unix)
│   └── build.bat           # 构建脚本 (Windows)
└── docs/
    ├── README.md           # 项目说明（本文档）
    ├── api-docs.md         # API 接口文档
    ├── migration-guide.md  # 迁移指南
    ├── data-migration.md   # 数据迁移方案
    ├── progress-report.md  # 项目进度报告
    ├── quickstart.md       # 快速启动指南
    ├── project-structure.md # 项目文件结构
    ├── completion-summary.md # 完成总结
    └── enhancement-summary.md # 完善总结
```

## 快速开始

### 环境要求
- Rust 1.70+
- PostgreSQL 18.0
- Node.js 18+ (可选，用于前端工具链)
- Trunk (前端构建工具)

### 后端启动

1. 克隆项目并进入后端目录
```bash
cd bingxi-rust/backend
```

2. 复制配置文件并修改
```bash
cp .env.example .env
# 编辑 .env 文件，配置数据库连接等信息
```

3. 运行后端服务
```bash
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/bingxi_backend
```

### 前端启动

1. 安装 Trunk
```bash
cargo install --locked trunk
```

2. 进入前端目录并启动开发服务器
```bash
cd bingxi-rust/frontend
trunk serve --open
```

3. 生产构建
```bash
trunk build --release
```

## API 接口

完整的 API 文档请查看：[docs/api-docs.md](docs/api-docs.md)

### 认证模块
- `POST /api/auth/login` - 用户登录

### 用户管理模块
- `GET /api/users` - 获取用户列表（分页）
- `GET /api/users/:id` - 获取用户详情
- `POST /api/users` - 创建用户

### 财务模块
- `GET /api/finance/payments` - 获取付款列表（分页、支持状态筛选）
- `GET /api/finance/payments/:id` - 获取付款详情
- `POST /api/finance/payments` - 创建付款记录

### 销售模块
- `GET /api/sales/orders` - 获取订单列表（分页、支持客户和状态筛选）
- `GET /api/sales/orders/:id` - 获取订单详情
- `POST /api/sales/orders` - 创建销售订单

## 数据库模型

已实现的核心模型：
- User (用户)
- Role (角色)
- Department (部门)
- RolePermission (角色权限)
- FinancePayment (财务付款)
- FinanceInvoice (财务发票)
- SalesOrder (销售订单)
- SalesOrderItem (销售订单明细)
- InventoryStock (库存)
- Product (产品)
- ProductCategory (产品分类)
- Warehouse (仓库)

## 性能目标

- **并发请求**: 5000+ req/s (目标 5 倍于原系统)
- **API 响应时间**: < 50ms (目标 6 倍于原系统)
- **页面加载时间**: < 1s
- **数据库连接池**: 可配置，默认 10 个连接

## 当前进度

- **总体完成度**: 75%
- **后端核心**: 85% ✅
- **前端功能**: 80% ✅
- **业务模块**: 70% ✅
- **文档完善**: 90% ✅
- **测试覆盖**: 0% ⏳

详见：[docs/enhancement-summary.md](docs/enhancement-summary.md)

## 开发规范

- 所有代码使用中文注释和文档
- 遵循 Rust 最佳实践和命名规范
- 所有数据库操作通过 SeaORM 进行
- API 设计遵循 RESTful 规范
- 错误处理使用 thiserror 和 anyhow

## 部署

### 后端部署
后端编译为单个二进制文件，可直接部署：
```bash
cargo build --release
# 复制 target/release/bingxi_backend 到服务器
# 配置环境变量或 .env 文件
# 运行 ./bingxi_backend
```

### 前端部署
前端构建为静态文件，可通过 CDN 或静态服务器部署：
```bash
trunk build --release
# 复制 dist/ 目录到 Nginx/Apache 或 CDN
```

## 测试

运行所有测试：
```bash
# 后端测试
cd backend
cargo test

# 前端测试
cd frontend
cargo test
```

## 许可证

Copyright © 2024 秉羲团队

## 联系方式

- 项目网站：https://github.com/boshi-xixixi
- 问题反馈：请提交 Issue

# 秉羲管理系统 Rust 版 - 项目文件结构

本文档详细说明了秉羲管理系统 Rust 技术栈版本的完整文件结构和组织方式。

## 项目根目录

```
bingxi-rust/
├── backend/                 # 后端服务
├── frontend/                # 前端应用
├── docs/                    # 文档目录
├── README.md               # 项目说明
└── .gitignore              # Git 忽略文件
```

## 后端目录结构 (backend/)

```
backend/
├── Cargo.toml              # Rust 依赖配置
├── .env.example            # 环境变量示例
├── .env                    # 环境变量配置（需创建）
├── build.sh                # Unix 构建脚本
├── build.bat               # Windows 构建脚本
└── src/
    ├── main.rs             # 应用程序入口
    ├── lib.rs              # 库根文件
    │
    ├── config/             # 配置管理模块
    │   ├── mod.rs          # 模块导出
    │   └── settings.rs     # 配置结构定义
    │
    ├── database/           # 数据库连接模块
    │   └── mod.rs          # 数据库连接管理
    │
    ├── models/             # SeaORM 数据模型
    │   ├── mod.rs          # 模型导出
    │   ├── user.rs         # 用户模型
    │   ├── role.rs         # 角色模型
    │   ├── department.rs   # 部门模型
    │   ├── role_permission.rs  # 角色权限模型
    │   ├── finance_payment.rs  # 财务付款模型
    │   ├── finance_invoice.rs  # 财务发票模型
    │   ├── sales_order.rs      # 销售订单模型
    │   ├── sales_order_item.rs # 销售订单明细模型
    │   ├── inventory_stock.rs  # 库存模型
    │   ├── product.rs          # 产品模型
    │   ├── product_category.rs # 产品分类模型
    │   └── warehouse.rs        # 仓库模型
    │
    ├── services/           # 业务逻辑层
    │   ├── mod.rs          # 服务导出
    │   ├── auth_service.rs # 认证服务
    │   └── user_service.rs # 用户服务
    │
    ├── handlers/           # API 处理器
    │   ├── mod.rs          # 处理器导出
    │   ├── auth_handler.rs # 认证处理器
    │   └── user_handler.rs # 用户处理器
    │
    ├── middleware/         # 中间件
    │   ├── mod.rs          # 中间件导出
    │   └── auth.rs         # JWT 认证中间件
    │
    ├── routes/             # 路由定义
    │   └── mod.rs          # 路由配置
    │
    ├── utils/              # 工具函数
    │   ├── mod.rs          # 工具导出
    │   ├── response.rs     # 统一响应格式
    │   └── error.rs        # 错误处理
    │
    └── grpc/               # gRPC 服务（待实现）
        ├── mod.rs          # 模块导出
        ├── server.rs       # gRPC 服务器
        └── proto/          # Protobuf 定义（待生成）
            └── mod.rs
```

### 后端核心文件说明

#### 入口文件
- **main.rs**: 应用程序入口，初始化日志、配置、数据库、中间件和路由
- **lib.rs**: 库根文件，导出所有公共模块

#### 配置模块 (config/)
- **settings.rs**: 定义配置结构（ServerConfig, DatabaseConfig, AuthConfig）
- 使用 `config` crate 从环境变量和文件加载配置

#### 数据模型 (models/)
- 使用 SeaORM 的 `DeriveEntityModel` 宏定义
- 包含字段、关联关系和 ActiveModelBehavior
- 支持异步 CRUD 操作

#### 服务层 (services/)
- 实现业务逻辑
- 封装数据库操作
- 提供领域服务

#### 处理器 (handlers/)
- 处理 HTTP 请求
- 参数验证
- 调用服务层
- 返回 HTTP 响应

#### 中间件 (middleware/)
- **auth.rs**: JWT 令牌验证
- 请求拦截和预处理
- 权限检查

## 前端目录结构 (frontend/)

```
frontend/
├── Cargo.toml              # Rust 依赖配置
├── Trunk.toml              # Trunk 构建配置
├── index.html              # HTML 入口模板
├── build.sh                # Unix 构建脚本
├── build.bat               # Windows 构建脚本
├── styles/                 # 样式文件
│   └── main.css            # 主样式表
└── src/
    ├── main.rs             # 前端入口
    │
    ├── app/                # 应用根组件
    │   └── mod.rs          # App 组件和路由定义
    │
    ├── components/         # UI 组件
    │   ├── mod.rs          # 组件导出
    │   ├── layout.rs       # 布局组件
    │   ├── button.rs       # 按钮组件
    │   ├── input.rs        # 输入框组件
    │   └── modal.rs        # 模态框组件
    │
    ├── pages/              # 页面组件
    │   ├── mod.rs          # 页面导出
    │   ├── login.rs        # 登录页面
    │   ├── dashboard.rs    # 仪表盘页面
    │   └── user_list.rs    # 用户列表页面
    │
    ├── services/           # API 服务层
    │   ├── mod.rs          # 服务导出
    │   ├── api.rs          # HTTP API 客户端
    │   └── auth.rs         # 认证服务
    │
    ├── models/             # 数据模型
    │   ├── mod.rs          # 模型导出
    │   ├── user.rs         # 用户数据模型
    │   └── auth.rs         # 认证数据模型
    │
    └── utils/              # 工具函数
        ├── mod.rs          # 工具导出
        ├── storage.rs      # 本地存储工具
        └── format.rs       # 格式化工具
```

### 前端核心文件说明

#### 入口文件
- **main.rs**: 初始化 panic hook，渲染根组件
- 使用 `console_error_panic_hook` 捕获错误

#### 应用组件 (app/)
- **mod.rs**: 定义路由枚举和根组件
- 使用 `yew-router` 进行路由管理
- 定义路由：Login, Dashboard, Users, NotFound

#### 页面组件 (pages/)
- **login.rs**: 登录页面
- **dashboard.rs**: 仪表盘页面
- **user_list.rs**: 用户列表页面

#### 服务层 (services/)
- **api.rs**: HTTP API 客户端（使用 gloo-net）
- **auth.rs**: 认证服务（登录、令牌管理）

#### 样式 (styles/)
- **main.css**: 全局样式
- 响应式设计
- 主题色定义

## 文档目录 (docs/)

```
docs/
├── migration-guide.md      # 迁移指南（完整的技术迁移文档）
├── data-migration.md       # 数据迁移方案（详细的数据库迁移步骤）
├── progress-report.md      # 项目进度报告（当前完成情况和计划）
└── quickstart.md           # 快速启动指南（5 分钟快速开始）
```

## 配置文件详解

### 后端配置 (.env)

```env
# 服务器配置
SERVER__HOST=0.0.0.0
SERVER__PORT=8000

# 数据库配置
DATABASE__CONNECTION_STRING=postgresql://user:password@localhost:5432/bingxi?Version=18.0
DATABASE__MAX_CONNECTIONS=10

# JWT 认证配置
JWT__SECRET=your-super-secret-jwt-key
JWT__EXPIRATION_HOURS=24

# 日志配置
RUST_LOG=bingxi_backend=debug,tower_http=debug
```

### 前端配置 (Trunk.toml)

```toml
[build]
target = "index.html"
dist = "dist"
public_url = "/"

[serve]
address = "127.0.0.1"
port = 3000
open = true

[watch]
watch = ["src", "assets", "index.html"]
```

## 依赖版本

### 后端依赖 (backend/Cargo.toml)

```toml
[dependencies]
axum = "0.7"                    # Web 框架
tokio = { version = "1.0", features = ["full"] }  # 异步运行时
sea-orm = { version = "1.0", features = ["sqlx-postgres", "runtime-tokio-rustls" ] }  # ORM
tonic = "0.10"                  # gRPC 框架
jsonwebtoken = "9.0"            # JWT 认证
bcrypt = "0.15"                 # 密码哈希
serde = { version = "1.0", features = ["derive"] }  # 序列化
serde_json = "1.0"              # JSON 处理
validator = { version = "0.16", features = ["derive"] }  # 验证
chrono = { version = "0.4", features = ["serde"] }  # 日期时间
config = "0.14"                 # 配置管理
tracing = "0.1"                 # 日志
tower-http = { version = "0.5", features = ["cors", "trace"] }  # 中间件
rust_decimal = "1.0"            # 高精度数字
thiserror = "1.0"               # 错误处理
```

### 前端依赖 (frontend/Cargo.toml)

```toml
[dependencies]
yew = { version = "0.21", features = ["csr"] }  # 前端框架
yew-router = "0.17"               # 路由
wasm-bindgen = "0.2"              # WASM 绑定
console_error_panic_hook = "0.1"  # 错误捕获
gloo-net = "0.4"                  # HTTP 客户端
serde = { version = "1.0", features = ["derive"] }  # 序列化
chrono = { version = "0.4", features = ["serde"] }  # 日期时间
```

## 模块依赖关系

### 后端模块依赖

```
main.rs
├── config/settings.rs
├── database/mod.rs
├── routes/mod.rs
│   ├── handlers/auth_handler.rs
│   └── handlers/user_handler.rs
│       ├── services/auth_service.rs
│       └── services/user_service.rs
│           └── models/*.rs
└── middleware/auth.rs
```

### 前端模块依赖

```
main.rs
└── app/mod.rs
    ├── pages/login.rs
    ├── pages/dashboard.rs
    └── pages/user_list.rs
        ├── services/api.rs
        └── models/user.rs
```

## 代码组织原则

### 1. 分层架构
- **表现层**: handlers, pages
- **应用层**: services
- **领域层**: models
- **基础设施层**: database, config, utils

### 2. 关注点分离
- 每个模块只负责单一职责
- 服务层封装业务逻辑
- 处理器只处理 HTTP 协议

### 3. 依赖注入
- 使用 Axum 的 State 提取器注入数据库连接
- 服务层通过参数传递依赖

### 4. 错误处理
- 使用 thiserror 定义错误类型
- 使用 anyhow 处理应用错误
- 统一响应格式

## 命名规范

### Rust 命名规范
- **变量和函数**: snake_case（如 `create_user`, `user_id`）
- **类型和结构体**: PascalCase（如 `User`, `AuthService`）
- **常量**: UPPER_SNAKE_CASE（如 `MAX_CONNECTIONS`）
- **文件名**: snake_case（如 `auth_handler.rs`）

### 中文使用规范
- 所有注释使用中文
- 文档使用中文
- 错误消息使用中文
- 日志输出使用中文

## 构建产物

### 后端构建
```bash
cargo build --release
# 输出：target/release/bingxi_backend (单个二进制文件)
```

### 前端构建
```bash
trunk build --release
# 输出：dist/ 目录（静态文件）
# - index.html
# - *.wasm (WebAssembly 模块)
# - *.js (JavaScript glue code)
# - *.css (样式文件)
```

## 测试组织

### 后端测试
```
backend/
├── src/
│   └── services/
│       ├── auth_service.rs
│       └── tests/         # 测试模块
│           └── auth_test.rs
└── tests/                 # 集成测试
    └── api_test.rs
```

### 前端测试
```
frontend/
├── src/
│   └── components/
│       └── tests/         # 组件测试
│           └── button_test.rs
└── tests/                 # E2E 测试
    └── e2e_test.rs
```

## 版本控制

### Git 分支策略
```
main          # 主分支，生产环境代码
develop       # 开发分支
feature/*     # 功能分支
bugfix/*      # 修复分支
release/*     # 发布分支
```

### 提交信息规范
```
feat: 新功能
fix: 修复 bug
docs: 文档更新
style: 代码格式
refactor: 重构
test: 测试
chore: 构建/工具
```

## 总结

秉羲管理系统 Rust 技术栈版本采用现代化的项目结构：

- **清晰的分层架构**: 表现层 → 应用层 → 领域层 → 基础设施层
- **模块化设计**: 每个模块职责单一，易于维护
- **完整的文档**: 包含 README、迁移指南、快速启动等文档
- **标准化配置**: 统一的配置文件和构建脚本
- **中文友好**: 全中文注释和文档

这种结构既遵循 Rust 最佳实践，又考虑了团队协作和长期维护的需求。

---

**最后更新**: 2024-01-XX  
**维护者**: 秉羲团队

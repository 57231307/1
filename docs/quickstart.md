# 秉羲管理系统 Rust 版 - 快速启动指南

本指南将帮助您在 5 分钟内启动秉羲管理系统 Rust 版的开发和运行环境。

## 环境准备

### 必需软件

1. **Rust 工具链**
   ```bash
   # Windows (使用 winget)
   winget install Rustlang.Rust.MSVC
   
   # 或使用 rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **PostgreSQL 18**
   - 下载地址：https://www.postgresql.org/download/
   - 安装时记住端口（默认 5432）和管理员密码

3. **开发工具（推荐）**
   - VS Code + rust-analyzer 扩展
   - Git

### 可选软件

- Trunk（前端构建工具，安装 Rust 后可通过 cargo 安装）
  ```bash
  cargo install --locked trunk
  ```

## 第一步：克隆项目

```bash
git clone <项目地址> bingxi-rust
cd bingxi-rust
```

## 第二步：配置数据库

1. **创建数据库**
   ```sql
   -- 使用 psql 或 pgAdmin 连接 PostgreSQL
   CREATE DATABASE bingxi_rust;
   CREATE USER bingxi_user WITH PASSWORD 'your_password';
   GRANT ALL PRIVILEGES ON DATABASE bingxi_rust TO bingxi_user;
   ```

2. **运行数据库迁移**（待实现）
   ```bash
   # 后续会提供迁移脚本
   # 目前需要手动创建表结构（参考 docs/data-migration.md）
   ```

## 第三步：配置后端

1. **进入后端目录**
   ```bash
   cd backend
   ```

2. **复制配置文件**
   ```bash
   cp .env.example .env
   ```

3. **编辑 .env 文件**
   ```bash
   # 使用记事本或 VS Code 打开 .env
   notepad .env
   ```

   修改以下配置：
   ```env
   # 数据库连接（修改密码和主机）
   DATABASE__CONNECTION_STRING=postgresql://bingxi_user:your_password@localhost:5432/bingxi_rust?Version=18.0
   
   # JWT 密钥（生产环境请修改为随机字符串）
   JWT__SECRET=your-super-secret-jwt-key-change-this-in-production
   
   # 日志级别
   RUST_LOG=bingxi_backend=debug,tower_http=debug
   ```

4. **运行后端**
   ```bash
   # 开发模式运行
   cargo run
   
   # 或构建后运行
   cargo build --release
   ./target/release/bingxi_backend
   ```

   看到以下日志表示启动成功：
   ```
   启动秉羲管理系统 Rust 版
   配置加载成功
   数据库连接成功
   服务器监听地址：0.0.0.0:8000
   ```

5. **测试后端 API**
   ```bash
   # 新开一个终端
   curl http://localhost:8000/api/health
   ```

## 第四步：配置前端

1. **进入前端目录**
   ```bash
   cd ../frontend
   ```

2. **安装 Trunk（如果未安装）**
   ```bash
   cargo install --locked trunk
   ```

3. **启动开发服务器**
   ```bash
   trunk serve --open
   ```

   浏览器会自动打开 http://localhost:3000

   看到登录页面表示启动成功。

## 第五步：验证安装

### 后端验证

1. **检查 API 端点**
   ```bash
   # 健康检查（待实现）
   curl http://localhost:8000/api/health
   
   # 登录接口
   curl -X POST http://localhost:8000/api/auth/login \
     -H "Content-Type: application/json" \
     -d '{"username":"admin","password":"admin123"}'
   ```

### 前端验证

1. 打开浏览器访问 http://localhost:3000
2. 查看是否显示"秉羲管理系统"登录页面
3. 检查浏览器控制台是否有错误（F12）

## 常见问题

### 1. 数据库连接失败

**错误信息**: `database connection failed`

**解决方案**:
- 检查 PostgreSQL 服务是否运行
- 确认 .env 中的连接字符串正确
- 确认数据库用户和密码正确
- 检查防火墙设置

### 2. 端口被占用

**错误信息**: `Address already in use`

**解决方案**:
```bash
# Windows - 查找占用端口的进程
netstat -ano | findstr :8000
taskkill /PID <PID> /F

# 修改 .env 中的端口
SERVER__PORT=8001
```

### 3. Rust 编译错误

**错误信息**: `error[E0433]: failed to resolve: use of undeclared crate or module`

**解决方案**:
```bash
# 清理并重新构建
cargo clean
cargo build
```

### 4. 前端无法访问后端

**错误信息**: `Network Error` 或 `CORS error`

**解决方案**:
- 确认后端已启动且运行在 8000 端口
- 检查后端日志中的 CORS 配置
- 前端代理配置（开发环境）

## 开发工作流

### 日常开发

1. **启动后端**（终端 1）
   ```bash
   cd backend
   cargo run
   ```

2. **启动前端**（终端 2）
   ```bash
   cd frontend
   trunk serve
   ```

3. **开发调试**
   - 后端代码修改后自动重载（使用 cargo-watch）
     ```bash
     cargo install cargo-watch
     cargo watch -x run
     ```
   - 前端代码修改后自动重载（Trunk 内置）

### 构建发布版本

1. **构建后端**
   ```bash
   cd backend
   cargo build --release
   ```

2. **构建前端**
   ```bash
   cd frontend
   trunk build --release
   ```

   构建产物在 `frontend/dist/` 目录

## 项目结构概览

```
bingxi-rust/
├── backend/              # 后端服务（Axum + SeaORM）
│   ├── src/
│   │   ├── handlers/    # API 处理器
│   │   ├── services/    # 业务逻辑
│   │   ├── models/      # 数据模型
│   │   ├── routes/      # 路由定义
│   │   └── main.rs      # 入口文件
│   ├── .env             # 配置文件
│   └── Cargo.toml       # 依赖配置
├── frontend/            # 前端应用（Yew + Trunk）
│   ├── src/
│   │   ├── pages/      # 页面组件
│   │   ├── components/ # UI 组件
│   │   └── main.rs     # 入口文件
│   ├── index.html      # HTML 模板
│   └── Cargo.toml      # 依赖配置
└── docs/               # 文档
```

## 下一步

1. **阅读开发文档**
   - [项目 README](../README.md)
   - [迁移指南](./migration-guide.md)
   - [数据迁移方案](./data-migration.md)

2. **开始开发功能**
   - 实现完整的登录功能
   - 添加用户管理 CRUD
   - 开发业务模块

3. **参与贡献**
   - 查看 Issue 列表
   - 提交 Pull Request
   - 参与代码审查

## 获取帮助

- **项目文档**: `docs/` 目录
- **问题反馈**: 提交 Issue
- **技术讨论**: 加入项目群组

---

**提示**: 本指南假设您已具备基本的 Rust 开发知识。如果您是 Rust 新手，建议先学习 [The Rust Programming Language](https://doc.rust-lang.org/book/)。

**最后更新**: 2024-01-XX

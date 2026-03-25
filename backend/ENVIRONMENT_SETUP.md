# 秉羲 Rust 后端 - 开发环境配置文档

> **最后更新**: 2026-03-15  
> **Rust 版本**: 1.94.0  
> **项目路径**: `e:\1\10\bingxi-rust\backend`

---

## 📦 已安装的工具和环境

### 1. Rust 工具链
- **版本**: 1.94.0
- **工具链**: `stable-x86_64-pc-windows-gnu` (GNU)
- **默认工具链**: GNU 工具链
- **Cargo 版本**: 1.94.0

### 2. MinGW-w64 (Linker)
- **版本**: GCC 16.0.1 (MinGW-w64 UCRT)
- **安装路径**: `e:\1\mingw64\mingw64`
- **Bin 路径**: `e:\1\mingw64\mingw64\bin`
- **用途**: 作为 Rust GNU 工具链的 linker

### 3. protoc (Protocol Buffers)
- **版本**: 29.3
- **安装路径**: `e:\1\protoc`
- **Bin 路径**: `e:\1\protoc\bin`
- **用途**: 编译 `.proto` 文件（gRPC 所需）

---

## 🔧 环境变量配置

以下路径已添加到用户环境变量 PATH：

```
e:\1\mingw64\mingw64\bin
e:\1\protoc\bin
```

---

## 📋 项目依赖

### 核心依赖

| 类别 | 库 | 版本 | 特性 |
|------|-----|------|------|
| **Web 框架** | axum | 0.7 | json, multipart |
| **异步运行时** | tokio | 1.0 | full |
| **数据库 ORM** | sea-orm | 1.0 | sqlx-postgres, runtime-tokio-rustls |
| **序列化** | serde | 1.0 | derive |
| **序列化** | serde_json | 1.0 | - |
| **认证** | jsonwebtoken | 9.0 | - |
| **认证** | bcrypt | 0.15 | - |
| **gRPC** | tonic | 0.10 | - |
| **gRPC** | prost | 0.12 | - |

### 其他重要依赖

- **验证**: validator 0.16 (with derive)
- **配置**: config 0.14, dotenvy 0.15
- **日志**: tracing 0.1, tracing-subscriber 0.3, tracing-appender 0.2
- **时间**: chrono 0.4 (with serde)
- **邮件**: lettre 0.11 (tokio1-rustls-tls, builder, smtp-transport)
- **Excel**: rust_xlsxwriter 0.58, calamine 0.24
- **并发**: dashmap 5.5
- **监控**: prometheus 0.13

---

## 🚀 常用命令

### 环境验证

```powershell
# 验证 Rust
rustc --version
cargo --version
rustup show

# 验证 GCC
gcc --version

# 验证 protoc
protoc --version
```

### 项目构建

```powershell
# 进入项目目录
cd e:\1\10\bingxi-rust\backend

# 清理构建缓存
cargo clean

# 检查代码（快速）
cargo check

# 编译项目（debug 模式）
cargo build

# 编译项目（release 模式）
cargo build --release

# 运行项目
cargo run

# 运行测试
cargo test
```

### 依赖管理

```powershell
# 更新依赖
cargo update

# 添加新依赖
cargo add <crate-name>

# 移除依赖
cargo remove <crate-name>
```

---

## ⚙️ 配置文件

### .cargo/config.toml

```toml
# Windows GNU 工具链配置
# 使用 Rust 自带的 MinGW 工具链，不指定自定义 linker
```

### Cargo.toml 关键配置

```toml
[package]
name = "bingxi-backend"
version = "1.0.0"
edition = "2021"

[dependencies]
# 邮件配置（已修复 TLS 冲突）
lettre = { version = "0.11", default-features = false, features = ["tokio1-rustls-tls", "builder", "smtp-transport"] }

# 数据库配置
sea-orm = { version = "1.0", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros"] }

[profile.release]
lto = true
codegen-units = 1
opt-level = 3
strip = true
```

---

## 🛠️ 已知问题和解决方案

### 问题 1: linker 找不到

**症状**: `error: linker 'x86_64-w64-mingw32-gcc.exe' not found`

**解决**: 确保 MinGW 路径已添加到环境变量，并且使用的是 GNU 工具链

```powershell
rustup show  # 确认工具链
```

### 问题 2: protoc 未找到

**症状**: `Could not find 'protoc'`

**解决**: 确保 protoc 已安装并添加到 PATH

```powershell
protoc --version  # 验证安装
```

### 问题 3: lettre TLS 冲突

**症状**: `Lettre is being built with the 'tokio1' and the 'native-tls' features...`

**解决**: 已在 Cargo.toml 中配置 `default-features = false`

---

## 📚 项目结构

```
backend/
├── .cargo/
│   └── config.toml          # Cargo 配置
├── proto/
│   └── bingxi.proto         # gRPC 定义
├── src/
│   ├── config/              # 配置模块
│   ├── models/              # 数据模型（SeaORM）
│   ├── routes/              # 路由定义
│   ├── services/            # 业务逻辑
│   ├── utils/               # 工具函数
│   ├── lib.rs               # 库入口
│   └── main.rs              # 程序入口
├── Cargo.toml               # 项目配置
├── Cargo.lock               # 依赖锁定
└── build.rs                 # 构建脚本（生成 gRPC 代码）
```

---

## 🔐 安全配置

### 敏感信息管理

- 数据库连接字符串、API 密钥等敏感信息存储在 `.env` 文件
- `.env` 文件已添加到 `.gitignore`
- 使用 `dotenvy` 加载环境变量

### 示例 .env 文件

```env
DATABASE_URL=postgresql://user:password@host:5432/database
JWT_SECRET=your-secret-key
SMTP_SERVER=smtp.example.com
SMTP_PORT=587
```

---

## 📖 开发规范

### 代码风格

- 使用 `rustfmt` 格式化代码
- 使用 `clippy` 进行代码检查
- 遵循 Rust 官方编码规范

### 格式化代码

```powershell
# 格式化代码
cargo fmt

# 运行 clippy
cargo clippy
```

### 提交前检查

```powershell
# 完整检查
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

---

## 🎯 下一步

环境已完全配置完成！现在可以：

1. ✅ 开始开发新功能
2. ✅ 运行和测试项目
3. ✅ 编译和部署应用

### 快速开始

```powershell
cd e:\1\10\bingxi-rust\backend
cargo run
```

---

## 📞 需要帮助？

如果遇到问题，请检查：

1. 环境变量是否正确配置
2. Rust 工具链是否为 GNU 版本
3. 所有依赖是否已正确下载
4. `.env` 文件是否存在并配置正确

---

**文档维护**: BingXi Team  
**最后审查**: 2026-03-15

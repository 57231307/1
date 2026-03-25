# 秉羲 ERP 系统 - 部署指南

**文档版本**: v1.0.0  
**最后更新**: 2026-03-21  
**部署环境**: Windows Server  
**部署路径**: `E:\1\10\bingxi-rust\backend`

---

## 📋 部署前检查清单

### 1. 环境要求
- [ ] **操作系统**: Windows 10/Server 2019+
- [ ] **Rust 版本**: 1.70+ (稳定版)
- [ ] **PostgreSQL**: 18.0+ (已安装并运行)
- [ ] **GCC 编译器**: 已安装 (用于编译ring等依赖)
- [ ] **Protoc**: 3.0+ (用于gRPC代码生成)

### 2. 项目文件完整性
- [ ] **项目代码**: 已下载到 `E:\1\10\bingxi-rust`
- [ ] **数据库迁移脚本**: 已准备
- [ ] **配置文件**: `.env` 已配置
- [ ] **编译脚本**: `check_build.bat` 可用

---

## 🛠️ 环境配置

### 2.1 安装 Rust
```powershell
# 下载并安装 Rust
Invoke-WebRequest -Uri https://win.rustup.rs/ -OutFile rustup-init.exe
.\rustup-init.exe -y

# 配置国内镜像源 (加速下载)
$env:CARGO_REGISTRY_INDEX = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
$env:RUSTUP_DIST_SERVER = "https://mirrors.tuna.tsinghua.edu.cn/rustup"

# 验证安装
rustc --version
cargo --version
```

### 2.2 安装 PostgreSQL
```powershell
# 下载 PostgreSQL 18.0
# 官网: https://www.postgresql.org/download/

# 安装后创建数据库
psql -U postgres -c "CREATE DATABASE bingxi_erp;"
psql -U postgres -c "CREATE USER bingxi_user WITH PASSWORD 'your_password';"
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE bingxi_erp TO bingxi_user;"
```

### 2.3 安装 GCC 编译器
```powershell
# 使用 MSYS2 安装 MinGW-w64
# 下载: https://www.msys2.org/

# 在 MSYS2 shell 中执行:
pacman -S mingw-w64-x86_64-gcc

# 添加环境变量
$env:Path += ";C:\msys64\mingw64\bin"
```

### 2.4 安装 Protoc
```powershell
# 下载 protoc
# https://github.com/protocolbuffers/protobuf/releases

# 解压并添加环境变量
$env:Path += ";C:\protoc\bin"

# 验证安装
protoc --version
```

---

## 🔧 项目配置

### 3.1 配置数据库连接

创建 `.env` 文件:

```env
# 数据库配置
DATABASE_URL=postgres://bingxi_user:your_password@localhost:5432/bingxi_erp

# 服务器配置
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# gRPC 配置
GRPC_HOST=0.0.0.0
GRPC_PORT=50051

# JWT 密钥
JWT_SECRET=your_jwt_secret_here_min_32_characters_long
JWT_EXPIRATION=86400

# 日志配置
LOG_LEVEL=info
LOG_DIR=./logs

# 环境
ENV=production
```

### 3.2 数据库迁移

```powershell
cd E:\1\10\bingxi-rust\backend

# 运行数据库迁移（待实现）
cargo run --bin migrate

# 或者手动执行 SQL 脚本
psql -U bingxi_user -d bingxi_erp -f ./migrations/init.sql
```

---

## 🚀 编译和部署

### 4.1 编译前检查

运行编译检查脚本:

```powershell
cd E:\1\10\bingxi-rust\backend
.\check_build.bat
```

**期望输出**:
```
[INFO] Running cargo check...
[INFO] Cargo check completed. Exit code: 0
[INFO] Results:
    Finished dev [unoptimized + debuginfo] target(s) in xx.xx secs
[INFO] Done.
```

### 4.2 编译项目

```powershell
# 编译 Debug 版本
cargo build

# 编译 Release 版本（推荐用于生产）
cargo build --release
```

**编译时间**: 约 5-10 分钟（首次编译）

### 4.3 运行测试

```powershell
# 运行单元测试
cargo test

# 运行特定模块测试
cargo test --test integration_tests
```

### 4.4 启动服务

```powershell
# Debug 模式运行
cargo run

# Release 模式运行（推荐用于生产）
cargo run --release

# 或者直接运行编译后的二进制文件
.\target\release\server.exe
```

### 4.5 验证服务

```powershell
# 检查服务状态
curl http://localhost:8080/api/v1/erp/health/health

# 期望返回: {"status":"success","data":"healthy","message":"服务运行正常"}
```

---

## 📊 部署验证

### 5.1 检查服务端口

```powershell
# 检查端口监听
netstat -ano | findstr "8080"
netstat -ano | findstr "50051"
```

### 5.2 日志检查

```powershell
# 查看实时日志
tail -f .\logs\bingxi_backend.log

# Windows 下使用 PowerShell
type .\logs\bingxi_backend.log | Select-Object -Tail 100
```

### 5.3 API 测试

```powershell
# 测试登录接口
curl -X POST http://localhost:8080/api/v1/erp/auth/login `
  -H "Content-Type: application/json" `
  -d '{"username":"admin","password":"admin123"}'

# 测试产品列表
curl -X GET http://localhost:8080/api/v1/erp/products/list `
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

---

## 🐛 常见问题

### 问题1: 编译错误 "cannot find crate for 'bpm_service'"
**解决方案**: 已从 services/mod.rs 中移除 bpm_service 和 code_conversion_service 的声明

### 问题2: 模型导入错误
**解决方案**: code_conversion_service.rs 已删除，依赖的7个映射表模型待实现

### 问题3: gRPC 重复导入
**解决方案**: 已为 gRPC trait 和服务实现添加别名区分

### 问题4: CreateProductColorInput 缺失
**解决方案**: 已在 product_service.rs 中添加该结构体

### 问题5: 连接数据库失败
**解决方案**: 
- 检查 .env 中的 DATABASE_URL
- 确认 PostgreSQL 服务已启动
- 验证用户名和密码

---

## 📈 性能优化

### 生产环境建议

1. **使用 Release 模式编译**
   ```powershell
   cargo build --release
   ```

2. **配置数据库连接池**
   ```env
   DATABASE_MAX_CONNECTIONS=20
   DATABASE_MIN_CONNECTIONS=5
   ```

3. **启用日志轮转**
   ```powershell
   # 配置日志大小限制（50MB）
   $env:LOG_MAX_SIZE=52428800
   $env:LOG_MAX_FILES=7
   ```

4. **使用反向代理（Nginx）**
   ```nginx
   server {
       listen 80;
       server_name your-domain.com;
       
       location / {
           proxy_pass http://127.0.0.1:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
       }
   }
   ```

---

## 🔄 后续维护

### 日常维护

1. **监控日志**: 定期检查日志文件
2. **备份数据库**: 每日自动备份
3. **更新依赖**: 每月运行 `cargo update`
4. **安全更新**: 及时应用 Rust 安全补丁

### 版本更新

```powershell
# 更新代码
git pull origin main

# 重新编译
cargo build --release

# 重启服务
Restart-Service -Name "BingXi ERP"
```

---

## 📞 技术支持

- **项目负责人**: [待填写]
- **开发团队**: [待填写]
- **部署日期**: 2026-03-21
- **文档版本**: v1.0.0

---

## ✅ 部署检查清单

- [ ] 环境要求已满足
- [ ] 数据库已创建并配置
- [ ] `.env` 文件已正确配置
- [ ] 代码已编译通过 `cargo check`
- [ ] Release 版本已编译 `cargo build --release`
- [ ] 所有测试已通过 `cargo test`
- [ ] 服务已启动并验证
- [ ] API 接口可正常访问
- [ ] 日志系统正常工作
- [ ] 备份策略已配置

---

**🎉 部署完成！**

系统已准备就绪，可以开始使用了。

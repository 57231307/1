# 秉羲 ERP CLI 维护命令

`bingxi` 是秉羲 ERP 系统的命令行运维工具，提供系统管理、监控、备份等常用功能。

## 安装

CLI 工具已集成在后端项目中，编译后自动可用。

### 编译

```bash
cd backend
cargo build --release
```

编译后的二进制文件位于：
- 开发版：`backend/target/debug/bingxi`
- 生产版：`backend/target/release/bingxi`

### 部署到系统路径

```bash
# 复制到系统路径
sudo cp backend/target/release/bingxi /usr/local/bin/

# 验证安装
bingxi --version
```

## 快速开始

### 查看所有命令

```bash
bingxi --help
```

### 查看服务状态

```bash
bingxi status
```

输出示例：
```
🔍 检查服务状态...

✅ 后端服务：运行中
✅ Nginx 服务：运行中
⚠️  PostgreSQL 服务：使用远程数据库
```

## 命令参考

### 服务管理

#### 查看状态
```bash
bingxi status
```

#### 启动服务
```bash
bingxi start
```

#### 停止服务
```bash
bingxi stop
```

#### 重启服务
```bash
bingxi restart
```

### 日志查看

#### 查看后端日志（最近 100 行）
```bash
bingxi logs
```

#### 查看指定行数
```bash
bingxi logs --lines 50
```

#### 实时跟踪日志
```bash
bingxi logs --follow
```

#### 查看不同类型日志
```bash
# 后端日志
bingxi logs --type backend

# 前端日志
bingxi logs --type frontend

# Nginx 日志
bingxi logs --type nginx
```

### 数据备份

#### 完整备份（数据库 + 文件）
```bash
bingxi backup
```

#### 仅备份数据库
```bash
bingxi backup --type database
```

#### 仅备份文件
```bash
bingxi backup --type files
```

备份位置：`/opt/bingxi/backups/<timestamp>/`

### 数据恢复

#### 恢复数据库
```bash
bingxi restore --file /opt/bingxi/backups/1234567890/database.sql
```

### 健康检查

```bash
bingxi health
```

检查项目：
- HTTP 服务状态
- 数据库连接
- 磁盘空间

### 数据库迁移

#### 应用迁移
```bash
bingxi migrate --direction up
```

#### 回滚迁移
```bash
bingxi migrate --direction down
```

### 版本回滚

```bash
bingxi rollback --version v2026.517.1040
```

### 系统清理

```bash
bingxi clean
```

清理内容：
- 30 天前的日志
- 90 天前的备份

### 配置查看

```bash
bingxi config
```

显示配置：
- 后端配置：`/opt/bingxi/backend/config.yaml`
- Nginx 配置：`/etc/nginx/sites-available/bingxi`

### 密码哈希

```bash
bingxi hash-password --password "your_password"
```

生成 Argon2id 哈希密码，用于创建或重置用户密码。

## 使用场景

### 日常巡检

```bash
# 1. 检查服务状态
bingxi status

# 2. 健康检查
bingxi health

# 3. 查看最近日志
bingxi logs --lines 50
```

### 部署新版本

```bash
# 1. 备份当前版本
bingxi backup

# 2. 停止服务
bingxi stop

# 3. 部署新版本（手动）

# 4. 启动服务
bingxi start

# 5. 健康检查
bingxi health
```

### 故障排查

```bash
# 1. 查看服务状态
bingxi status

# 2. 实时查看日志
bingxi logs --follow --type backend

# 3. 检查配置
bingxi config

# 4. 检查磁盘空间
bingxi health
```

### 紧急回滚

```bash
# 1. 停止服务
bingxi stop

# 2. 恢复数据库
bingxi restore --file /opt/bingxi/backups/<timestamp>/database.sql

# 3. 启动服务
bingxi start
```

## 故障排除

### 命令不存在

```bash
# 检查是否已编译
ls -la backend/target/release/bingxi

# 检查 PATH
which bingxi

# 重新编译
cargo build --release
```

### 权限错误

```bash
# 使用 sudo
sudo bingxi status

# 或修改权限
sudo chown root:root /usr/local/bin/bingxi
sudo chmod 755 /usr/local/bin/bingxi
```

## 版本历史

- **v2.0.0** (2026-05-17): 初始版本

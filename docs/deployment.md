# 秉羲管理系统 - 部署文档

## 概述

本文档详细描述了秉羲管理系统 Rust 版的部署流程，包括后端服务、前端应用、数据库和 Nginx 的配置与部署。

**系统架构**:
- **后端**: Rust + Axum + SeaORM + PostgreSQL 18
- **前端**: Rust + Yew + Trunk
- **通信**: REST API + gRPC
- **Web 服务器**: Nginx
- **操作系统**: Linux (Ubuntu/CentOS)

---

## 部署前准备

### 1. 服务器要求

**最低配置**:
- CPU: 2 核心
- 内存：4GB
- 磁盘：20GB
- 操作系统：Ubuntu 20.04+ 或 CentOS 8+

**推荐配置**:
- CPU: 4 核心
- 内存：8GB
- 磁盘：50GB SSD
- 操作系统：Ubuntu 22.04 LTS

### 2. 软件依赖

#### 安装系统依赖

**Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install -y curl git postgresql-client nginx
```

**CentOS/RHEL**:
```bash
sudo yum update
sudo yum install -y curl git postgresql nginx
```

#### 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

验证安装：
```bash
rustc --version
cargo --version
```

#### 安装 PostgreSQL 18

**Ubuntu**:
```bash
# 添加 PostgreSQL 官方源
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo apt-key add -
sudo apt-get update
sudo apt-get install -y postgresql-18
```

**CentOS**:
```bash
sudo yum install -y https://download.postgresql.org/pub/repos/yum/reporpms/EL-8-x86_64/pgdg-redhat-repo-latest.noarch.rpm
sudo yum install -y postgresql18-server
sudo /usr/pgsql-18/bin/postgresql-18-setup initdb
```

---

## 数据库部署

### 1. 配置 PostgreSQL

编辑 PostgreSQL 配置文件：

**Ubuntu**: `/etc/postgresql/18/main/postgresql.conf`
**CentOS**: `/var/lib/pgsql/18/data/postgresql.conf`

```conf
# 监听所有 IP
listen_addresses = '*'

# 最大连接数
max_connections = 100

# 日志配置
log_destination = 'stderr'
logging_collector = on
log_directory = 'log'
log_filename = 'postgresql-%Y-%m-%d.log'
```

### 2. 配置访问控制

编辑 `pg_hba.conf`:

```conf
# 允许本地连接
host    all             all             127.0.0.1/32            scram-sha-256
host    all             all             ::1/128                 scram-sha-256

# 允许内网访问（根据实际网络配置）
host    all             all             192.168.1.0/24          scram-sha-256
```

重启 PostgreSQL：
```bash
sudo systemctl restart postgresql
```

### 3. 创建数据库和用户

```bash
# 切换到 postgres 用户
sudo -i -u postgres

# 创建数据库
psql -c "CREATE DATABASE bingxi_erp OWNER postgres;"

# 创建用户（请修改密码）
psql -c "CREATE USER bingxi WITH PASSWORD 'your_secure_password';"

# 授权
psql -c "GRANT ALL PRIVILEGES ON DATABASE bingxi_erp TO bingxi;"

# 退出
\q
exit
```

### 4. 执行数据库迁移

```bash
# 连接到数据库
psql -U bingxi -d bingxi_erp -h localhost

# 执行迁移脚本
\i /path/to/backend/database/migration/001_init.sql

# 验证
\dt
\q
```

---

## 后端部署

### 1. 编译后端

```bash
cd backend

# 编译 Release 版本
cargo build --release

# 编译产物位置
ls target/release/面料 ERP 后端
```

### 2. 创建部署目录

```bash
sudo mkdir -p /opt/bingxi/bin
sudo mkdir -p /etc/bingxi
sudo mkdir -p "/var/log/面料 ERP"
```

### 3. 复制文件

```bash
# 复制二进制文件
sudo cp target/release/面料ERP后端 /opt/bingxi/bin/
sudo chmod +x /opt/bingxi/bin/面料 ERP 后端

# 复制配置文件
sudo cp .env.example /etc/bingxi/.env
```

### 4. 配置环境变量

编辑 `/etc/bingxi/.env`:

```ini
# 服务器配置
SERVER__HOST=0.0.0.0
SERVER__PORT=8080

# 数据库配置
DATABASE__CONNECTION_STRING=postgres://bingxi:your_password@192.168.1.100:5432/bingxi_erp
DATABASE__MAX_CONNECTIONS=10

# 认证配置
AUTH__JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
AUTH__TOKEN_EXPIRY_HOURS=24

# gRPC 配置
GRPC__HOST=0.0.0.0
GRPC__PORT=50051

# 日志配置
LOG__LEVEL=info
LOG__DIR=/var/log/面料 ERP/

# 运行环境
ENV=production
```

### 5. 创建系统用户

```bash
sudo groupadd -r bingxi
sudo useradd -r -g bingxi -s /bin/false -d /opt/bingxi bingxi
```

### 6. 设置权限

```bash
sudo chown -R bingxi:bingxi /opt/bingxi
sudo chown -R bingxi:bingxi "/var/log/面料 ERP"
sudo chmod 750 "/var/log/面料 ERP"
```

### 7. 安装 systemd 服务

```bash
# 复制服务文件
sudo cp deploy/bingxi-backend.service /etc/systemd/system/

# 重载 systemd
sudo systemctl daemon-reload

# 启用服务
sudo systemctl enable bingxi-backend

# 启动服务
sudo systemctl start bingxi-backend

# 查看状态
sudo systemctl status bingxi-backend
```

### 8. 查看日志

```bash
# 实时查看日志
sudo journalctl -u bingxi-backend -f

# 查看最近 100 行
sudo journalctl -u bingxi-backend -n 100

# 查看特定时间的日志
sudo journalctl -u bingxi-backend --since "2026-03-15 10:00:00"
```

---

## 前端部署

### 1. 编译前端

```bash
cd frontend

# 编译 Release 版本
trunk build --release

# 编译产物在 dist/ 目录
ls dist/
```

### 2. 部署到 Nginx

```bash
# 创建部署目录
sudo mkdir -p /var/www/bingxi-frontend

# 复制静态文件
sudo cp -r dist/* /var/www/bingxi-frontend/

# 设置权限
sudo chown -R www-data:www-data /var/www/bingxi-frontend
sudo chmod -R 755 /var/www/bingxi-frontend
```

### 3. 配置 Nginx

```bash
# 复制 Nginx 配置文件
sudo cp deploy/nginx.conf /etc/nginx/sites-available/bingxi-frontend

# 启用站点
sudo ln -s /etc/nginx/sites-available/bingxi-frontend /etc/nginx/sites-enabled/

# 测试配置
sudo nginx -t

# 重新加载 Nginx
sudo systemctl reload nginx
```

### 4. 验证部署

```bash
# 检查 Nginx 状态
sudo systemctl status nginx

# 查看访问日志
sudo tail -f /var/log/nginx/bingxi_access.log
```

---

## 使用部署脚本（推荐）

### 后端部署脚本

```bash
cd /path/to/bingxi-rust

# 先编译
cd backend
cargo build --release

# 运行部署脚本
cd ..
sudo ./deploy/deploy-backend.sh
```

### 前端部署脚本

```bash
cd /path/to/bingxi-rust

# 先编译
cd frontend
trunk build --release

# 运行部署脚本
cd ..
sudo ./deploy/deploy-frontend.sh
```

---

## 验证部署

### 1. 检查服务状态

```bash
# 后端服务
sudo systemctl status bingxi-backend

# Nginx
sudo systemctl status nginx

# PostgreSQL
sudo systemctl status postgresql
```

### 2. 测试 API

```bash
# 测试登录接口
curl -X POST http://localhost:8080/api/v1/erp/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# 测试用户列表（需要 Token）
curl -X GET http://localhost:8080/api/v1/erp/users \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### 3. 访问前端

打开浏览器访问：`http://服务器IP`

默认登录账号：
- 用户名：`admin`
- 密码：`admin123`

**重要**: 首次登录后请立即修改默认密码！

---

## 常见问题

### 1. 后端服务无法启动

**检查日志**:
```bash
sudo journalctl -u bingxi-backend -n 100 --no-pager
```

**常见原因**:
- 数据库连接失败：检查 `.env` 中的数据库配置
- 端口被占用：检查 8080 端口是否被其他服务占用
- 权限问题：确保 bingxi 用户有权限访问日志目录

### 2. 前端页面无法访问

**检查 Nginx 配置**:
```bash
sudo nginx -t
sudo systemctl status nginx
```

**检查静态文件**:
```bash
ls -la /var/www/bingxi-frontend/
```

**查看 Nginx 日志**:
```bash
sudo tail -f /var/log/nginx/bingxi_error.log
```

### 3. 数据库连接失败

**检查 PostgreSQL 状态**:
```bash
sudo systemctl status postgresql
```

**检查防火墙**:
```bash
# Ubuntu
sudo ufw status
sudo ufw allow 5432/tcp

# CentOS
sudo firewall-cmd --permanent --add-port=5432/tcp
sudo firewall-cmd --reload
```

**测试数据库连接**:
```bash
psql -U bingxi -d bingxi_erp -h 192.168.1.100
```

### 4. Token 无效或过期

**解决方案**:
1. 清除浏览器缓存
2. 重新登录获取新 Token
3. 检查服务器时间是否准确

---

## 性能优化

### 1. 数据库优化

**创建索引**（迁移脚本已包含）:
```sql
-- 高频查询字段已创建索引
-- 查看索引
\di

-- 分析表
ANALYZE users;
ANALYZE inventory_stocks;
```

**定期维护**:
```sql
-- 清理死元组
VACUUM ANALYZE;

-- 重建索引（可选）
REINDEX DATABASE bingxi_erp;
```

### 2. Nginx 优化

启用 Gzip 压缩（已在配置中包含）:
```nginx
gzip on;
gzip_types text/plain text/css application/json application/javascript;
gzip_min_length 1000;
```

### 3. 系统优化

**文件描述符限制**:
```bash
# 编辑 /etc/security/limits.conf
bingxi soft nofile 65535
bingxi hard nofile 65535
```

**内核参数优化**:
```bash
# 编辑 /etc/sysctl.conf
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
```

应用配置：
```bash
sudo sysctl -p
```

---

## 备份与恢复

### 数据库备份

**备份脚本**:
```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/var/backups/bingxi"
DATE=$(date +%Y%m%d_%H%M%S)
DB_NAME="bingxi_erp"
DB_USER="postgres"

mkdir -p $BACKUP_DIR

pg_dump -U $DB_USER $DB_NAME | gzip > $BACKUP_DIR/$DB_NAME_$DATE.sql.gz

# 保留最近 7 天的备份
find $BACKUP_DIR -name "*.sql.gz" -mtime +7 -delete
```

**定时备份**（crontab）:
```bash
# 每天凌晨 2 点备份
0 2 * * * /path/to/backup.sh
```

### 数据库恢复

```bash
# 解压备份文件
gunzip /var/backups/bingxi/bingxi_erp_20260315_020000.sql.gz

# 恢复数据库
psql -U postgres -d bingxi_erp < /var/backups/bingxi/bingxi_erp_20260315_020000.sql
```

---

## 监控与告警

### 1. 系统监控

**安装监控工具**:
```bash
# 安装 htop
sudo apt install -y htop

# 安装 iotop
sudo apt install -y iotop
```

**查看资源使用**:
```bash
# CPU 和内存
htop

# 磁盘 I/O
iotop

# 磁盘空间
df -h
```

### 2. 应用监控

**查看进程状态**:
```bash
ps aux | grep 面料 ERP 后端
```

**查看网络连接**:
```bash
netstat -tlnp | grep 8080
netstat -tlnp | grep 50051
```

### 3. 日志分析

**后端日志**:
```bash
# 实时查看
sudo journalctl -u bingxi-backend -f

# 按级别过滤
sudo journalctl -u bingxi-backend -p err
```

**Nginx 日志**:
```bash
# 访问日志
sudo tail -f /var/log/nginx/bingxi_access.log

# 错误日志
sudo tail -f /var/log/nginx/bingxi_error.log
```

---

## 升级指南

### 1. 后端升级

```bash
# 停止服务
sudo systemctl stop bingxi-backend

# 备份旧版本
sudo cp /opt/bingxi/bin/面料ERP后端 /opt/bingxi/bin/面料 ERP 后端.bak

# 编译新版本
cd backend
cargo build --release

# 复制新版本
sudo cp target/release/面料 ERP 后端 /opt/bingxi/bin/

# 重启服务
sudo systemctl start bingxi-backend

# 验证
sudo systemctl status bingxi-backend
```

### 2. 前端升级

```bash
# 编译新版本
cd frontend
trunk build --release

# 备份旧版本
sudo cp -r /var/www/bingxi-frontend /var/www/bingxi-frontend.bak

# 复制新版本
sudo cp -r dist/* /var/www/bingxi-frontend/

# 重载 Nginx
sudo systemctl reload nginx
```

---

## 安全建议

### 1. 防火墙配置

```bash
# Ubuntu (UFW)
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS（如启用）
sudo ufw allow ssh
sudo ufw enable

# CentOS (firewalld)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --reload
```

### 2. SSL/TLS 配置（推荐）

使用 Let's Encrypt 免费证书：

```bash
# 安装 certbot
sudo apt install -y certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d your-domain.com

# 自动续期
sudo certbot renew --dry-run
```

### 3. 数据库安全

```sql
-- 修改默认密码
ALTER USER bingxi WITH PASSWORD 'new_secure_password';

-- 限制远程访问
-- 编辑 pg_hba.conf，只允许特定 IP 段访问
```

### 4. 定期更新

```bash
# 更新系统包
sudo apt update && sudo apt upgrade -y

# 更新 Rust
rustup update
```

---

## 联系支持

如遇到部署问题，请收集以下信息：

1. 操作系统版本：`cat /etc/os-release`
2. Rust 版本：`rustc --version`
3. PostgreSQL 版本：`psql --version`
4. Nginx 版本：`nginx -v`
5. 后端日志：`sudo journalctl -u bingxi-backend -n 200`
6. Nginx 错误日志：`sudo tail -n 100 /var/log/nginx/bingxi_error.log`

---

**文档版本**: v1.0  
**最后更新**: 2026-03-15  
**维护者**: 秉羲团队

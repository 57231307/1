# 秉羲 ERP 系统 - 部署指南

## 📋 部署方式

### 方式一：快速部署（推荐）

使用提供的快速部署脚本，敏感信息通过环境变量或交互式输入。

#### 步骤：

1. **设置环境变量（可选）**
```bash
export DEPLOY_SERVER_HOST="111.230.99.236"
export DEPLOY_SERVER_USER="root"
export DEPLOY_SERVER_PORT="22"
export DB_HOST="39.99.34.194"
export DB_PORT="5432"
export DB_NAME="bingxi"
export DB_USER="bingxi"
```

2. **运行部署脚本**
```bash
# 交互式输入密码
./quick-deploy.sh

# 或使用环境变量
export DEPLOY_SERVER_PASSWORD="your_ssh_password"
export DB_PASSWORD="your_db_password"
./quick-deploy.sh
```

### 方式二：手动部署

#### 1. 下载发布包
```bash
# 获取最新版本
VERSION=$(curl -s https://api.github.com/repos/57231307/1/releases/latest | grep '"tag_name"' | cut -d'"' -f4)

# 下载（使用加速镜像）
wget https://ghproxy.net/https://github.com/57231307/1/releases/download/${VERSION}/release-${VERSION}.tar.gz -O /tmp/release.tar.gz
```

#### 2. 上传到服务器
```bash
scp /tmp/release.tar.gz root@111.230.99.236:/tmp/
```

#### 3. 在服务器上执行
```bash
# 解压
tar -xzf /tmp/release.tar.gz -C /tmp/

# 停止服务
systemctl stop bingxi

# 备份
cp -r /opt/bingxi /opt/bingxi.backup.$(date +%Y%m%d_%H%M%S)

# 部署
mkdir -p /opt/bingxi
mv /tmp/frontend/dist /opt/bingxi/frontend
mv /tmp/server /opt/bingxi/backend
chmod +x /opt/bingxi/backend/server

# 配置环境
mkdir -p /etc/bingxi
cat > /etc/bingxi/.env << 'ENVEOF'
ENV=production
SERVER__HOST=0.0.0.0
SERVER__PORT=8082
DATABASE__HOST=39.99.34.194
DATABASE__PORT=5432
DATABASE__NAME=bingxi
DATABASE__USERNAME=bingxi
DATABASE__PASSWORD=你的数据库密码
JWT__SECRET=openssl rand -hex 32
COOKIE__SECRET=openssl rand -hex 32
ENVEOF

# 安装服务
cp /workspace/deploy/bingxi.service /etc/systemd/system/
cp /workspace/deploy/nginx.conf /etc/nginx/sites-available/bingxi
ln -sf /etc/nginx/sites-available/bingxi /etc/nginx/sites-enabled/

# 启动
systemctl daemon-reload
systemctl enable bingxi
systemctl start bingxi
systemctl restart nginx
```

## 🔧 配置文件说明

### 环境变量文件：/etc/bingxi/.env

```bash
# 运行环境
ENV=production

# 服务器配置
SERVER__HOST=0.0.0.0
SERVER__PORT=8082

# 数据库配置
DATABASE__HOST=39.99.34.194
DATABASE__PORT=5432
DATABASE__NAME=bingxi
DATABASE__USERNAME=bingxi
DATABASE__PASSWORD=你的密码

# 安全密钥（自动生成）
JWT__SECRET=随机生成
COOKIE__SECRET=随机生成
```

## 📝 管理命令

```bash
# 查看服务状态
systemctl status bingxi

# 查看日志
journalctl -u bingxi -f

# 重启服务
systemctl restart bingxi

# 停止服务
systemctl stop bingxi

# 启动服务
systemctl start bingxi

# 查看 Nginx 状态
systemctl status nginx

# 查看 Nginx 日志
tail -f /var/log/nginx/error.log
```

## 🔒 安全说明

### 敏感信息管理

- ✅ **不要**将 `.env` 文件提交到 Git
- ✅ **不要**在代码中硬编码密码
- ✅ 使用环境变量传递敏感信息
- ✅ 定期更换密钥
- ✅ 限制数据库访问 IP

### 文件权限

```bash
# 设置正确的权限
chmod 600 /etc/bingxi/.env
chown root:root /etc/bingxi/.env
chmod 755 /opt/bingxi/backend/server
```

## 🚨 故障排查

### 服务启动失败

```bash
# 查看系统日志
journalctl -u bingxi --no-pager -n 50

# 检查配置文件
cat /etc/bingxi/.env

# 测试后端
curl http://127.0.0.1:8082/health
```

### 数据库连接失败

```bash
# 测试连接
psql -h 39.99.34.194 -U bingxi -d bingxi -c "SELECT 1"

# 检查网络
telnet 39.99.34.194 5432
```

### Nginx 代理失败

```bash
# 测试配置
nginx -t

# 查看错误日志
tail -f /var/log/nginx/error.log

# 重启 Nginx
systemctl restart nginx
```

## 📊 监控

### 系统资源
```bash
# CPU 和内存
top -p $(pgrep -f bingxi)

# 磁盘使用
df -h /opt/bingxi

# 网络连接
netstat -tlnp | grep :8082
```

### 应用指标
```bash
# Prometheus 指标（如果启用）
curl http://127.0.0.1:8082/metrics
```

## 🔄 升级流程

1. 备份当前版本
2. 下载新版本
3. 停止服务
4. 替换文件
5. 启动服务
6. 健康检查

或使用 CLI 工具：
```bash
bingxi upgrade
```

## 📞 技术支持

- 项目地址：https://github.com/57231307/1
- 问题反馈：https://github.com/57231307/1/issues

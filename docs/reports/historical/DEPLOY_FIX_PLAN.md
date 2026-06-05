# 部署流程修复方案

## 一、问题全景分析

### 1.1 路径不一致问题 (6个文件, 4种路径方案)

| 文件 | 后端路径 | 前端路径 | 服务名 |
|------|----------|----------|--------|
| deploy-backend.sh | /opt/bingxi/bin | - | bingxi-backend |
| deploy-frontend.sh | - | /var/www/bingxi-frontend | - |
| bingxi.service | /opt/bingxi/backend | - | bingxi |
| bingxi-backend.service | /opt/bingxi-erp/backend | - | bingxi-backend |
| nginx.conf | - | /opt/bingxi/frontend/dist | - |
| deploy.sh | /opt/bingxi-erp/backend | /opt/bingxi-erp/frontend | bingxi-backend |
| **实际服务器** | /opt/bingxi-erp/backend | /opt/bingxi/frontend/dist | bingxi |

**结果**: 部署时文件位置混乱，服务启动失败。

### 1.2 双服务端口冲突

服务器存在两个 systemd 服务:
- bingxi.service -> /opt/bingxi/backend/server (端口 8082)
- bingxi-backend.service -> /opt/bingxi-erp/backend/server (端口 8082)

两个服务绑定同一端口，新版本无法启动。

### 1.3 config.yaml 连接字符串问题

config.yaml 中 connection_string 使用 localhost:
```yaml
database:
  connection_string: "postgres://bingxi:bingxi123@localhost:5432/bingxi"
```

环境变量只覆盖 host 字段:
```env
DATABASE__HOST=39.99.34.194
```

代码逻辑 (settings.rs:127-136): connection_string 非空时直接使用，不会从 host/port 重新生成。

**结果**: 新版本连接 localhost:5432 (无 PostgreSQL)，进入 setup 模式。

### 1.4 deploy_latest.sh 问题

1. SSH heredoc 与 sshpass 不兼容
2. 没有处理旧服务停止
3. 没有数据库迁移步骤
4. 没有配置文件更新逻辑
5. 没有健康检查验证

### 1.5 数据库迁移事务回滚

迁移脚本使用事务包裹，CREATE TABLE IF NOT EXISTS 失败导致整个事务回滚。

### 1.6 加速地址未统一

install.sh 有镜像列表，但 deploy_latest.sh 没有使用。

---

## 二、修复方案

### 2.1 统一路径标准

```
后端目录: /opt/bingxi-erp/backend/
前端目录: /opt/bingxi/frontend/dist/
配置目录: /etc/bingxi/
日志目录: /opt/bingxi-erp/backend/logs/
服务名称: bingxi-backend.service
Nginx配置: /etc/nginx/sites-available/bingxi-erp
```

### 2.2 需要修改的文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| deploy/deploy.sh | 重写 | 主部署脚本，处理全新部署和更新 |
| deploy_latest.sh | 重写 | 远程更新脚本，使用加速地址 |
| 快速部署/install.sh | 重写 | 一键安装脚本，修复 update 流程 |
| deploy/bingxi-backend.service | 保留 | 统一使用此服务文件 |
| deploy/bingxi.service | 删除 | 移除旧服务文件 |
| deploy/nginx.conf | 修复 | 统一前端路径 |
| deploy/deploy-backend.sh | 重写 | 后端部署脚本 |
| deploy/deploy-frontend.sh | 重写 | 前端部署脚本 |
| .github/workflows/ci-cd.yml | 修复 | 版本嵌入和发布包结构 |
| deploy/deploy-prepare.sh | 更新 | 本地构建流程 |

### 2.3 加速地址列表

```bash
MIRRORS=(
    "https://ghp.ci/"
    "https://gh-proxy.com/"
    "https://ghproxy.net/"
    "https://github.moeyy.xyz/"
    "https://mirror.ghproxy.com/"
    ""  # 直连作为最后退路
)
```

### 2.4 平滑升级流程

```
1. 获取最新 Release 版本号
2. 下载发布包 (使用加速地址)
3. 备份当前版本
4. 停止旧服务
5. 解压新版本
6. 更新 config.yaml (从 .env 生成正确的 connection_string)
7. 执行数据库迁移
8. 更新 systemd 服务文件
9. 更新 Nginx 配置
10. 启动新服务
11. 健康检查验证
12. 失败时自动回滚
```

### 2.5 全新部署流程

```
1. 安装依赖 (nginx, curl, jq, unzip)
2. 下载最新发布包 (使用加速地址)
3. 创建目录结构
4. 解压发布包
5. 生成 .env 配置文件
6. 生成 config.yaml
7. 执行数据库初始化
8. 安装 systemd 服务
9. 配置 Nginx
10. 启动服务
11. 安装 CLI 工具
12. 健康检查验证
```

---

## 三、加速地址使用策略

### 3.1 GitHub API 加速

```bash
# 获取最新 Release 信息
API_URL="https://api.github.com/repos/57231307/1/releases/latest"
```

### 3.2 下载加速

```bash
# 原始下载地址
DOWNLOAD_URL="https://github.com/57231307/1/releases/download/vX.X.X/bingxi-erp-X.X.X.zip"

# 镜像地址
MIRRORS=(
    "https://ghp.ci/"
    "https://gh-proxy.com/"
    "https://ghproxy.net/"
    "https://github.moeyy.xyz/"
    "https://mirror.ghproxy.com/"
    ""
)

# 构建完整 URL
for MIRROR in "${MIRRORS[@]}"; do
    FULL_URL="${MIRROR}${DOWNLOAD_URL}"
    # 尝试下载
done
```

### 3.3 CLI update 命令加速

```bash
# CLI update 命令使用加速地址下载 install.sh
UPDATE_URL="https://ghp.ci/https://raw.githubusercontent.com/57231307/1/main/快速部署/install.sh"
curl -fsSL --http1.1 --ipv4 --retry 3 "$UPDATE_URL" | sudo bash -s update
```

---

## 四、config.yaml 修复策略

### 4.1 从 .env 生成 config.yaml

部署脚本读取 /etc/bingxi/.env 中的数据库配置，动态生成 config.yaml:

```bash
# 读取 .env
source /etc/bingxi/.env

# 生成 connection_string
CONN_STR="postgres://${DATABASE__USERNAME}:${DATABASE__PASSWORD}@${DATABASE__HOST}:${DATABASE__PORT}/${DATABASE__NAME}?sslmode=disable"

# 写入 config.yaml
cat > /opt/bingxi-erp/backend/config.yaml << EOF
server:
  host: "0.0.0.0"
  port: "8082"

database:
  connection_string: "${CONN_STR}"
  host: "${DATABASE__HOST}"
  port: ${DATABASE__PORT}
  name: "${DATABASE__NAME}"
  username: "${DATABASE__USERNAME}"
  password: "${DATABASE__PASSWORD}"
  max_connections: 50
  min_connections: 5
  ssl_mode: "disable"

auth:
  jwt_secret: "${JWT_SECRET}"
  cookie_secret: "${COOKIE_SECRET}"
  token_expiry_hours: 24

grpc:
  host: "0.0.0.0"
  port: 50051

log:
  level: "info"
  dir: "/opt/bingxi-erp/backend/logs"

cors:
  allowed_origins:
    - "http://localhost"
    - "http://127.0.0.1"

redis:
  url: "${REDIS__URL}"
  max_connections: ${REDIS__MAX_CONNECTIONS}

env: "production"
EOF
```

---

## 五、数据库迁移修复策略

### 5.1 使用独立事务

每个迁移语句使用独立事务，避免一个失败导致全部回滚:

```sql
DO $$ BEGIN
    CREATE TABLE IF NOT EXISTS users (...);
EXCEPTION WHEN duplicate_table THEN
    -- 表已存在，忽略
END $$;
```

### 5.2 迁移执行方式

```bash
# 逐个执行迁移文件，每个文件独立事务
for f in migrations/*.sql; do
    PGPASSWORD=$DB_PASS psql -h $DB_HOST -U $DB_USER -d $DB_NAME -f "$f" 2>/dev/null || true
done
```

---

## 六、CI/CD 修复策略

### 6.1 版本嵌入

确保 Cargo.toml 版本号在构建前更新:

```yaml
- name: 更新 Cargo.toml 版本号
  run: |
    sed -i 's/^version = ".*"/version = "${{ steps.version.outputs.version }}"/' backend/Cargo.toml
```

### 6.2 发布包结构

```
bingxi-erp-X.X.X.zip
├── backend/
│   ├── server          # 后端可执行文件
│   ├── bingxi          # CLI 工具
│   ├── config.yaml     # 配置模板
│   └── .env.example    # 环境变量模板
├── frontend/
│   └── dist/           # 前端静态文件
├── deploy/
│   ├── deploy.sh       # 主部署脚本
│   ├── bingxi-backend.service
│   └── nginx.conf
├── database/
│   └── migration/      # 数据库迁移脚本
└── README.md
```

---

## 七、验证清单

### 7.1 全新部署验证

- [ ] 依赖安装成功
- [ ] 发包下载成功 (加速地址)
- [ ] 目录创建成功
- [ ] 配置文件生成成功
- [ ] 数据库初始化成功
- [ ] 服务启动成功
- [ ] Nginx 配置成功
- [ ] 健康检查通过
- [ ] CLI 工具可用

### 7.2 平滑升级验证

- [ ] 版本检测成功
- [ ] 发包下载成功 (加速地址)
- [ ] 备份创建成功
- [ ] 旧服务停止成功
- [ ] 新版本部署成功
- [ ] config.yaml 更新成功
- [ ] 数据库迁移成功
- [ ] 新服务启动成功
- [ ] 健康检查通过
- [ ] 回滚机制可用

---

## 八、CLI 数字快捷操作

### 8.1 设计理念

运维人员需要快速执行常用操作，数字比英文命令更直观快捷。

### 8.2 命令映射表

```
bingxi          # 显示交互菜单
bingxi 1        # 启动服务 (等同 bingxi start)
bingxi 2        # 停止服务 (等同 bingxi stop)
bingxi 3        # 重启服务 (等同 bingxi restart)
bingxi 4        # 查看状态 (等同 bingxi status)
bingxi 5        # 查看日志 (等同 bingxi logs)
bingxi 6        # 更新系统 (等同 bingxi update)
bingxi 7        # 回滚版本 (等同 bingxi rollback)
bingxi 8        # 数据库迁移 (等同 bingxi migrate)
bingxi 9        # 健康检查 (等同 bingxi health)
bingxi 0        # 查看版本 (等同 bingxi version)
```

### 8.3 交互菜单设计

```bash
$ bingxi

==========================================
  秉羲 ERP 系统管理工具 v2026.522.2144
==========================================

  [1] 启动服务        [6] 更新系统
  [2] 停止服务        [7] 回滚版本
  [3] 重启服务        [8] 数据库迁移
  [4] 查看状态        [9] 健康检查
  [5] 查看日志        [0] 查看版本

  [q] 退出

==========================================
请输入数字选择操作:
```

### 8.4 CLI 完整功能列表

| 数字 | 命令 | 说明 | 加速地址 |
|------|------|------|----------|
| 1 | start | 启动后端和 Nginx | - |
| 2 | stop | 停止后端和 Nginx | - |
| 3 | restart | 重启后端和 Nginx | - |
| 4 | status | 查看服务状态 | - |
| 5 | logs | 查看后端日志 (tail -f) | - |
| 6 | update | 从 GitHub Release 下载最新版本更新 | 使用加速地址 |
| 7 | rollback | 回滚到上一个备份版本 | - |
| 8 | migrate | 执行数据库迁移 | - |
| 9 | health | 执行健康检查 | - |
| 0 | version | 显示当前版本 | - |

### 8.5 CLI 实现代码

```bash
#!/bin/bash
# 秉羲 ERP 系统管理 CLI

VERSION_FILE="/opt/bingxi-erp/VERSION"
BACKUP_DIR="/opt/bingxi-erp/backups"
SERVICE_NAME="bingxi-backend"

# 加速地址列表
MIRRORS=(
    "https://ghp.ci/"
    "https://gh-proxy.com/"
    "https://ghproxy.net/"
    "https://github.moeyy.xyz/"
    "https://mirror.ghproxy.com/"
    ""
)

# 显示菜单
show_menu() {
    local ver=$(cat "$VERSION_FILE" 2>/dev/null || echo "unknown")
    echo ""
    echo "=========================================="
    echo "  秉羲 ERP 系统管理工具 v${ver}"
    echo "=========================================="
    echo ""
    echo "  [1] 启动服务        [6] 更新系统"
    echo "  [2] 停止服务        [7] 回滚版本"
    echo "  [3] 重启服务        [8] 数据库迁移"
    echo "  [4] 查看状态        [9] 健康检查"
    echo "  [5] 查看日志        [0] 查看版本"
    echo ""
    echo "  [q] 退出"
    echo ""
    echo "=========================================="
}

# 下载加速
download_with_mirror() {
    local url=$1
    local output=$2
    
    for MIRROR in "${MIRRORS[@]}"; do
        local full_url="${MIRROR}${url}"
        echo "尝试下载: ${full_url:0:80}..."
        if curl --http1.1 --ipv4 -L -C - --retry 3 --retry-delay 2 --connect-timeout 8 --max-time 1800 -o "$output" "$full_url" 2>/dev/null; then
            return 0
        fi
    done
    return 1
}

# 命令处理
case "$1" in
    1|start)
        sudo systemctl start $SERVICE_NAME
        sudo systemctl start nginx
        echo "服务已启动"
        ;;
    2|stop)
        sudo systemctl stop $SERVICE_NAME
        sudo systemctl stop nginx
        echo "服务已停止"
        ;;
    3|restart)
        sudo systemctl restart $SERVICE_NAME
        sudo systemctl restart nginx
        echo "服务已重启"
        ;;
    4|status)
        sudo systemctl status $SERVICE_NAME --no-pager
        echo ""
        echo "--- Nginx 状态 ---"
        sudo systemctl status nginx --no-pager | head -5
        ;;
    5|logs)
        sudo journalctl -u $SERVICE_NAME -f --no-pager
        ;;
    6|update)
        echo "开始更新..."
        UPDATE_SCRIPT="/tmp/bingxi-update.sh"
        UPDATE_URL="https://raw.githubusercontent.com/57231307/1/main/快速部署/install.sh"
        if download_with_mirror "$UPDATE_URL" "$UPDATE_SCRIPT"; then
            sudo bash "$UPDATE_SCRIPT" update
            rm -f "$UPDATE_SCRIPT"
        else
            echo "更新脚本下载失败"
            exit 1
        fi
        ;;
    7|rollback)
        if [ -d "$BACKUP_DIR" ]; then
            LATEST_BACKUP=$(ls -t "$BACKUP_DIR" | head -1)
            if [ -n "$LATEST_BACKUP" ]; then
                echo "回滚到: $LATEST_BACKUP"
                sudo systemctl stop $SERVICE_NAME
                sudo cp -r "$BACKUP_DIR/$LATEST_BACKUP/backend/"* /opt/bingxi-erp/backend/
                sudo systemctl start $SERVICE_NAME
                echo "回滚完成"
            else
                echo "没有可用的备份"
            fi
        else
            echo "备份目录不存在"
        fi
        ;;
    8|migrate)
        echo "执行数据库迁移..."
        source /etc/bingxi/.env
        for f in /opt/bingxi-erp/database/migration/*.sql; do
            if [ -f "$f" ]; then
                echo "执行: $(basename $f)"
                PGPASSWORD="$DATABASE__PASSWORD" psql -h "$DATABASE__HOST" -U "$DATABASE__USERNAME" -d "$DATABASE__NAME" -f "$f" 2>/dev/null || true
            fi
        done
        echo "迁移完成"
        ;;
    9|health)
        echo "执行健康检查..."
        curl -s http://127.0.0.1:8082/api/v1/erp/health | jq . 2>/dev/null || curl -s http://127.0.0.1:8082/api/v1/erp/health
        ;;
    0|version)
        echo "当前版本: $(cat $VERSION_FILE 2>/dev/null || echo 'unknown')"
        echo "服务状态: $(systemctl is-active $SERVICE_NAME)"
        echo "Nginx状态: $(systemctl is-active nginx)"
        ;;
    "")
        show_menu
        read -p "请输入数字选择操作: " choice
        exec "$0" "$choice"
        ;;
    *)
        echo "未知命令: $1"
        show_menu
        exit 1
        ;;
esac
```

---

## 九、执行顺序

1. 删除 deploy/bingxi.service
2. 重写 deploy/bingxi-backend.service
3. 重写 deploy/nginx.conf
4. 重写 deploy/deploy.sh
5. 重写 deploy/deploy-backend.sh
6. 重写 deploy/deploy-frontend.sh
7. 重写 deploy_latest.sh
8. 重写 快速部署/install.sh (包含 CLI 数字快捷操作)
9. 更新 deploy/deploy-prepare.sh
10. 修复 .github/workflows/ci-cd.yml
11. 测试部署流程

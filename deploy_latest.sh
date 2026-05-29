#!/bin/bash
# 秉羲 ERP 远程部署/更新脚本
# 用途：从开发机远程部署到服务器

set -e

SERVER_IP="${BINGXI_SERVER_IP:-111.230.99.236}"
SSH_USER="${BINGXI_SSH_USER:-root}"
SSH_PASS="${BINGXI_SSH_PASS}"
if [ -z "$SSH_PASS" ]; then
    echo "错误：请设置 BINGXI_SSH_PASS 环境变量"
    exit 1
fi
REPO="57231307/1"

# 加速地址
MIRRORS=(
    "https://ghp.ci/"
    "https://gh-proxy.com/"
    "https://ghproxy.net/"
    "https://github.moeyy.xyz/"
    "https://mirror.ghproxy.com/"
    ""
)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[$(date '+%H:%M:%S')]${NC} $1"; }
warn() { echo -e "${YELLOW}[$(date '+%H:%M:%S')]${NC} $1"; }
error() { echo -e "${RED}[$(date '+%H:%M:%S')]${NC} $1"; exit 1; }

# 检查依赖
check_deps() {
    if ! command -v sshpass &>/dev/null; then
        error "未安装 sshpass，请运行: apt-get install -y sshpass"
    fi
}

# SSH 命令封装
remote_exec() {
    sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 "$SSH_USER@$SERVER_IP" "$1"
}

# 获取最新版本
get_latest_version() {
    log "获取最新版本信息..."
    local version=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | jq -r '.tag_name // empty' | sed 's/^v//')
    if [ -z "$version" ]; then
        error "无法获取版本信息"
    fi
    echo "$version"
}

# 下载发布包 (带加速)
download_release() {
    local version=$1
    local download_url="https://github.com/$REPO/releases/download/v${version}/bingxi-erp-${version}.zip"

    log "下载发布包 v${version}..."

    for MIRROR in "${MIRRORS[@]}"; do
        local full_url="${MIRROR}${download_url}"
        log "尝试: ${full_url:0:80}..."

        local result=$(sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=no "$SSH_USER@$SERVER_IP" "
            curl --http1.1 --ipv4 -L -C - --retry 3 --retry-delay 2 --connect-timeout 8 --max-time 1800 -o /tmp/bingxi-erp-latest.zip '$full_url' 2>&1 && echo 'SUCCESS' || echo 'FAILED'
        " 2>&1)

        if echo "$result" | grep -q "SUCCESS"; then
            log "下载成功"
            return 0
        fi
    done

    error "所有下载源均失败"
}

# 执行远程部署
deploy_remote() {
    log "执行远程部署..."

    remote_exec "
        set -e

        # 解压发布包
        mkdir -p /tmp/bingxi-deploy
        cd /tmp/bingxi-deploy
        unzip -o /tmp/bingxi-erp-latest.zip

        # 停止并禁用旧服务
        stop_old_services() {
            systemctl stop bingxi 2>/dev/null || true
            systemctl stop bingxi-backend 2>/dev/null || true
            systemctl disable bingxi 2>/dev/null || true
            rm -f /etc/systemd/system/bingxi.service
            systemctl daemon-reload
        }
        stop_old_services
        sleep 2

        # 杀死占用端口的进程
        pid=\$(ss -tlnp | grep :8082 | grep -oP 'pid=\K[0-9]+' | head -1)
        if [ -n \"\$pid\" ]; then
            kill -9 \$pid 2>/dev/null || true
            sleep 1
        fi

        # 备份当前版本
        if [ -f /opt/bingxi-erp/backend/server ]; then
            mkdir -p /opt/bingxi-erp/backups/backup_\$(date +%Y%m%d_%H%M%S)
            cp -r /opt/bingxi-erp/backend /opt/bingxi-erp/backups/backup_\$(date +%Y%m%d_%H%M%S)/
            # 只保留最近 5 个备份
            ls -dt /opt/bingxi-erp/backups/backup_* 2>/dev/null | tail -n +6 | xargs rm -rf 2>/dev/null || true
        fi

        # 创建目录
        mkdir -p /opt/bingxi-erp/backend
        mkdir -p /opt/bingxi/frontend/dist
        mkdir -p /etc/bingxi

        # 部署后端
        cp /tmp/bingxi-deploy/backend/server /opt/bingxi-erp/backend/
        cp /tmp/bingxi-deploy/backend/bingxi /opt/bingxi-erp/backend/ 2>/dev/null || true
        chmod +x /opt/bingxi-erp/backend/server /opt/bingxi-erp/backend/bingxi 2>/dev/null || true

        # 部署前端
        rm -rf /opt/bingxi/frontend/dist/*
        cp -r /tmp/bingxi-deploy/frontend/dist/* /opt/bingxi/frontend/dist/
        chown -R www-data:www-data /opt/bingxi/frontend/dist

        # 生成 config.yaml (关键修复)
        if [ -f /etc/bingxi/.env ]; then
            source /etc/bingxi/.env
            DB_HOST=\${DATABASE__HOST:-localhost}
            DB_PORT=\${DATABASE__PORT:-5432}
            DB_NAME=\${DATABASE__NAME:-bingxi}
            DB_USER=\${DATABASE__USERNAME:-bingxi}
            DB_PASS=\${DATABASE__PASSWORD:-bingxi123}
            JWT=\${JWT_SECRET:-default_jwt_secret}
            COOKIE=\${COOKIE_SECRET:-default_cookie_secret}
            REDIS_URL=\${REDIS__URL:-redis://127.0.0.1:6379}
            REDIS_MAX=\${REDIS__MAX_CONNECTIONS:-10}
            CONN_STR=\"postgres://\${DB_USER}:\${DB_PASS}@\${DB_HOST}:\${DB_PORT}/\${DB_NAME}?sslmode=disable\"

            cat > /opt/bingxi-erp/backend/config.yaml << EOF
server:
  host: \"0.0.0.0\"
  port: \"8082\"

database:
  connection_string: \"\${CONN_STR}\"
  host: \"\${DB_HOST}\"
  port: \${DB_PORT}
  name: \"\${DB_NAME}\"
  username: \"\${DB_USER}\"
  password: \"\${DB_PASS}\"
  max_connections: 50
  min_connections: 5
  ssl_mode: \"disable\"

auth:
  jwt_secret: \"\${JWT}\"
  cookie_secret: \"\${COOKIE}\"
  token_expiry_hours: 24

grpc:
  host: \"0.0.0.0\"
  port: 50051

log:
  level: \"info\"
  dir: \"/opt/bingxi-erp/backend/logs\"

cors:
  allowed_origins:
    - \"http://localhost\"
    - \"http://127.0.0.1\"

redis:
  url: \"\${REDIS_URL}\"
  max_connections: \${REDIS_MAX}

env: \"production\"
EOF
        fi

        # 执行数据库迁移
        if [ -f /etc/bingxi/.env ]; then
            source /etc/bingxi/.env
            for f in /tmp/bingxi-deploy/database/migration/*.sql; do
                if [ -f \"\$f\" ]; then
                    PGPASSWORD=\"\$DATABASE__PASSWORD\" psql -h \"\$DATABASE__HOST\" -U \"\$DATABASE__USERNAME\" -d \"\$DATABASE__NAME\" -f \"\$f\" 2>/dev/null || true
                fi
            done
        fi

        # 安装服务
        cp /tmp/bingxi-deploy/deploy/bingxi-backend.service /etc/systemd/system/
        systemctl daemon-reload
        systemctl enable bingxi-backend

        # 配置 Nginx
        cp /tmp/bingxi-deploy/deploy/nginx.conf /etc/nginx/sites-available/bingxi-erp
        ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/
        rm -f /etc/nginx/sites-enabled/default
        nginx -t && systemctl reload nginx

        # 启动服务
        systemctl start bingxi-backend
        sleep 5

        # 安装 CLI
        cp /tmp/bingxi-deploy/backend/bingxi /usr/local/bin/bingxi 2>/dev/null || true
        chmod +x /usr/local/bin/bingxi 2>/dev/null || true

        # 保存版本号
        cp /tmp/bingxi-deploy/VERSION /opt/bingxi-erp/VERSION 2>/dev/null || true

        # 清理
        rm -rf /tmp/bingxi-deploy /tmp/bingxi-erp-latest.zip

        # 健康检查
        curl -s http://127.0.0.1:8082/api/v1/erp/health
    "
}

# 主函数
main() {
    check_deps

    echo ""
    echo "=========================================="
    echo "  秉羲 ERP 远程部署"
    echo "=========================================="
    echo ""

    # 测试连接
    log "测试服务器连接..."
    if ! remote_exec "echo '连接成功'" >/dev/null 2>&1; then
        error "无法连接到服务器 $SERVER_IP"
    fi
    log "服务器连接正常"

    # 获取最新版本
    local version=$(get_latest_version 2>/dev/null | tail -1)
    if [ -z "$version" ]; then
        error "无法获取版本信息"
    fi
    log "最新版本: v${version}"

    # 检查当前版本
    local current=$(remote_exec "cat /opt/bingxi-erp/VERSION 2>/dev/null" 2>/dev/null | tail -1 || echo "unknown")
    log "当前版本: v${current}"

    if [ "$version" = "$current" ]; then
        log "已是最新版本，无需更新"
        exit 0
    fi

    # 下载发布包
    download_release "$version"

    # 执行部署
    deploy_remote

    echo ""
    echo "=========================================="
    echo "  部署完成！"
    echo "=========================================="
    echo "  版本: v${version}"
    echo "  地址: http://${SERVER_IP}"
    echo "=========================================="
}

main

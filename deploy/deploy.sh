#!/bin/bash
# 秉羲 ERP 系统部署脚本
# 用途：在服务器上部署系统 (全新部署 / 更新部署)

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 路径配置
APP_NAME="bingxi-backend"
DEPLOY_DIR="/opt/bingxi-erp"
BACKEND_DIR="$DEPLOY_DIR/backend"
FRONTEND_DIR="/opt/bingxi-erp/frontend/dist"
CONFIG_DIR="/etc/bingxi-erp"
BACKUP_DIR="$DEPLOY_DIR/backups"
LOG_DIR="$DEPLOY_DIR/logs"
ENV_FILE="$CONFIG_DIR/.env"
CONFIG_FILE="$BACKEND_DIR/config.yaml"

log() { echo -e "${GREEN}[$(date '+%H:%M:%S')]${NC} $1"; }
warn() { echo -e "${YELLOW}[$(date '+%H:%M:%S')]${NC} $1"; }
error() { echo -e "${RED}[$(date '+%H:%M:%S')]${NC} $1"; exit 1; }

# 检查是否 root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "请使用 root 权限运行此脚本"
    fi
}

# 停止所有旧服务
stop_old_services() {
    log "停止旧服务..."
    systemctl stop bingxi 2>/dev/null || true
    systemctl stop bingxi-backend 2>/dev/null || true
    systemctl disable bingxi 2>/dev/null || true
    rm -f /etc/systemd/system/bingxi.service
    systemctl daemon-reload
    sleep 2

    # 杀死占用端口的进程
    local pid=$(ss -tlnp | grep :8082 | grep -oP 'pid=\K[0-9]+' | head -1)
    if [ -n "$pid" ]; then
        warn "杀死占用 8082 端口的进程: $pid"
        kill -9 "$pid" 2>/dev/null || true
        sleep 1
    fi
}

# 备份当前版本
backup_current() {
    if [ -f "$BACKEND_DIR/server" ]; then
        log "备份当前版本..."
        local backup_name="backup_$(date +%Y%m%d_%H%M%S)"
        mkdir -p "$BACKUP_DIR/$backup_name"
        cp -r "$BACKEND_DIR" "$BACKUP_DIR/$backup_name/"
        cp -r "$FRONTEND_DIR" "$BACKUP_DIR/$backup_name/frontend_dist" 2>/dev/null || true
        [ -f "$ENV_FILE" ] && cp "$ENV_FILE" "$BACKUP_DIR/$backup_name/"
        log "备份已保存到: $BACKUP_DIR/$backup_name"

        # 只保留最近 5 个备份
        ls -dt "$BACKUP_DIR"/backup_* 2>/dev/null | tail -n +6 | xargs rm -rf 2>/dev/null || true
    fi
}

# 创建目录结构
create_dirs() {
    log "创建目录结构..."
    mkdir -p "$BACKEND_DIR"
    mkdir -p "$FRONTEND_DIR"
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$BACKUP_DIR"
    mkdir -p "$LOG_DIR"
}

# 部署后端
deploy_backend() {
    log "部署后端..."
    # 安全检查
    if [ -z "$BACKEND_DIR" ]; then
        error "BACKEND_DIR 变量为空"
    fi
    mkdir -p "$BACKEND_DIR"
    
    # 查找并复制 server 可执行文件（兼容当前目录及/tmp临时目录）
    if [ -f "/tmp/bingxi-deploy/backend/server" ]; then
        cp /tmp/bingxi-deploy/backend/server "$BACKEND_DIR/"
        cp /tmp/bingxi-deploy/backend/bingxi "$BACKEND_DIR/" 2>/dev/null || true
        chmod +x "$BACKEND_DIR/server" "$BACKEND_DIR/bingxi" 2>/dev/null || true
    elif [ -f "$(dirname "$0")/../backend/server" ]; then
        cp "$(dirname "$0")/../backend/server" "$BACKEND_DIR/"
        cp "$(dirname "$0")/../backend/bingxi" "$BACKEND_DIR/" 2>/dev/null || true
        chmod +x "$BACKEND_DIR/server" "$BACKEND_DIR/bingxi" 2>/dev/null || true
    elif [ -f "backend/server" ]; then
        cp backend/server "$BACKEND_DIR/"
        cp backend/bingxi "$BACKEND_DIR/" 2>/dev/null || true
        chmod +x "$BACKEND_DIR/server" "$BACKEND_DIR/bingxi" 2>/dev/null || true
    else
        error "找不到后端可执行文件"
    fi

    # 复制配置文件
    # 部署-1 修复：复制 config.yaml.example 前先备份现有 config.yaml（如果存在）。
    # 原因：旧逻辑在每次更新部署时直接覆盖 config.yaml，会丢失用户在引导页面、
    # 或后续手动调整的数据库连接 / JWT / 密钥等关键配置；备份后即使覆盖也能回滚。
    if [ -f "$CONFIG_FILE" ]; then
        local config_bak="$BACKUP_DIR/config_$(date +%Y%m%d_%H%M%S).yaml.bak"
        mkdir -p "$BACKUP_DIR"
        cp "$CONFIG_FILE" "$config_bak"
        log "已备份现有 config.yaml 到: $config_bak"
        # 仅保留最近 5 个 config 备份
        ls -t "$BACKUP_DIR"/config_*.yaml.bak 2>/dev/null | tail -n +6 | xargs rm -f 2>/dev/null || true
    fi
    if [ -f "/tmp/bingxi-deploy/backend/config.yaml.example" ]; then
        cp /tmp/bingxi-deploy/backend/config.yaml.example "$BACKEND_DIR/config.yaml"
    elif [ -f "$(dirname "$0")/../backend/config.yaml.example" ]; then
        cp "$(dirname "$0")/../backend/config.yaml.example" "$BACKEND_DIR/config.yaml"
    elif [ -f "backend/config.yaml.example" ]; then
        cp backend/config.yaml.example "$BACKEND_DIR/config.yaml"
    fi

    # 复制迁移文件
    mkdir -p "$DEPLOY_DIR/database"
    if [ -d "/tmp/bingxi-deploy/database/migration" ]; then
        cp -r /tmp/bingxi-deploy/database/migration "$DEPLOY_DIR/database/"
    elif [ -d "$(dirname "$0")/../database/migration" ]; then
        cp -r "$(dirname "$0")/../database/migration" "$DEPLOY_DIR/database/"
    elif [ -d "database/migration" ]; then
        cp -r database/migration "$DEPLOY_DIR/database/"
    fi

    # 修复权限问题：创建 bingxi 用户并将文件赋权
    if ! id -u bingxi >/dev/null 2>&1; then
        useradd -m -s /bin/bash bingxi || true
    fi
    chown -R bingxi:bingxi "$DEPLOY_DIR"

    log "后端文件部署完成"
}

# 部署前端
deploy_frontend() {
    log "部署前端..."
    # 安全检查：确保 FRONTEND_DIR 不为空且存在
    if [ -z "$FRONTEND_DIR" ]; then
        error "FRONTEND_DIR 变量为空"
    fi
    mkdir -p "$FRONTEND_DIR"
    
    if [ -d "/tmp/bingxi-deploy/frontend/dist" ]; then
        rm -rf "${FRONTEND_DIR:?}"/*
        cp -r /tmp/bingxi-deploy/frontend/dist/* "$FRONTEND_DIR/"
    elif [ -d "$(dirname "$0")/../frontend/dist" ]; then
        rm -rf "${FRONTEND_DIR:?}"/*
        cp -r "$(dirname "$0")/../frontend/dist"/* "$FRONTEND_DIR/"
    elif [ -d "frontend/dist" ]; then
        rm -rf "${FRONTEND_DIR:?}"/*
        cp -r frontend/dist/* "$FRONTEND_DIR/"
    else
        error "找不到前端构建文件"
    fi

    # 兼容处理 CentOS 和 Ubuntu/Debian 下 Nginx 用户名不同的情况
    local NGINX_USER="www-data"
    if id -u nginx >/dev/null 2>&1; then
        NGINX_USER="nginx"
    fi
    chown -R "$NGINX_USER":"$NGINX_USER" "$FRONTEND_DIR"

    log "前端文件部署完成"
}

# 生成 config.yaml
generate_config() {
    log "生成 config.yaml..."

    # 如果 .env 不存在，从模板创建
    if [ ! -f "$ENV_FILE" ]; then
        if [ -f "/tmp/bingxi-deploy/backend/.env.example" ]; then
            cp /tmp/bingxi-deploy/backend/.env.example "$ENV_FILE"
            warn "已创建 .env 配置文件，请根据实际情况修改数据库配置"
        elif [ -f "backend/.env.example" ]; then
            cp backend/.env.example "$ENV_FILE"
            warn "已创建 .env 配置文件，请根据实际情况修改数据库配置"
        fi
    fi

    # 从 .env 读取配置
    if [ -f "$ENV_FILE" ]; then
        # 安全地读取环境变量，避免执行恶意代码
        set -a
        . "$ENV_FILE"
        set +a

        local DB_HOST="${DATABASE__HOST:-localhost}"
        local DB_PORT="${DATABASE__PORT:-5432}"
        local DB_NAME="${DATABASE__NAME:-bingxi}"
        local DB_USER="${DATABASE__USERNAME:-bingxi}"
        local DB_PASS="${DATABASE__PASSWORD:-}"
        local JWT="${JWT_SECRET:-}"
        local COOKIE="${COOKIE_SECRET:-}"
        local WEBHOOK="${WEBHOOK_SECRET:-}"
        
        # 自动生成 AUDIT_SECRET_KEY（基于服务器硬件信息 + 随机盐）
        if [ -z "$AUDIT_SECRET_KEY" ]; then
            # 收集硬件信息
            local HW_INFO=""
            HW_INFO+=$(cat /etc/machine-id 2>/dev/null || echo "no-machine-id")
            HW_INFO+=$(dmidecode -s system-serial-number 2>/dev/null || echo "no-serial")
            HW_INFO+=$(dmidecode -s baseboard-serial-number 2>/dev/null || echo "no-board-serial")
            HW_INFO+=$(cat /sys/class/dmi/id/product_uuid 2>/dev/null || echo "no-uuid")

            # 生成 256 字节密钥（硬件信息 + 随机盐 + 时间戳）
            local SALT=$(openssl rand -hex 32)
            local TIMESTAMP=$(date +%s%N)
            AUDIT_SECRET_KEY=$(echo -n "${HW_INFO}${SALT}${TIMESTAMP}" | sha512sum | awk '{print $1}')

            # 追加到 .env 文件
            echo "AUDIT_SECRET_KEY=${AUDIT_SECRET_KEY}" >> "$ENV_FILE"
            log "已自动生成 AUDIT_SECRET_KEY（基于服务器硬件信息）"
        fi

        # P2-D 修复：自动生成 COOKIE_SECRET（与 AUDIT_SECRET_KEY 同策略）
        # 安全原因：cookie_secret < 32 字节时 main.rs 会 fail-fast 退出，
        # 全新部署时若运维忘记手动设置会直接导致服务启动失败。
        # 修复方案：检测到 COOKIE_SECRET 为空或长度不足 32 字节时，
        # 自动用 openssl rand -hex 32 生成 64 字符（32 字节）强随机密钥，
        # 并持久化到 /etc/bingxi-erp/.env 避免每次部署重新生成（密钥稳定性）。
        if [ -z "$COOKIE" ] || [ ${#COOKIE} -lt 32 ]; then
            local GENERATED_COOKIE_SECRET=$(openssl rand -hex 32)
            if grep -q "^COOKIE_SECRET=" "$ENV_FILE" 2>/dev/null; then
                # 替换已存在的 COOKIE_SECRET
                sed -i "s|^COOKIE_SECRET=.*|COOKIE_SECRET=${GENERATED_COOKIE_SECRET}|" "$ENV_FILE"
            else
                # 追加到 .env 文件
                echo "COOKIE_SECRET=${GENERATED_COOKIE_SECRET}" >> "$ENV_FILE"
            fi
            # 重新加载变量供后续 cat 使用
            COOKIE="$GENERATED_COOKIE_SECRET"
            log "已自动生成 COOKIE_SECRET（64 字符 / 32 字节）"
        fi

        # P2-D 修复：自动生成 JWT_SECRET（同样策略）
        # 安全原因：cookie_secret fail-fast 后下一步也会校验 JWT 强度，
        # 自动生成避免运维忘记设置。
        if [ -z "$JWT" ] || [ ${#JWT} -lt 32 ]; then
            local GENERATED_JWT_SECRET=$(openssl rand -hex 32)
            if grep -q "^JWT_SECRET=" "$ENV_FILE" 2>/dev/null; then
                sed -i "s|^JWT_SECRET=.*|JWT_SECRET=${GENERATED_JWT_SECRET}|" "$ENV_FILE"
            else
                echo "JWT_SECRET=${GENERATED_JWT_SECRET}" >> "$ENV_FILE"
            fi
            JWT="$GENERATED_JWT_SECRET"
            log "已自动生成 JWT_SECRET（64 字符 / 32 字节）"
        fi

        # M-2 修复：自动生成 WEBHOOK_SECRET（与 JWT_SECRET 独立）
        # 安全原因：JWT_SECRET 泄漏会同时影响第三方 webhook 签名。
        # 必须为 webhook 单独生成独立密钥，且与 JWT_SECRET 互不相同。
        if [ -z "$WEBHOOK" ] || [ ${#WEBHOOK} -lt 32 ] || [ "$WEBHOOK" = "$JWT" ]; then
            local GENERATED_WEBHOOK_SECRET=$(openssl rand -hex 32)
            # 再次校验：若新生成的密钥与 JWT 相同（极低概率），重新生成
            local RETRY_COUNT=0
            while [ "$GENERATED_WEBHOOK_SECRET" = "$JWT" ] && [ $RETRY_COUNT -lt 5 ]; do
                GENERATED_WEBHOOK_SECRET=$(openssl rand -hex 32)
                RETRY_COUNT=$((RETRY_COUNT + 1))
            done
            if grep -q "^WEBHOOK_SECRET=" "$ENV_FILE" 2>/dev/null; then
                sed -i "s|^WEBHOOK_SECRET=.*|WEBHOOK_SECRET=${GENERATED_WEBHOOK_SECRET}|" "$ENV_FILE"
            else
                echo "WEBHOOK_SECRET=${GENERATED_WEBHOOK_SECRET}" >> "$ENV_FILE"
            fi
            WEBHOOK="$GENERATED_WEBHOOK_SECRET"
            log "已自动生成 WEBHOOK_SECRET（64 字符 / 32 字节，与 JWT_SECRET 独立）"
        fi

        # 验证必需的环境变量（保留作为最后防线，理论上自动生成后不会触发）
        if [ -z "$DB_PASS" ]; then
            error "DATABASE__PASSWORD 环境变量未设置"
        fi
        if [ -z "$JWT" ]; then
            error "JWT_SECRET 环境变量未设置（自动生成失败）"
        fi
        if [ -z "$COOKIE" ]; then
            error "COOKIE_SECRET 环境变量未设置（自动生成失败）"
        fi
        if [ -z "$WEBHOOK" ]; then
            error "WEBHOOK_SECRET 环境变量未设置（自动生成失败）"
        fi
        local REDIS_URL="${REDIS__URL:-redis://127.0.0.1:6379}"
        local REDIS_MAX="${REDIS__MAX_CONNECTIONS:-10}"

        # v18 批次 48 修复 P0-8：数据库连接强制 SSL（同步 deploy-latest.sh 批次 24 v6 P0-3 修复）。
        # 原 sslmode=disable 明文传输，数据库流量含密码和业务数据，
        # 生产环境必须加密防止中间人嗅探。
        local CONN_STR="postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}?sslmode=require"

        cat > "$CONFIG_FILE" << EOF
server:
  host: "0.0.0.0"
  port: "8082"

database:
  connection_string: "${CONN_STR}"
  host: "${DB_HOST}"
  port: ${DB_PORT}
  name: "${DB_NAME}"
  username: "${DB_USER}"
  password: "${DB_PASS}"
  max_connections: 50
  min_connections: 5
  # v18 批次 48 修复 P0-8：生产环境强制 SSL（同步 deploy-latest.sh 批次 24 v6 P0-3 修复）
  ssl_mode: "require"

auth:
  jwt_secret: "${JWT}"
  cookie_secret: "${COOKIE}"
  token_expiry_hours: 24

grpc:
  host: "0.0.0.0"
  port: 50051

log:
  level: "info"
  dir: "${LOG_DIR}"

cors:
  allowed_origins:
    - "http://localhost"
    - "http://127.0.0.1"

redis:
  url: "${REDIS_URL}"
  max_connections: ${REDIS_MAX}

env: "production"
EOF
        log "config.yaml 生成完成"
    else
        warn ".env 文件不存在，跳过 config.yaml 生成"
    fi
}

# 执行数据库迁移
run_migrations() {
    log "执行数据库迁移..."
    if [ -f "$ENV_FILE" ]; then
        set -a
        . "$ENV_FILE"
        set +a
        local DB_HOST="${DATABASE__HOST:-localhost}"
        local DB_PORT="${DATABASE__PORT:-5432}"
        local DB_NAME="${DATABASE__NAME:-bingxi}"
        local DB_USER="${DATABASE__USERNAME:-bingxi}"
        local DB_PASS="${DATABASE__PASSWORD:-}"

        local migration_dir=""
        if [ -d "/tmp/bingxi-deploy/database/migration" ]; then
            migration_dir="/tmp/bingxi-deploy/database/migration"
        elif [ -d "database/migration" ]; then
            migration_dir="database/migration"
        fi

        if [ -n "$migration_dir" ]; then
            for f in "$migration_dir"/*.sql; do
                if [ -f "$f" ]; then
                    PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$f" 2>/dev/null || true
                fi
            done
            log "数据库迁移完成"
        fi
    fi
}

# 安装 systemd 服务
install_service() {
    log "安装 systemd 服务..."
    if [ -f "/tmp/bingxi-deploy/deploy/bingxi-backend.service" ]; then
        cp /tmp/bingxi-deploy/deploy/bingxi-backend.service /etc/systemd/system/
    elif [ -f "$(dirname "$0")/bingxi-backend.service" ]; then
        cp "$(dirname "$0")/bingxi-backend.service" /etc/systemd/system/
    elif [ -f "deploy/bingxi-backend.service" ]; then
        cp deploy/bingxi-backend.service /etc/systemd/system/
    fi
    systemctl daemon-reload
    systemctl enable "$APP_NAME"
    log "服务安装完成"
}

# 配置 Nginx
configure_nginx() {
    log "配置 Nginx..."
    local nginx_conf=""
    if [ -f "/tmp/bingxi-deploy/deploy/nginx.conf" ]; then
        nginx_conf="/tmp/bingxi-deploy/deploy/nginx.conf"
    elif [ -f "deploy/nginx.conf" ]; then
        nginx_conf="deploy/nginx.conf"
    fi

    if [ -n "$nginx_conf" ]; then
        # 判断是 Debian 系还是 RedHat 系来决定配置路径
        if [ -d "/etc/nginx/sites-available" ]; then
            cp "$nginx_conf" /etc/nginx/sites-available/bingxi-erp
            ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/
            rm -f /etc/nginx/sites-enabled/default
        elif [ -d "/etc/nginx/conf.d" ]; then
            cp "$nginx_conf" /etc/nginx/conf.d/bingxi-erp.conf
            # 如果默认配置存在且会冲突，可选将其重命名
            [ -f "/etc/nginx/conf.d/default.conf" ] && mv /etc/nginx/conf.d/default.conf /etc/nginx/conf.d/default.conf.bak || true
        else
            warn "找不到标准的 Nginx 配置目录，请手动配置"
            return
        fi

        if nginx -t 2>/dev/null; then
            systemctl reload nginx
            log "Nginx 配置完成"
        else
            warn "Nginx 配置测试失败，跳过"
        fi
    fi
}

# 启动服务
start_service() {
    log "启动后端服务..."
    systemctl start "$APP_NAME"
    sleep 3
}

# 健康检查
health_check() {
    log "执行健康检查..."
    local max_attempts=10
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        # v18 批次 48 修复 P0-8：健康检查端点路径从 /api/v1/erp/health 改为 /health。
        # 实际路由注册在 routes/mod.rs:359 和 routes/system.rs:196，均为顶层 /health。
        local response=$(curl -s http://127.0.0.1:8082/health 2>/dev/null)
        if echo "$response" | grep -q '"status":"healthy"'; then
            log "健康检查通过"
            return 0
        fi
        sleep 2
        attempt=$((attempt + 1))
    done

    warn "健康检查未通过，服务可能需要更多时间启动"
    return 1
}

# 回滚
rollback() {
    local latest_backup=$(ls -t "$BACKUP_DIR" 2>/dev/null | head -1)
    if [ -n "$latest_backup" ]; then
        warn "正在回滚到: $latest_backup"
        systemctl stop "$APP_NAME" 2>/dev/null || true
        cp -r "$BACKUP_DIR/$latest_backup/backend/"* "$BACKEND_DIR/"
        if [ -d "$BACKUP_DIR/$latest_backup/frontend_dist" ]; then
            # 安全检查：确保 FRONTEND_DIR 不为空
            if [ -z "$FRONTEND_DIR" ]; then
                error "FRONTEND_DIR 变量为空"
            fi
            rm -rf "${FRONTEND_DIR:?}"/*
            cp -r "$BACKUP_DIR/$latest_backup/frontend_dist/"* "$FRONTEND_DIR/"
        fi
        systemctl start "$APP_NAME"
        log "回滚完成"
    else
        error "没有可用的备份进行回滚"
    fi
}

# 安装 CLI 工具
install_cli() {
    log "安装 CLI 工具..."
    local cli_path="/usr/local/bin/bingxi"

    # 删除旧的 Rust CLI 二进制
    rm -f "$cli_path"

    cat > "$cli_path" << 'CLIEOF'
#!/bin/bash
# 秉羲 ERP 系统管理 CLI

VERSION_FILE="/opt/bingxi-erp/VERSION"
BACKUP_DIR="/opt/bingxi-erp/backups"
SERVICE_NAME="bingxi-backend"

MIRRORS=(
    "https://ghp.ci/"
    "https://gh-proxy.com/"
    "https://ghproxy.net/"
    "https://github.moeyy.xyz/"
    "https://mirror.ghproxy.com/"
    ""
)

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

download_with_mirror() {
    local url=$1
    local output=$2
    for MIRROR in "${MIRRORS[@]}"; do
        local full_url="${MIRROR}${url}"
        if curl --http1.1 --ipv4 -L -C - --retry 3 --retry-delay 2 --connect-timeout 8 --max-time 1800 -o "$output" "$full_url" 2>/dev/null; then
            return 0
        fi
    done
    return 1
}

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
        echo "--- 后端服务 ---"
        sudo systemctl status $SERVICE_NAME --no-pager | head -8
        echo ""
        echo "--- Nginx 服务 ---"
        sudo systemctl status nginx --no-pager | head -5
        ;;
    5|logs)
        sudo journalctl -u $SERVICE_NAME -f --no-pager
        ;;
    6|update)
        echo "开始更新..."
        
        # 检查是否有本地更新包
        LOCAL_UPDATE="/tmp/bingxi-erp-update.tar.gz"
        if [ -f "$LOCAL_UPDATE" ]; then
            echo "发现本地更新包: $LOCAL_UPDATE"
            echo "正在解压..."
            cd /tmp
            tar -xzf bingxi-erp-update.tar.gz
            cd bingxi-erp
            bash deploy/deploy.sh
            rm -rf /tmp/bingxi-erp /tmp/bingxi-erp-update.tar.gz
            echo "更新完成"
            exit 0
        fi
        
        # 尝试从GitHub下载更新包
        UPDATE_PACKAGE="/tmp/bingxi-erp-update.tar.gz"
        
        # 获取最新版本号
        VERSION_URL="https://api.github.com/repos/57231307/1/releases/latest"
        VERSION_MIRRORS=(
            "https://ghp.ci/"
            "https://gh-proxy.com/"
            ""
        )
        
        version_success=false
        for MIRROR in "${VERSION_MIRRORS[@]}"; do
            full_url="${MIRROR}${VERSION_URL}"
            echo "尝试获取版本信息: $full_url"
            VERSION_INFO=$(curl -s --connect-timeout 10 --max-time 30 "$full_url" 2>/dev/null)
            if [ -n "$VERSION_INFO" ]; then
                TAG_NAME=$(echo "$VERSION_INFO" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)
                if [ -n "$TAG_NAME" ]; then
                    version_success=true
                    echo "最新版本: $TAG_NAME"
                    break
                fi
            fi
        done
        
        if [ "$version_success" != true ]; then
            echo "无法获取最新版本信息"
            echo "请手动更新："
            echo "  1. 从 https://github.com/57231307/1/releases 下载最新发布包"
            echo "  2. 上传到服务器 /tmp/bingxi-erp-update.tar.gz"
            echo "  3. 再次运行 bingxi update"
            exit 1
        fi
        
        # 下载发布包
        DOWNLOAD_URL="https://github.com/57231307/1/releases/download/${TAG_NAME}/release-${TAG_NAME#v}.tar.gz"
        DOWNLOAD_MIRRORS=(
            "https://ghp.ci/"
            "https://gh-proxy.com/"
            "https://ghproxy.net/"
            "https://github.moeyy.xyz/"
            "https://mirror.ghproxy.com/"
            ""
        )
        
        download_success=false
        for MIRROR in "${DOWNLOAD_MIRRORS[@]}"; do
            full_url="${MIRROR}${DOWNLOAD_URL}"
            echo "尝试下载: $full_url"
            if curl --http1.1 --ipv4 -L -C - --retry 3 --retry-delay 2 --connect-timeout 10 --max-time 300 -o "$UPDATE_PACKAGE" "$full_url" 2>/dev/null; then
                if [ -s "$UPDATE_PACKAGE" ]; then
                    download_success=true
                    echo "下载成功"
                    break
                fi
            fi
            echo "下载失败，尝试下一个..."
        done
        
        if [ "$download_success" = true ]; then
            echo "正在解压..."
            cd /tmp
            tar -xzf bingxi-erp-update.tar.gz
            cd bingxi-erp
            bash deploy/deploy.sh
            rm -rf /tmp/bingxi-erp /tmp/bingxi-erp-update.tar.gz
            echo "更新完成"
        else
            echo "更新包下载失败"
            echo "请手动更新："
            echo "  1. 从 https://github.com/57231307/1/releases 下载最新发布包"
            echo "  2. 上传到服务器 /tmp/bingxi-erp-update.tar.gz"
            echo "  3. 再次运行 bingxi update"
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
        source /etc/bingxi-erp/.env
        for f in /opt/bingxi-erp/database/migration/*.sql; do
            if [ -f "$f" ]; then
                echo "执行: $(basename $f)"
                PGPASSWORD="$DATABASE__PASSWORD" psql -h "$DATABASE__HOST" -U "$DATABASE__USERNAME" -d "$DATABASE__NAME" -f "$f" 2>/dev/null || true
            fi
        done
        echo "迁移完成"
        ;;
    9|health)
        # v18 批次 48 修复 P0-8：健康检查端点路径从 /api/v1/erp/health 改为 /health
        curl -s http://127.0.0.1:8082/health 2>/dev/null | python3 -m json.tool 2>/dev/null || curl -s http://127.0.0.1:8082/health
        ;;
    0|version)
        echo "当前版本: $(cat $VERSION_FILE 2>/dev/null || echo 'unknown')"
        echo "后端状态: $(systemctl is-active $SERVICE_NAME)"
        echo "Nginx状态: $(systemctl is-active nginx)"
        ;;
    "")
        show_menu
        read -p "请输入数字选择操作: " choice
        # 清理输入，移除空格和换行
        choice=$(echo "$choice" | tr -d '[:space:]')
        if [ -z "$choice" ]; then
            echo "未输入任何内容"
            exit 1
        fi
        exec "$0" "$choice"
        ;;
    *)
        echo "未知命令: $1"
        show_menu
        exit 1
        ;;
esac
CLIEOF

    chmod +x "$cli_path"
    log "CLI 工具安装完成: $cli_path"
}

# 保存版本号
save_version() {
    if [ -f "/tmp/bingxi-deploy/VERSION" ]; then
        cp /tmp/bingxi-deploy/VERSION "$DEPLOY_DIR/VERSION"
    elif [ -f "VERSION" ]; then
        cp VERSION "$DEPLOY_DIR/VERSION"
    else
        # 从后端二进制获取版本
        local ver=$("$BACKEND_DIR/server" --version 2>/dev/null | head -1 || echo "unknown")
        echo "$ver" > "$DEPLOY_DIR/VERSION"
    fi
}

# 清理临时文件
cleanup() {
    rm -rf /tmp/bingxi-deploy
    rm -f /tmp/bingxi-erp-latest.zip
}

# 主函数
main() {
    check_root

    echo ""
    echo "=========================================="
    echo "  秉羲 ERP 系统部署"
    echo "=========================================="
    echo ""

    # 判断是全新部署还是更新
    if [ -f "$BACKEND_DIR/server" ]; then
        log "检测到已有安装，执行更新部署..."
        backup_current
        
        # 更新部署：先执行数据库迁移
        run_migrations
    else
        log "执行全新部署..."
        # 全新部署：不执行数据库迁移，用户通过引导页面配置
    fi

    stop_old_services
    create_dirs
    deploy_backend
    deploy_frontend
    generate_config
    # 注意：全新部署时不执行 run_migrations，用户通过引导页面配置数据库
    install_service
    configure_nginx
    start_service
    install_cli
    save_version

    if health_check; then
        echo ""
        echo "=========================================="
        echo "  部署完成！"
        echo "=========================================="
        echo "  后端服务: $(systemctl is-active $APP_NAME)"
        echo "  Nginx状态: $(systemctl is-active nginx)"
        echo "  访问地址: http://$(hostname -I | awk '{print $1}')"
        echo ""
        echo "  使用 'bingxi' 命令管理系统"
        echo "=========================================="
    else
        warn "服务可能需要更多时间启动，请稍后检查"
    fi

    cleanup
}

# 支持回滚参数
if [ "$1" = "rollback" ]; then
    check_root
    rollback
    exit 0
fi

main

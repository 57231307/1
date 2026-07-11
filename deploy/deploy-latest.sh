#!/bin/bash
# 秉羲 ERP 远程部署/更新脚本
# 用途：从开发机远程部署到服务器
#
# ==============================================================================
# 安全提示（批次 21 修复）：推荐使用 SSH 密钥认证替代密码登录
# ==============================================================================
# 密码登录（sshpass）存在以下风险：
#   1. 密码以明文形式通过环境变量传递，可能被 /proc/<pid>/environ 读取
#   2. StrictHostKeyChecking=no 完全禁用主机密钥校验，易受中间人攻击
#
# SSH 密钥认证配置步骤：
#   1. 生成密钥对：ssh-keygen -t ed25519 -C "deploy@bingxi"
#   2. 上传公钥：ssh-copy-id -i ~/.ssh/deploy_ed25519.pub $SSH_USER@$SERVER_IP
#   3. 设置环境变量 BINGXI_SSH_KEY=~/.ssh/deploy_ed25519
#   4. 移除 BINGXI_SSH_PASS 环境变量（彻底弃用密码认证）
#
# 认证方式优先级：BINGXI_SSH_KEY（密钥，推荐）> BINGXI_SSH_PASS（密码，过渡回退）
# ==============================================================================

set -e

# 批次 24 v6 P0-1 修复：移除硬编码生产服务器 IP 默认值。
# 原默认值 111.230.99.236 暴露了真实生产 IP，攻击者扫描 GitHub 即可获取。
# 改为强制要求设置环境变量，缺失时直接退出（fail-secure）。
SERVER_IP="${BINGXI_SERVER_IP:?必须设置 BINGXI_SERVER_IP 环境变量（生产服务器 IP）}"
SSH_USER="${BINGXI_SSH_USER:-root}"
# 认证方式：优先使用 SSH 密钥（BINGXI_SSH_KEY），密码（BINGXI_SSH_PASS）作为过渡回退
SSH_KEY="${BINGXI_SSH_KEY:-}"
SSH_PASS="${BINGXI_SSH_PASS:-}"

# 认证方式选择：密钥优先，密码回退
if [[ -n "$SSH_KEY" ]]; then
    SSH_AUTH_MODE="key"
elif [[ -n "$SSH_PASS" ]]; then
    # 密码认证回退（不推荐，仅用于过渡）
    SSH_AUTH_MODE="password"
    echo "警告：使用密码认证，建议尽快迁移到 SSH 密钥认证（设置 BINGXI_SSH_KEY 环境变量）" >&2
else
    echo "错误：请设置 BINGXI_SSH_KEY（推荐，SSH 密钥认证）或 BINGXI_SSH_PASS（密码，过渡回退）环境变量" >&2
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

# 检查依赖（密钥认证无需 sshpass，密码认证才需要）
check_deps() {
    if [[ "$SSH_AUTH_MODE" == "password" ]]; then
        if ! command -v sshpass &>/dev/null; then
            error "未安装 sshpass，请运行: apt-get install -y sshpass，或改用 SSH 密钥认证（设置 BINGXI_SSH_KEY）"
        fi
    fi
}

# SSH 命令封装（密钥认证优先，密码认证回退）
# StrictHostKeyChecking=accept-new：首次连接自动接受主机密钥，后续校验防止中间人攻击
remote_exec() {
    if [[ "$SSH_AUTH_MODE" == "key" ]]; then
        # 使用 SSH 密钥认证（推荐）
        ssh -i "$SSH_KEY" -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 "$SSH_USER@$SERVER_IP" "$1"
    else
        # 密码认证回退（不推荐，仅用于过渡）
        sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 "$SSH_USER@$SERVER_IP" "$1"
    fi
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

        local result=$(remote_exec "
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

        # 创建系统用户和组（修复 bingxi 用户不存在导致服务无法启动）
        if ! id bingxi &>/dev/null; then
            groupadd -r bingxi
            useradd -r -g bingxi -s /bin/false -d /opt/bingxi-erp bingxi
        fi

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
            # 批次 24 v6 P0-2 修复：移除硬编码默认密码/密钥。
            # 原默认值 bingxi123/default_jwt_secret/default_cookie_secret 极易被字典爆破或 JWT 伪造。
            # 改为强制要求设置环境变量，缺失时直接退出（fail-secure）。
            DB_PASS=\${DATABASE__PASSWORD:?必须设置 DATABASE__PASSWORD}
            JWT=\${JWT_SECRET:?必须设置 JWT_SECRET}
            COOKIE=\${COOKIE_SECRET:?必须设置 COOKIE_SECRET}
            # 批次 273 修复：强制要求 AUDIT_SECRET_KEY（后端 settings.rs:355 校验，
            # 缺失或弱密钥会导致 systemd 启动失败）
            AUDIT=\${AUDIT_SECRET_KEY:?必须设置 AUDIT_SECRET_KEY}
            # 批次 24 v6 P0-3 修复：数据库连接强制 SSL（原 sslmode=disable 明文传输）。
            # 生产环境数据库流量含密码和业务数据，必须加密防止中间人嗅探。
            CONN_STR=\"postgres://\${DB_USER}:\${DB_PASS}@\${DB_HOST}:\${DB_PORT}/\${DB_NAME}?sslmode=require\"

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
  # 批次 24 v6 P0-3 修复：生产环境强制 SSL（原 disable 明文传输）
  ssl_mode: \"require\"

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

env: \"production\"
EOF
        fi

        # 执行数据库迁移
        # P2-5 修复（批次 84 v1 复审）：移除 2>/dev/null，保留 stderr 输出便于排错
        # 保留 || true 避免单个迁移文件失败阻塞整体部署（迁移文件可能存在顺序依赖）
        if [ -f /etc/bingxi/.env ]; then
            source /etc/bingxi/.env
            for f in /tmp/bingxi-deploy/database/migration/*.sql; do
                if [ -f \"\$f\" ]; then
                    PGPASSWORD=\"\$DATABASE__PASSWORD\" psql -h \"\$DATABASE__HOST\" -U \"\$DATABASE__USERNAME\" -d \"\$DATABASE__NAME\" -f \"\$f\" || echo \"::warning::迁移文件 \$f 执行失败（继续执行后续迁移）\"
                fi
            done
        fi

        # 安装服务
        cp /tmp/bingxi-deploy/deploy/bingxi-backend.service /etc/systemd/system/
        systemctl daemon-reload
        systemctl enable bingxi-backend

        # 设置目录权限（bingxi 用户需要读取 .env 和写入日志）
        chown -R bingxi:bingxi /opt/bingxi-erp
        chown -R bingxi:bingxi /etc/bingxi
        chmod 750 /opt/bingxi-erp/backend
        chmod 640 /etc/bingxi/.env 2>/dev/null || true

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
        # 批次 24 v6 P0-4 修复：健康检查端点路径从 /api/v1/erp/health 改为 /health。
        # 原路径返回 404，运维误以为部署成功但实际无法判断服务健康状态。
        # 实际路由注册在 routes/mod.rs:359 和 routes/system.rs:196，均为顶层 /health。
        curl -s http://127.0.0.1:8082/health
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

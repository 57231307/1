#!/bin/bash
# 秉羲 ERP 系统 - 一键安装与管理脚本
# 使用方法: curl -fsSL --http1.1 --retry 3 <install.sh url> | sudo bash -s {install|update|start|stop|status|restart}

set -e

REPO="57231307/1"
DEPLOY_DIR="/opt/bingxi-erp"
BACKEND_DIR="$DEPLOY_DIR/backend"
FRONTEND_DIR="/opt/bingxi/frontend/dist"
CONFIG_DIR="/etc/bingxi"
CLI_PATH="/usr/local/bin/bingxi"

# 加速地址
MIRRORS=(
    "https://ghp.ci/"
    "https://gh-proxy.com/"
    "https://ghproxy.net/"
    "https://github.moeyy.xyz/"
    "https://mirror.ghproxy.com/"
    ""
)

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[Bingxi]${NC} $1"; }
warn() { echo -e "${YELLOW}[Bingxi]${NC} $1"; }
error() { echo -e "${RED}[Error]${NC} $1"; exit 1; }

check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "请使用 root 权限 (sudo) 运行此脚本"
    fi
}

install_deps() {
    log "安装依赖..."
    if command -v apt-get >/dev/null; then
        apt-get update -y >/dev/null
        apt-get install -y curl jq unzip tar nginx postgresql-client >/dev/null
    elif command -v yum >/dev/null; then
        yum install -y curl jq unzip tar nginx postgresql >/dev/null
    fi
}

# 带加速的下载
download_with_mirror() {
    local url=$1
    local output=$2

    for MIRROR in "${MIRRORS[@]}"; do
        local full_url="${MIRROR}${url}"
        log "尝试下载: ${full_url:0:80}..."
        if curl --http1.1 --ipv4 -L -C - --retry 5 --retry-delay 2 --connect-timeout 8 --max-time 1800 -o "$output" "$full_url" 2>/dev/null; then
            return 0
        fi
    done
    return 1
}

download_latest() {
    log "获取最新版本..."
    local download_url=$(curl -s --http1.1 "https://api.github.com/repos/$REPO/releases/latest" | jq -r '.assets[] | select(.name | endswith(".zip")) | .browser_download_url' | head -n 1)

    if [ -z "$download_url" ] || [ "$download_url" == "null" ]; then
        error "无法获取最新版本信息"
    fi

    if ! download_with_mirror "$download_url" "/tmp/bingxi-erp-latest.zip"; then
        error "所有下载源均失败"
    fi
    log "下载完成"
}

run_deploy() {
    log "执行部署..."
    mkdir -p /tmp/bingxi-deploy
    cd /tmp/bingxi-deploy
    unzip -o /tmp/bingxi-erp-latest.zip

    if [ -f "deploy/deploy.sh" ]; then
        chmod +x deploy/deploy.sh
        ./deploy/deploy.sh
    else
        error "发布包中未找到 deploy/deploy.sh"
    fi

    rm -rf /tmp/bingxi-deploy /tmp/bingxi-erp-latest.zip
}

setup_cli() {
    log "安装 CLI 工具..."
    cat > "$CLI_PATH" << 'CLIEOF'
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
        curl -s http://127.0.0.1:8082/api/v1/erp/health 2>/dev/null | python3 -m json.tool 2>/dev/null || curl -s http://127.0.0.1:8082/api/v1/erp/health
        ;;
    0|version)
        echo "当前版本: $(cat $VERSION_FILE 2>/dev/null || echo 'unknown')"
        echo "后端状态: $(systemctl is-active $SERVICE_NAME)"
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
CLIEOF

    chmod +x "$CLI_PATH"
    log "CLI 工具安装完成"
}

install() {
    check_root
    log "开始全新安装..."
    install_deps
    download_latest
    run_deploy
    setup_cli

    echo ""
    echo "=========================================="
    echo "  安装成功！"
    echo "=========================================="
    echo "  使用 'bingxi' 命令管理系统"
    echo "  例如: bingxi 4 (查看状态)"
    echo "=========================================="
}

update() {
    check_root
    log "开始更新..."
    install_deps
    download_latest
    run_deploy
    setup_cli

    echo ""
    echo "=========================================="
    echo "  更新成功！"
    echo "=========================================="
}

case "$1" in
    install) install ;;
    update) update ;;
    start) sudo systemctl start bingxi-backend; sudo systemctl start nginx ;;
    stop) sudo systemctl stop bingxi-backend; sudo systemctl stop nginx ;;
    restart) sudo systemctl restart bingxi-backend; sudo systemctl restart nginx ;;
    status) sudo systemctl status bingxi-backend --no-pager ;;
    *)
        echo "用法: $0 {install|update|start|stop|restart|status}"
        ;;
esac

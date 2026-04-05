#!/bin/bash
# 秉羲ERP系统 - 一键安装与管理脚本
# 使用方法: curl -fsSL https://raw.githubusercontent.com/57231307/1/main/install.sh | sudo bash -s {install|update|start|stop|status}

set -e

REPO="57231307/1"
DEPLOY_DIR="/opt/bingxi-erp"
CLI_PATH="/usr/local/bin/bingxi"

# 颜色
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[Bingxi]${NC} $1"
}
error() {
    echo -e "${RED}[Error]${NC} $1"
    exit 1
}

check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "请使用 root 权限 (sudo) 运行此脚本"
    fi
}

install_deps() {
    log "安装必要依赖 (curl, jq, unzip, nginx)..."
    if command -v apt-get >/dev/null; then
        apt-get update -y >/dev/null
        apt-get install -y curl jq unzip tar systemd nginx >/dev/null
    elif command -v yum >/dev/null; then
        yum install -y curl jq unzip tar systemd nginx >/dev/null
    else
        log "警告: 未检测到apt或yum，假定依赖已手动安装"
    fi
}

download_latest() {
    log "获取最新版本信息..."
    # 查找最新 release 中的 zip 资产
    DOWNLOAD_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | jq -r '.assets[] | select(.name | endswith(".zip")) | .browser_download_url' | head -n 1)
    
    if [ -z "$DOWNLOAD_URL" ] || [ "$DOWNLOAD_URL" == "null" ]; then
        error "无法找到最新的 .zip 发布包"
    fi
    
    log "下载最新发布包: $DOWNLOAD_URL"
    curl -L -o /tmp/bingxi-erp-latest.zip "$DOWNLOAD_URL"
}

setup_cli() {
    log "配置命令行工具 (bingxi)..."
    cat << 'CLIEOF' > $CLI_PATH
#!/bin/bash
# 秉羲系统管理 CLI

case "$1" in
    start)
        sudo systemctl start bingxi-backend
        sudo systemctl start nginx
        echo "服务已启动"
        ;;
    stop)
        sudo systemctl stop bingxi-backend
        sudo systemctl stop nginx
        echo "服务已停止"
        ;;
    restart)
        sudo systemctl restart bingxi-backend
        sudo systemctl restart nginx
        echo "服务已重启"
        ;;
    status)
        sudo systemctl status bingxi-backend --no-pager
        ;;
    update)
        curl -fsSL https://raw.githubusercontent.com/57231307/1/main/install.sh | sudo bash -s update
        ;;
    *)
        echo "秉羲管理系统 CLI 工具"
        echo "用法: bingxi {start|stop|restart|status|update}"
        ;;
esac
CLIEOF
    chmod +x $CLI_PATH
}

run_deploy_script() {
    log "解压发布包以执行安装..."
    mkdir -p /tmp/bingxi-deploy
    mv /tmp/bingxi-erp-latest.zip /tmp/bingxi-deploy/
    cd /tmp/bingxi-deploy
    
    # 我们解压出 deploy 脚本
    unzip -q -o bingxi-erp-latest.zip

    if [ -f "deploy/deploy.sh" ]; then
        chmod +x deploy/deploy.sh
        # 执行内置部署脚本，该脚本会自动处理环境变量、systemd 配置、nginx 配置以及保活设置
        ./deploy/deploy.sh
    else
        error "发布包中未找到 deploy/deploy.sh，无法继续安装。"
    fi
    
    # 清理临时目录
    rm -rf /tmp/bingxi-deploy
}

install() {
    check_root
    log "开始全新安装 秉羲ERP 系统..."
    install_deps
    download_latest
    run_deploy_script
    setup_cli
    
    log "==========================================="
    log "安装成功！"
    log "您可以随时使用 'bingxi' 命令来管理系统："
    log "  - bingxi start   : 启动系统"
    log "  - bingxi stop    : 停止系统"
    log "  - bingxi status  : 查看状态"
    log "  - bingxi update  : 一键更新到最新版"
    log "==========================================="
}

update() {
    check_root
    log "开始在线更新 秉羲ERP 系统..."
    install_deps
    download_latest
    run_deploy_script
    log "==========================================="
    log "系统更新成功，并已自动重启服务！"
    log "==========================================="
}

case "$1" in
    install) install ;;
    update) update ;;
    start) sudo systemctl start bingxi-backend; sudo systemctl start nginx ;;
    stop) sudo systemctl stop bingxi-backend; sudo systemctl stop nginx ;;
    status) sudo systemctl status bingxi-backend --no-pager ;;
    restart) sudo systemctl restart bingxi-backend; sudo systemctl restart nginx ;;
    *) 
        echo "秉羲管理系统 - 一键管理脚本"
        echo "使用方法: $0 {install|update|start|stop|status|restart}"
        ;;
esac

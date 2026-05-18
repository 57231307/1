#!/bin/bash
# 从 GitHub 拉取最新构建并部署到服务器

set -e

SERVER_IP="111.230.99.236"
SSH_USER="root"
SSH_PASS="Txx19960917"

echo "=========================================="
echo "   从 GitHub 拉取最新构建并部署"
echo "=========================================="

# 检查 sshpass 是否安装
if ! command -v sshpass &> /dev/null; then
    echo "错误：未安装 sshpass，请先安装：apt-get install -y sshpass"
    exit 1
fi

# 检查是否可以使用 sshpass
SSH_CMD="sshpass -p \"${SSH_PASS}\" ssh -o StrictHostKeyChecking=no"

echo "连接到服务器 ${SERVER_IP}..."

# 在服务器上执行部署
$SSH_CMD ${SSH_USER}@${SERVER_IP} << 'EOF'
    set -e
    
    echo "=========================================="
    echo "   开始部署最新版本"
    echo "=========================================="
    
    cd /tmp
    
    # 下载最新构建
    echo "下载最新构建包..."
    rm -f bingxi-erp-latest.zip
    
    MIRRORS=(
        "https://ghp.ci/"
        "https://gh-proxy.com/"
        "https://ghproxy.net/"
        ""
    )
    
    LATEST_URL=$(curl -s "https://api.github.com/repos/57231307/1/releases/latest" | jq -r '.assets[] | select(.name | endswith(".zip")) | .browser_download_url' | head -n 1)
    
    if [ -z "$LATEST_URL" ] || [ "$LATEST_URL" == "null" ]; then
        echo "无法获取最新版本信息"
        exit 1
    fi
    
    echo "最新版本下载地址：$LATEST_URL"
    
    for MIRROR in "${MIRRORS[@]}"; do
        if [ -n "$MIRROR" ]; then
            REAL_URL="${MIRROR}${LATEST_URL}"
            echo "尝试通过镜像下载：${REAL_URL:0:80}..."
        else
            REAL_URL="$LATEST_URL"
            echo "尝试直连下载..."
        fi
        
        if curl --http1.1 --ipv4 -L -C - --retry 3 --retry-delay 2 -o bingxi-erp-latest.zip "$REAL_URL" 2>/dev/null; then
            echo "下载成功！"
            break
        fi
    done
    
    if [ ! -f "bingxi-erp-latest.zip" ]; then
        echo "下载失败，请检查网络"
        exit 1
    fi
    
    # 使用 bingxi 命令更新
    echo "执行系统更新..."
    if command -v bingxi &> /dev/null; then
        bingxi update
    else
        echo "CLI 工具未安装，使用手动部署..."
        # 手动部署逻辑
        mkdir -p /tmp/bingxi-deploy
        unzip -o bingxi-erp-latest.zip -d /tmp/bingxi-deploy
        cd /tmp/bingxi-deploy
        
        if [ -f "deploy/deploy-backend.sh" ]; then
            chmod +x deploy/deploy-backend.sh
            ./deploy/deploy-backend.sh
        fi
        
        if [ -f "deploy/deploy-frontend.sh" ]; then
            chmod +x deploy/deploy-frontend.sh
            ./deploy/deploy-frontend.sh
        fi
    fi
    
    # 清理临时文件
    rm -rf /tmp/bingxi-deploy
    rm -f bingxi-erp-latest.zip
    
    echo "=========================================="
    echo "   部署完成！"
    echo "=========================================="
    echo "服务状态："
    systemctl status bingxi-backend --no-pager | head -10
    echo ""
    echo "API 健康检查："
    curl -s http://localhost:8082/api/v1/erp/health | jq . || true
EOF

echo ""
echo "=========================================="
echo "   部署脚本执行完成"
echo "=========================================="
echo ""
echo "请手动测试 API："
echo "  curl http://${SERVER_IP}/api/v1/erp/health"
echo ""
echo "或运行全面测试脚本："
echo "  ./comprehensive_test.sh"

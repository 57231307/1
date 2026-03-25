#!/bin/bash
# 秉羲管理系统 - 前端部署脚本
# 使用方式：sudo ./deploy-frontend.sh

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 配置变量
APP_NAME="bingxi-frontend"
INSTALL_DIR="/var/www/bingxi-frontend"
NGINX_CONFIG="/etc/nginx/sites-available/bingxi-frontend"
NGINX_ENABLED="/etc/nginx/sites-enabled/bingxi-frontend"

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}秉羲管理系统前端部署脚本${NC}"
echo -e "${GREEN}=========================================${NC}"

# 检查是否以 root 运行
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}错误：请使用 sudo 运行此脚本${NC}"
    exit 1
fi

# 1. 创建部署目录
echo -e "${YELLOW}[1/5] 创建部署目录...${NC}"
mkdir -p "$INSTALL_DIR"
echo -e "${GREEN}✓ 目录创建完成${NC}"

# 2. 复制前端静态文件
echo -e "${YELLOW}[2/5] 复制前端静态文件...${NC}"
if [ -d "dist" ]; then
    cp -r dist/* "$INSTALL_DIR/"
    echo -e "${GREEN}✓ 静态文件复制完成${NC}"
else
    echo -e "${RED}错误：未找到 dist 目录，请先运行 trunk build${NC}"
    exit 1
fi

# 3. 设置目录权限
echo -e "${YELLOW}[3/5] 设置目录权限...${NC}"
chown -R www-data:www-data "$INSTALL_DIR"
chmod -R 755 "$INSTALL_DIR"
echo -e "${GREEN}✓ 权限设置完成${NC}"

# 4. 配置 Nginx
echo -e "${YELLOW}[4/5] 配置 Nginx...${NC}"
cp deploy/nginx.conf "$NGINX_CONFIG"
ln -sf "$NGINX_CONFIG" "$NGINX_ENABLED"

# 测试 Nginx 配置
if nginx -t; then
    echo -e "${GREEN}✓ Nginx 配置测试通过${NC}"
    systemctl reload nginx
    echo -e "${GREEN}✓ Nginx 已重新加载${NC}"
else
    echo -e "${RED}✗ Nginx 配置测试失败${NC}"
    exit 1
fi

# 5. 检查服务状态
echo -e "${YELLOW}[5/5] 检查服务状态...${NC}"
if systemctl is-active --quiet nginx; then
    echo -e "${GREEN}✓ Nginx 运行正常${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}部署完成！${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo -e "访问地址：http://localhost"
    echo -e "静态文件目录：$INSTALL_DIR"
    echo -e "Nginx 配置：$NGINX_CONFIG"
    echo -e ""
    echo -e "常用命令："
    echo -e "  查看状态：sudo systemctl status nginx"
    echo -e "  重启服务：sudo systemctl restart nginx"
    echo -e "  查看日志：sudo tail -f /var/log/nginx/bingxi_access.log"
else
    echo -e "${RED}✗ Nginx 未运行${NC}"
    echo -e "启动命令：sudo systemctl start nginx"
    exit 1
fi

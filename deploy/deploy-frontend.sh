#!/bin/bash
# 面料管理 - 前端部署脚本 (Vue 3 + Element Plus)
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
echo -e "${GREEN}面料管理前端部署脚本 (Vue 3)${NC}"
echo -e "${GREEN}=========================================${NC}"

# 检查是否以 root 运行
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}错误：请使用 sudo 运行此脚本${NC}"
    exit 1
fi

# 1. 安装依赖并构建
echo -e "${YELLOW}[1/4] 安装前端依赖并构建...${NC}"
cd frontend
npm install --production=false
npm run build
echo -e "${GREEN}✓ 前端构建完成${NC}"
cd ..

# 2. 创建部署目录并复制文件
echo -e "${YELLOW}[2/4] 部署前端静态文件...${NC}"
mkdir -p "$INSTALL_DIR"
if [ -d "frontend/dist" ]; then
    rm -rf "$INSTALL_DIR"/*
    cp -r frontend/dist/* "$INSTALL_DIR/"
    echo -e "${GREEN}✓ 静态文件部署完成${NC}"
else
    echo -e "${RED}错误：未找到 dist 目录，请先运行 npm run build${NC}"
    exit 1
fi

# 3. 设置目录权限
echo -e "${YELLOW}[3/4] 设置目录权限...${NC}"
chown -R www-data:www-data "$INSTALL_DIR"
chmod -R 755 "$INSTALL_DIR"
echo -e "${GREEN}✓ 权限设置完成${NC}"

# 4. 配置 Nginx 并重启
echo -e "${YELLOW}[4/4] 配置 Nginx...${NC}"
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
echo -e "  查看日志：sudo tail -f /var/log/nginx/access.log"

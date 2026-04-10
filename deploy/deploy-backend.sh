#!/bin/bash
# 秉羲面料管理 - 后端服务部署脚本
# 使用方式：sudo ./deploy-backend.sh

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 配置变量
APP_NAME="bingxi-backend"
APP_USER="bingxi"
APP_GROUP="bingxi"
INSTALL_DIR="/opt/bingxi"
BIN_DIR="$INSTALL_DIR/bin"
CONFIG_DIR="/etc/bingxi"
LOG_DIR="/var/log/面料 ERP"
SERVICE_FILE="bingxi-backend.service"

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}秉羲面料管理后端服务部署脚本${NC}"
echo -e "${GREEN}=========================================${NC}"

# 检查是否以 root 运行
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}错误：请使用 sudo 运行此脚本${NC}"
    exit 1
fi

# 1. 创建用户和组
echo -e "${YELLOW}[1/8] 创建系统用户和组...${NC}"
if ! id "$APP_USER" &>/dev/null; then
    groupadd -r "$APP_GROUP"
    useradd -r -g "$APP_GROUP" -s /bin/false -d "$INSTALL_DIR" "$APP_USER"
    echo -e "${GREEN}✓ 用户和组创建成功${NC}"
else
    echo -e "${GREEN}✓ 用户已存在${NC}"
fi

# 2. 创建目录
echo -e "${YELLOW}[2/8] 创建安装目录...${NC}"
mkdir -p "$BIN_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$LOG_DIR"
echo -e "${GREEN}✓ 目录创建完成${NC}"

# 3. 复制二进制文件
echo -e "${YELLOW}[3/8] 复制二进制文件...${NC}"
if [ -f "target/release/面料 ERP 后端" ]; then
    cp target/release/面料ERP后端 "$BIN_DIR/"
    chmod +x "$BIN_DIR/面料 ERP 后端"
    echo -e "${GREEN}✓ 二进制文件复制完成${NC}"
else
    echo -e "${RED}错误：未找到二进制文件，请先运行 cargo build --release${NC}"
    exit 1
fi

# 4. 复制配置文件
echo -e "${YELLOW}[4/8] 复制配置文件...${NC}"
if [ -f ".env.example" ]; then
    if [ ! -f "$CONFIG_DIR/.env" ]; then
        cp .env.example "$CONFIG_DIR/.env"
        echo -e "${GREEN}✓ 配置文件已复制，请编辑 $CONFIG_DIR/.env 配置数据库等信息${NC}"
    else
        echo -e "${GREEN}✓ 配置文件已存在${NC}"
    fi
fi

# 5. 复制 systemd 服务文件
echo -e "${YELLOW}[5/8] 安装 systemd 服务...${NC}"
cp deploy/$SERVICE_FILE /etc/systemd/system/
systemctl daemon-reload
echo -e "${GREEN}✓ systemd 服务安装完成${NC}"

# 6. 设置目录权限
echo -e "${YELLOW}[6/8] 设置目录权限...${NC}"
chown -R "$APP_USER:$APP_GROUP" "$INSTALL_DIR"
chown -R "$APP_USER:$APP_GROUP" "$LOG_DIR"
chmod 750 "$LOG_DIR"
echo -e "${GREEN}✓ 权限设置完成${NC}"

# 7. 启用并启动服务
echo -e "${YELLOW}[7/8] 启用并启动服务...${NC}"
systemctl enable $APP_NAME
systemctl start $APP_NAME
echo -e "${GREEN}✓ 服务已启动${NC}"

# 8. 检查服务状态
echo -e "${YELLOW}[8/8] 检查服务状态...${NC}"
sleep 2
if systemctl is-active --quiet $APP_NAME; then
    echo -e "${GREEN}✓ 服务运行正常${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}部署完成！${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo -e "服务名称：$APP_NAME"
    echo -e "安装目录：$INSTALL_DIR"
    echo -e "日志目录：$LOG_DIR"
    echo -e "配置文件：$CONFIG_DIR/.env"
    echo -e ""
    echo -e "常用命令："
    echo -e "  查看状态：sudo systemctl status $APP_NAME"
    echo -e "  启动服务：sudo systemctl start $APP_NAME"
    echo -e "  停止服务：sudo systemctl stop $APP_NAME"
    echo -e "  重启服务：sudo systemctl restart $APP_NAME"
    echo -e "  查看日志：sudo journalctl -u $APP_NAME -f"
else
    echo -e "${RED}✗ 服务启动失败，请检查日志${NC}"
    echo -e "查看日志：sudo journalctl -u $APP_NAME -n 50"
    exit 1
fi

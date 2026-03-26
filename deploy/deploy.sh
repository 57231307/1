#!/bin/bash
# 秉羲ERP系统部署脚本
# 用途：在服务器上部署系统

set -e

APP_NAME="bingxi-erp"
DEPLOY_DIR="/opt/bingxi-erp"
BACKUP_DIR="/opt/bingxi-erp/backups"
LOG_DIR="/var/log/bingxi-erp"

echo "========================================"
echo "  秉羲ERP系统部署脚本"
echo "========================================"

# 创建目录
echo "[1/6] 创建目录..."
sudo mkdir -p $DEPLOY_DIR
sudo mkdir -p $BACKUP_DIR
sudo mkdir -p $LOG_DIR

# 备份当前版本
if [ -f "$DEPLOY_DIR/backend/bingxi_backend" ]; then
    echo "[2/6] 备份当前版本..."
    BACKUP_NAME="backup_$(date +%Y%m%d_%H%M%S)"
    sudo mkdir -p $BACKUP_DIR/$BACKUP_NAME
    sudo cp -r $DEPLOY_DIR/backend $BACKUP_DIR/$BACKUP_NAME/
    sudo cp -r $DEPLOY_DIR/frontend $BACKUP_DIR/$BACKUP_NAME/
    echo "备份已保存到: $BACKUP_DIR/$BACKUP_NAME"
else
    echo "[2/6] 无需备份（首次部署）"
fi

# 解压发布包
echo "[3/6] 解压发布包..."
if [ -f "bingxi-erp-*.tar.gz" ]; then
    sudo tar -xzvf bingxi-erp-*.tar.gz -C $DEPLOY_DIR
else
    echo "错误：找不到发布包"
    exit 1
fi

# 设置权限
echo "[4/6] 设置权限..."
sudo chmod +x $DEPLOY_DIR/backend/bingxi_backend
sudo chown -R www-data:www-data $DEPLOY_DIR
sudo chown -R www-data:www-data $LOG_DIR

# 配置 systemd 服务
echo "[5/6] 配置系统服务..."
sudo cp $DEPLOY_DIR/deploy/bingxi-backend.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable bingxi-backend
sudo systemctl restart bingxi-backend

# 配置 Nginx
echo "[6/6] 配置 Nginx..."
sudo cp $DEPLOY_DIR/deploy/nginx.conf /etc/nginx/sites-available/bingxi-erp
sudo ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx

echo "========================================"
echo "  部署完成！"
echo "========================================"
echo "后端服务状态: $(systemctl is-active bingxi-backend)"
echo "访问地址: http://$(hostname -I | awk '{print $1}')"
echo "日志目录: $LOG_DIR"

#!/bin/bash
# 秉羲ERP系统部署脚本
# 用途：在服务器上部署系统

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

APP_NAME="bingxi-erp"
DEPLOY_DIR="/opt/bingxi-erp"
BACKUP_DIR="/opt/bingxi-erp/backups"
LOG_DIR="/var/log/bingxi-erp"
CONFIG_DIR="/etc/bingxi"

# 日志文件
DEPLOY_LOG="/var/log/bingxi-erp/deploy.log"

# 创建日志目录和文件
mkdir -p $(dirname $DEPLOY_LOG)
touch $DEPLOY_LOG

# 日志函数
log() {
    local level=$1
    local message=$2
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    
    case $level in
        "INFO")
            echo -e "${BLUE}[INFO]${NC} $timestamp - $message"
            echo "[INFO] $timestamp - $message" >> $DEPLOY_LOG
            ;;
        "SUCCESS")
            echo -e "${GREEN}[SUCCESS]${NC} $timestamp - $message"
            echo "[SUCCESS] $timestamp - $message" >> $DEPLOY_LOG
            ;;
        "WARNING")
            echo -e "${YELLOW}[WARNING]${NC} $timestamp - $message"
            echo "[WARNING] $timestamp - $message" >> $DEPLOY_LOG
            ;;
        "ERROR")
            echo -e "${RED}[ERROR]${NC} $timestamp - $message"
            echo "[ERROR] $timestamp - $message" >> $DEPLOY_LOG
            ;;
        *)
            echo -e "[UNKNOWN] $timestamp - $message"
            echo "[UNKNOWN] $timestamp - $message" >> $DEPLOY_LOG
            ;;
    esac
}

# 检查命令是否存在
check_command() {
    local cmd=$1
    if ! command -v $cmd &> /dev/null; then
        log "ERROR" "命令 $cmd 不存在，请安装"
        exit 1
    fi
}

# 检查服务状态
check_service() {
    local service=$1
    local max_attempts=30
    local attempt=1
    
    log "INFO" "检查服务 $service 状态..."
    
    while [ $attempt -le $max_attempts ]; do
        if systemctl is-active --quiet $service; then
            log "SUCCESS" "服务 $service 运行正常"
            return 0
        fi
        
        log "INFO" "服务 $service 正在启动中... (尝试 $attempt/$max_attempts)"
        sleep 2
        attempt=$((attempt + 1))
    done
    
    log "ERROR" "服务 $service 启动失败"
    return 1
}

# 主部署函数
main() {
    log "INFO" "========================================"
    log "INFO" "  秉羲ERP系统部署脚本"
    log "INFO" "========================================"
    
    # 检查必要命令
    check_command "systemctl"
    check_command "nginx"
    check_command "tar"
    
    # 创建目录
    log "INFO" "[1/8] 创建目录..."
    mkdir -p $DEPLOY_DIR
    mkdir -p $BACKUP_DIR
    mkdir -p $LOG_DIR
    mkdir -p $CONFIG_DIR
    log "SUCCESS" "目录创建完成"
    
    # 备份当前版本
    if [ -f "$DEPLOY_DIR/backend/bingxi_backend" ]; then
        log "INFO" "[2/8] 备份当前版本..."
        BACKUP_NAME="backup_$(date +%Y%m%d_%H%M%S)"
        mkdir -p $BACKUP_DIR/$BACKUP_NAME
        cp -r $DEPLOY_DIR/backend $BACKUP_DIR/$BACKUP_NAME/
        cp -r $DEPLOY_DIR/frontend $BACKUP_DIR/$BACKUP_NAME/
        if [ -f "$CONFIG_DIR/.env" ]; then
            cp $CONFIG_DIR/.env $BACKUP_DIR/$BACKUP_NAME/
        fi
        log "SUCCESS" "备份已保存到: $BACKUP_DIR/$BACKUP_NAME"
    else
        log "INFO" "[2/8] 无需备份（首次部署）"
    fi
    
    # 解压发布包
    log "INFO" "[3/8] 解压发布包..."
    if [ -f "bingxi-erp-*.tar.gz" ]; then
        tar -xzvf bingxi-erp-*.tar.gz -C $DEPLOY_DIR
        log "SUCCESS" "发布包解压完成"
    else
        log "ERROR" "找不到发布包"
        exit 1
    fi
    
    # 配置环境变量
    log "INFO" "[4/8] 配置环境变量..."
    if [ ! -f "$CONFIG_DIR/.env" ]; then
        if [ -f "$DEPLOY_DIR/backend/.env.example" ]; then
            cp $DEPLOY_DIR/backend/.env.example $CONFIG_DIR/.env
            log "SUCCESS" "环境配置文件已创建，请编辑 $CONFIG_DIR/.env 配置数据库等信息"
        else
            log "WARNING" "未找到环境配置文件模板"
        fi
    else
        log "INFO" "环境配置文件已存在"
    fi
    
    # 设置权限
    log "INFO" "[5/8] 设置权限..."
    chmod +x $DEPLOY_DIR/backend/bingxi_backend
    chown -R www-data:www-data $DEPLOY_DIR
    chown -R www-data:www-data $LOG_DIR
    chown -R www-data:www-data $CONFIG_DIR
    chmod 750 $LOG_DIR
    log "SUCCESS" "权限设置完成"
    
    # 配置 systemd 服务
    log "INFO" "[6/8] 配置系统服务..."
    cp $DEPLOY_DIR/deploy/bingxi-backend.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable bingxi-backend
    systemctl restart bingxi-backend
    log "SUCCESS" "系统服务配置完成"
    
    # 配置 Nginx
    log "INFO" "[7/8] 配置 Nginx..."
    cp $DEPLOY_DIR/deploy/nginx.conf /etc/nginx/sites-available/bingxi-erp
    ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/
    if nginx -t; then
        systemctl reload nginx
        log "SUCCESS" "Nginx 配置完成"
    else
        log "ERROR" "Nginx 配置错误"
        exit 1
    fi
    
    # 检查服务状态
    log "INFO" "[8/8] 检查服务状态..."
    if check_service "bingxi-backend"; then
        log "SUCCESS" "后端服务运行正常"
    else
        log "ERROR" "后端服务启动失败"
        exit 1
    fi
    
    if check_service "nginx"; then
        log "SUCCESS" "Nginx 服务运行正常"
    else
        log "ERROR" "Nginx 服务启动失败"
        exit 1
    fi
    
    # 清理临时文件
    log "INFO" "清理临时文件..."
    rm -f bingxi-erp-*.tar.gz
    log "SUCCESS" "临时文件清理完成"
    
    log "INFO" "========================================"
    log "SUCCESS" "  部署完成！"
    log "INFO" "========================================"
    log "INFO" "后端服务状态: $(systemctl is-active bingxi-backend)"
    log "INFO" "Nginx 服务状态: $(systemctl is-active nginx)"
    log "INFO" "访问地址: http://$(hostname -I | awk '{print $1}')"
    log "INFO" "日志目录: $LOG_DIR"
    log "INFO" "部署日志: $DEPLOY_LOG"
    log "INFO" ""
    log "INFO" "常用命令："
    log "INFO" "  查看后端状态：sudo systemctl status bingxi-backend"
    log "INFO" "  查看 Nginx 状态：sudo systemctl status nginx"
    log "INFO" "  查看后端日志：sudo journalctl -u bingxi-backend -f"
    log "INFO" "  查看 Nginx 日志：sudo tail -f /var/log/nginx/error.log"
}

# 运行主函数
main

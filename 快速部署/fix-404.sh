#!/bin/bash
# 快速修复 404 问题脚本

echo "正在检查并修复 404 问题..."

DEPLOY_DIR="/opt/bingxi-erp"

# 检查目录结构
if [ -d "$DEPLOY_DIR/frontend" ] && [ ! -d "$DEPLOY_DIR/frontend/dist" ]; then
    echo "发现目录结构问题，正在修复..."
    
    # 创建 dist 目录
    mkdir -p "$DEPLOY_DIR/frontend/dist"
    
    # 移动文件到 dist 目录
    if [ -f "$DEPLOY_DIR/frontend/index.html" ]; then
        echo "移动前端文件到正确位置..."
        mv "$DEPLOY_DIR/frontend"/* "$DEPLOY_DIR/frontend/dist/" 2>/dev/null || true
        
        # 重新设置权限
        chown -R www-data:www-data "$DEPLOY_DIR"
        echo "权限已重新设置"
        
        # 重启 Nginx
        echo "重启 Nginx 服务..."
        systemctl reload nginx
        
        echo "修复完成！请刷新浏览器页面查看效果。"
    else
        echo "未找到前端文件，请重新运行更新命令"
    fi
else
    echo "目录结构正常，无需修复"
    echo "当前目录结构："
    ls -la "$DEPLOY_DIR/frontend/"
fi

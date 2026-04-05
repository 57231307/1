with open("deploy/deploy.sh", "r") as f:
    content = f.read()

# Fix Nginx config
nginx_fix = """    # 配置 Nginx
    log "INFO" "[7/8] 配置 Nginx..."
    cp $DEPLOY_DIR/deploy/nginx.conf /etc/nginx/sites-available/bingxi-erp
    ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/
    # 确保没有默认的冲突配置
    rm -f /etc/nginx/sites-enabled/default
    if nginx -t; then"""

content = content.replace("""    # 配置 Nginx
    log "INFO" "[7/8] 配置 Nginx..."
    cp $DEPLOY_DIR/deploy/nginx.conf /etc/nginx/sites-available/bingxi-erp
    ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/
    if nginx -t; then""", nginx_fix)


# Make the env check more robust to avoid immediate crash
env_check = """    # 配置环境变量
    log "INFO" "[4/8] 配置环境变量..."
    if [ ! -f "$CONFIG_DIR/.env" ]; then
        if [ -f "$DEPLOY_DIR/backend/.env.example" ]; then
            cp $DEPLOY_DIR/backend/.env.example $CONFIG_DIR/.env
            # 为了防止第一次启动直接崩溃，修改默认数据库配置为本地默认（假设装了 postgres）或提示
            sed -i 's/DATABASE__HOST=.*/DATABASE__HOST=127.0.0.1/' $CONFIG_DIR/.env
            sed -i 's/DATABASE__USERNAME=.*/DATABASE__USERNAME=postgres/' $CONFIG_DIR/.env
            sed -i 's/DATABASE__PASSWORD=.*/DATABASE__PASSWORD=postgres/' $CONFIG_DIR/.env
            sed -i 's/ENV=development/ENV=production/' $CONFIG_DIR/.env
            log "SUCCESS" "环境配置文件已创建: $CONFIG_DIR/.env"
            log "WARNING" "首次部署已使用默认本地数据库(postgres/postgres)，请务必修改配置后重启！"
        else
            log "WARNING" "未找到环境配置文件模板"
        fi
    else
        log "INFO" "环境配置文件已存在"
    fi"""

content = content.replace("""    # 配置环境变量
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
    fi""", env_check)


with open("deploy/deploy.sh", "w") as f:
    f.write(content)

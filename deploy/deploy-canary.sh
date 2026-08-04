#!/bin/bash
# V15 P1 20.7-A：灰度发布管理脚本
# 用途：管理灰度发布流程（10% → 50% → 100%），支持自动健康检查 + 失败回滚
#
# 用法：
#   ./deploy-canary.sh start <package>    # 启动灰度：部署新版本到 green，切 10% 流量
#   ./deploy-canary.sh promote 50         # 提升流量到 50%（10% 验证通过后）
#   ./deploy-canary.sh promote 100        # 全量切换到 green，停止 blue
#   ./deploy-canary.sh rollback           # 回滚：切回 blue，停止 green
#   ./deploy-canary.sh status             # 查询当前灰度状态
#
# 前置条件：
#   1. 已部署 blue 实例（端口 8082）
#   2. green 实例槽位空闲（端口 8083）
#   3. /etc/nginx/bingxi-upstream-{blue,green,canary-10,canary-50}.conf 已就位

set -euo pipefail

# 严格模式：未定义变量报错 + 管道失败传播
CONFIG_DIR="/etc/bingxi"
NGINX_UPSTREAM_DIR="/etc/nginx"
ACTIVE_CONF="${NGINX_UPSTREAM_DIR}/bingxi-upstream.active.conf"
LOG_DIR="/opt/bingxi-erp/logs"
mkdir -p "${LOG_DIR}"
LOG_FILE="${LOG_DIR}/canary-$(date +%Y%m%d-%H%M%S).log"

# 日志同时输出到 stdout 和文件
exec > >(tee -a "${LOG_FILE}") 2>&1

log()    { echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO]  $*"; }
warn()   { echo "[$(date '+%Y-%m-%d %H:%M:%S')] [WARN]  $*"; }
error()  { echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $*" >&2; }

# 检查 root 权限（操作 systemd + nginx 需要 root）
check_root() {
    if [[ $EUID -ne 0 ]]; then
        error "此脚本必须以 root 权限运行（请使用 sudo）"
        exit 1
    fi
}

# 健康检查指定端口（最多 10 次重试，每次 2 秒）
# 用法：health_check_port 8082
# V15 P2 25.1-E 修复：健康检查从仅看整体 status 增强为同时核验核心依赖 database。
health_check_port() {
    local port=$1
    local retry=0
    local max_retry=10
    while [[ $retry -lt $max_retry ]]; do
        local response
        response=$(curl -s "http://127.0.0.1:${port}/health" 2>/dev/null)
        if echo "$response" | grep -q '"status":"healthy"' && echo "$response" | grep -q '"database":{"status":"healthy"'; then
            log "端口 ${port} 健康检查通过（整体 + database 均 healthy）"
            return 0
        fi
        retry=$((retry + 1))
        warn "端口 ${port} 健康检查失败（${retry}/${max_retry}），2 秒后重试"
        sleep 2
    done
    error "端口 ${port} 健康检查连续 ${max_retry} 次失败"
    return 1
}

# 切换 nginx upstream 配置并 reload
# 用法：switch_upstream canary-10
switch_upstream() {
    local target=$1
    local conf="${NGINX_UPSTREAM_DIR}/bingxi-upstream-${target}.conf"
    if [[ ! -f "${conf}" ]]; then
        error "upstream 配置文件不存在：${conf}"
        exit 1
    fi
    ln -sf "${conf}" "${ACTIVE_CONF}"
    nginx -t
    nginx -s reload
    log "已切换 nginx upstream 到 ${target}"
}

# 启动灰度：部署新版本到 green，切 10% 流量
# 用法：canary_start <package_path>
canary_start() {
    local package=$1
    if [[ ! -f "${package}" ]]; then
        error "包文件不存在：${package}"
        exit 1
    fi

    log "=== 启动灰度发布 ==="
    log "包文件：${package}"

    # 1. 备份当前 blue 二进制
    if [[ -f /opt/bingxi-erp/backend/server ]]; then
        local backup_dir="/opt/bingxi-erp/backups/pre-canary-$(date +%Y%m%d%H%M%S)"
        mkdir -p "${backup_dir}"
        cp /opt/bingxi-erp/backend/server "${backup_dir}/server.blue"
        log "已备份 blue 二进制到 ${backup_dir}"
    fi

    # 2. 解压新版本到 green 实例目录
    log "解压新版本到 green 实例..."
    local tmp_dir="/tmp/canary-extract-$$"
    mkdir -p "${tmp_dir}"
    tar -xzf "${package}" -C "${tmp_dir}"
    cp "${tmp_dir}/server" /opt/bingxi-erp/backend/server.green
    rm -rf "${tmp_dir}"
    chmod 755 /opt/bingxi-erp/backend/server.green

    # 3. 启动 green 实例
    log "启动 green 实例（端口 8083）..."
    systemctl start bingxi-backend@green
    sleep 2

    # 4. 健康检查 green 实例
    if ! health_check_port 8083; then
        error "green 实例健康检查失败，停止 green 并清理"
        systemctl stop bingxi-backend@green
        rm -f /opt/bingxi-erp/backend/server.green
        exit 1
    fi

    # 5. 切换 nginx 到 10% 灰度配置
    switch_upstream canary-10
    log "10% 流量已切换到 green 实例"
    log "监控命令：journalctl -u bingxi-backend@green -f"
    log "=== 灰度启动完成（10% 流量）==="
}

# 提升流量比例
# 用法：canary_promote 50|100
canary_promote() {
    local percent=$1
    case "${percent}" in
        50)
            log "=== 提升灰度流量到 50% ==="
            switch_upstream canary-50
            log "监控 5-10 分钟，确认无异常后执行 promote 100"
            ;;
        100)
            log "=== 全量切换到 green 实例 ==="
            # 1. 切换 nginx 到 green 配置
            switch_upstream green
            # 2. 停止 blue 实例（green 已承接全部流量）
            log "停止 blue 实例..."
            systemctl stop bingxi-backend@blue
            # 3. 替换主二进制：green → 主路径
            cp /opt/bingxi-erp/backend/server.green /opt/bingxi-erp/backend/server
            rm -f /opt/bingxi-erp/backend/server.green
            # 4. 重启 blue 实例使用新二进制（保持 blue 槽位为新主版本）
            log "启动 blue 实例（使用新版本二进制）..."
            systemctl start bingxi-backend@blue
            sleep 2
            health_check_port 8082 || { error "blue 实例启动失败"; exit 1; }
            # 5. 切回 blue 为主
            switch_upstream blue
            log "停止 green 实例..."
            systemctl stop bingxi-backend@green
            log "=== 灰度发布完成（100% 流量到新版本）==="
            ;;
        *)
            error "无效的流量比例：${percent}（仅支持 50 或 100）"
            exit 1
            ;;
    esac
}

# 回滚：切回 blue，停止 green
canary_rollback() {
    log "=== 灰度回滚 ==="
    # 1. 切换 nginx 到 blue 配置
    switch_upstream blue
    # 2. 停止 green 实例
    log "停止 green 实例..."
    systemctl stop bingxi-backend@green 2>/dev/null || true
    # 3. 清理 green 二进制
    rm -f /opt/bingxi-erp/backend/server.green
    log "=== 回滚完成（100% 流量回到 blue）==="
}

# 查询灰度状态
canary_status() {
    log "=== 灰度发布状态 ==="
    # 当前活跃 upstream 配置
    if [[ -L "${ACTIVE_CONF}" ]]; then
        local target
        target=$(readlink "${ACTIVE_CONF}")
        log "当前 upstream 配置：${target}"
    else
        warn "未找到 active upstream symlink"
    fi
    # 实例状态
    log "blue 实例（8082）：$(systemctl is-active bingxi-backend@blue 2>/dev/null || echo 'inactive')"
    log "green 实例（8083）：$(systemctl is-active bingxi-backend@green 2>/dev/null || echo 'inactive')"
    # 健康检查
    if curl -sf "http://127.0.0.1:8082/health" >/dev/null 2>&1; then
        log "blue 健康检查：OK"
    else
        warn "blue 健康检查：失败"
    fi
    if curl -sf "http://127.0.0.1:8083/health" >/dev/null 2>&1; then
        log "green 健康检查：OK"
    else
        warn "green 健康检查：失败（实例未启动）"
    fi
}

# 主入口
main() {
    check_root
    local cmd=${1:-}
    case "${cmd}" in
        start)
            canary_start "$2"
            ;;
        promote)
            canary_promote "$2"
            ;;
        rollback)
            canary_rollback
            ;;
        status)
            canary_status
            ;;
        *)
            echo "用法：$0 {start <package> | promote <50|100> | rollback | status}"
            exit 1
            ;;
    esac
}

main "$@"

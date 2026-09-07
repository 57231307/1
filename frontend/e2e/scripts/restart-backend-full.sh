#!/bin/bash
# ==============================================================================
# Setup 向导真实链路 E2E：后端重启脚本（等效 systemd Restart=always）
#
# 真实链路时序：initialize-with-db 成功 → 后端自退（exit 0）→
# 本脚本等效 systemd 拉起 → 同库已初始化 → 完整模式（/health 200）。
# 环境变量与 setup-wizard-real-env.sh 完全一致（凭据读 test-context.env）。
# ==============================================================================
set -euo pipefail

CTX=/tmp/e2e-setup-logs/test-context.env
[ -f "$CTX" ] || { echo "缺少 $CTX（前置脚本未运行）"; exit 1; }
source "$CTX"

BACKEND_BIN="${BACKEND_BIN:?BACKEND_BIN 未设置}"
BACKEND_DIR="$(dirname "$BACKEND_BIN")"

echo "=== 重启后端（完整模式，端口 $SETUP_E2E_PORT，库 $SETUP_E2E_DB）==="

# 清理残留进程（后端自退后理论上无监听，防御性清理）
OLD_PID=$(ss -tlnp 2>/dev/null | grep ":${SETUP_E2E_PORT}\b" | grep -oP 'pid=\K[0-9]+' | head -1 || true)
if [ -n "$OLD_PID" ]; then
    echo "清理残留进程 PID=$OLD_PID"
    kill -9 "$OLD_PID" 2>/dev/null || true
    sleep 1
fi

# 同一套环境变量重新拉起：库已完成迁移+种子 → 启动即完整模式
export JWT_SECRET="${JWT_SECRET:-e2e-setup-jwt-secret-32bytes-real-data-abc}"
export COOKIE_SECRET="${COOKIE_SECRET:-e2e-setup-cookie-secret-32bytes-real-xyz}"
export WEBHOOK_SECRET="${WEBHOOK_SECRET:-e2e-setup-webhook-secret-32bytes-realq}"
export AUDIT_SECRET_KEY="${AUDIT_SECRET_KEY:-e2e-setup-audit-signing-key-32bytes-real}"
export INIT_TOKEN="$SETUP_E2E_INIT_TOKEN"
export APP_ENV=development
export SERVER__HOST="${SERVER__HOST:-127.0.0.1}"
export SERVER__PORT="$SETUP_E2E_PORT"
export DATABASE__HOST="${DATABASE__HOST:-127.0.0.1}"
export DATABASE__PORT="${DATABASE__PORT:-5432}"
export DATABASE__NAME="$SETUP_E2E_DB"
export DATABASE__USERNAME="${DATABASE__USERNAME:-bingxi}"
export DATABASE__PASSWORD="${DATABASE__PASSWORD:-bingxi_test}"
export DATABASE__MAX_CONNECTIONS=10
export LOG__LEVEL=info
export LOG__DIR=/tmp/e2e-setup-logs
export SLOW_QUERY__ENABLED=false
export CACHE_ENABLED=false
export KAFKA_ENABLED=false
export ELASTICSEARCH_URL=""

cd "$BACKEND_DIR"
# nohup + disown：脱离 CI step 进程组（同 setup-wizard-real-env.sh，自审修复）
nohup "$BACKEND_BIN" > /tmp/e2e-setup-logs/backend-full.log 2>&1 &
echo $! > /tmp/e2e-setup-logs/backend-full.pid
disown $! 2>/dev/null || true

READY=false
for i in $(seq 1 60); do
    if curl -s --max-time 2 "http://127.0.0.1:${SETUP_E2E_PORT}/health" 2>/dev/null | grep -q "healthy"; then
        READY=true
        echo "后端完整模式就绪（第 ${i} 次探测）"
        break
    fi
    sleep 1
done
if [ "$READY" != "true" ]; then
    echo "后端完整模式未就绪，日志："
    tail -40 /tmp/e2e-setup-logs/backend-full.log
    exit 1
fi

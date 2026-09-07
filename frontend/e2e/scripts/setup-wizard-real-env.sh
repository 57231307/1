#!/bin/bash
# ==============================================================================
# Setup 向导真实链路 E2E：环境准备脚本（本地/CI 通用）
#
# 真实数据约束（用户指令，禁止 mock）：
# - 全新空数据库 bingxi_setup_test（每次运行前 DROP+CREATE，保证 Setup 模式触发）
# - 后端以真实二进制启动（CI: ci-build-rust artifact；本地: GitHub Release 产物）
# - 数据库初始化通过 Setup 向导 UI 真实点击完成（UI → /init/test-database →
#   /init/initialize-with-db → 迁移+种子数据+管理员 → 后端自退）
# - 后端重启后以完整模式运行，再走真实登录验证
#
# 环境变量：
#   SETUP_E2E_PORT      后端端口（默认 8083，避开常规 8082 分片后端）
#   SETUP_E2E_DB        专用数据库名（默认 bingxi_setup_test）
#   SETUP_E2E_ADMIN     初始化管理员用户名（默认 setup_admin）
#   SETUP_E2E_PASS      初始化管理员密码（默认强密码）
#   SETUP_E2E_INIT_TOKEN  初始化令牌（默认 64 hex）
# ==============================================================================
set -euo pipefail

SETUP_E2E_PORT="${SETUP_E2E_PORT:-8083}"
SETUP_E2E_DB="${SETUP_E2E_DB:-bingxi_setup_test}"
SETUP_E2E_ADMIN="${SETUP_E2E_ADMIN:-setup_admin}"
SETUP_E2E_PASS="${SETUP_E2E_PASS:-Setup@E2E2026!x}"
SETUP_E2E_INIT_TOKEN="${SETUP_E2E_INIT_TOKEN:-$(openssl rand -hex 32)}"
BACKEND_BIN="${BACKEND_BIN:-/tmp/opencode/bingxi-erp/backend/server}"
BACKEND_DIR="$(dirname "$BACKEND_BIN")"

echo "=== [1/5] 重置专用空库 $SETUP_E2E_DB ==="
# 每次运行从零开始：断开现有连接 → DROP → CREATE（空库无表 → 后端启动即 Setup 模式）
# 优先 su postgres（本地 root 环境，socket 信任认证）；
# 失败（CI postgres service 无 postgres 系统用户）时回退 TCP + bingxi 凭据
run_psql() {
    if su postgres -c "psql -c 'SELECT 1;'" >/dev/null 2>&1; then
        su postgres -c "psql -q"
    else
        PGPASSWORD=bingxi_test psql -h 127.0.0.1 -U bingxi -d postgres -q
    fi
}
run_psql <<SQL
SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$SETUP_E2E_DB' AND pid <> pg_backend_pid();
DROP DATABASE IF EXISTS $SETUP_E2E_DB;
CREATE DATABASE $SETUP_E2E_DB OWNER bingxi;
SQL
echo "空库就绪: $SETUP_E2E_DB"

echo "=== [2/5] 清理旧后端进程（端口 $SETUP_E2E_PORT）==="
OLD_PID=$(ss -tlnp 2>/dev/null | grep ":${SETUP_E2E_PORT}\b" | grep -oP 'pid=\K[0-9]+' | head -1 || true)
if [ -n "$OLD_PID" ]; then
    echo "杀掉旧进程 PID=$OLD_PID"
    kill -9 "$OLD_PID" 2>/dev/null || true
    sleep 1
fi

echo "=== [3/5] 生成后端 config（Setup 模式：库已存在但无表）==="
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

mkdir -p "$LOG__DIR"

echo "=== [4/5] 启动后端（预期进入 Setup 模式：空库无表连接失败）==="
cd "$BACKEND_DIR"
# nohup + disown：CI（GitHub Actions）在 step 结束时清理进程组，普通 &
# 后台进程活不到下一个 step（自审发现）。nohup 脱离 SIGHUP、disown 脱离
# shell 作业表；runner 对 orphan 进程按"仍运行"处理（与 systemd 服务等效）
nohup "$BACKEND_BIN" > /tmp/e2e-setup-logs/backend-setup.log 2>&1 &
BACKEND_PID=$!
disown "$BACKEND_PID" 2>/dev/null || true
echo "$BACKEND_PID" > /tmp/e2e-setup-logs/backend.pid

# 探活：Setup 模式下 /health 404（仅 /init/* 路由），用 /init/status 探测。
# 窗口 300 次（connection refused 瞬回 + 0.5s sleep ≈ 150s 上限）：
# CI 实测后端启动耗时波动大（28s → 60s，随 runner 负载），二轮 60s 窗口
# 在就绪前 1s 耗尽再次误报（run 34154361046）。150s 上限覆盖最慢场景
READY=false
for i in $(seq 1 300); do
    STATUS=$(curl -s --max-time 2 "http://127.0.0.1:${SETUP_E2E_PORT}/api/v1/erp/init/status" 2>/dev/null || true)
    if echo "$STATUS" | grep -q '"mode"'; then
        READY=true
        echo "后端 Setup 模式就绪（第 ${i} 次探测）: $STATUS"
        break
    fi
    sleep 0.5
done
if [ "$READY" != "true" ]; then
    echo "后端未进入 Setup 模式，日志："
    tail -40 /tmp/e2e-setup-logs/backend-setup.log
    exit 1
fi

echo "=== [5/5] 输出测试上下文 ==="
cat > /tmp/e2e-setup-logs/test-context.env <<ENV
SETUP_E2E_PORT=$SETUP_E2E_PORT
SETUP_E2E_DB=$SETUP_E2E_DB
SETUP_E2E_ADMIN=$SETUP_E2E_ADMIN
SETUP_E2E_PASS=$SETUP_E2E_PASS
SETUP_E2E_INIT_TOKEN=$SETUP_E2E_INIT_TOKEN
ENV
chmod 600 /tmp/e2e-setup-logs/test-context.env
echo "环境准备完成（凭据写入 /tmp/e2e-setup-logs/test-context.env）"
echo "INIT_TOKEN=$SETUP_E2E_INIT_TOKEN" | sed 's/=.*$/=<redacted-for-log>/'

#!/bin/bash
# =============================================================================
# 修复生产服务器 config.yaml 缺失字段 (CORS 段)
# =============================================================================
# 背景:
#   - v2026.66.915 部署后服务循环 panic 重启 547+ 次
#   - 错误: missing field `allow_credentials` at settings.rs:145:17
#   - 根因: config.yaml 的 cors 段只有 allowed_origins，缺 4 个字段
#   - 已推送修复 commit 515a4e5 (CorsConfig #[serde(default)] 兜底)
# 用法: bash scripts/fix-server-config.sh
#
# =============================================================================
# 批次 28 v7 P0-6 修复（2026-06-29）：对齐 deploy-latest.sh 安全模式
# =============================================================================
# 原脚本存在的安全问题：
#   1. PROD_IP 默认值 111.230.99.236 暴露真实生产 IP（攻击者扫描 GitHub 即可获取）
#   2. 强制使用 sshpass 密码认证，未支持 SSH 密钥（更安全的方式）
#   3. StrictHostKeyChecking=no 完全禁用主机密钥校验（中间人攻击风险）
#   4. 健康检查端点 /api/v1/erp/health/ 末尾带斜杠且路径错误（应返回 /health）
#
# 修复方案（对齐 deploy-latest.sh 批次 24 模式）：
#   - PROD_IP 改为 BINGXI_SERVER_IP 环境变量强制要求（fail-secure）
#   - 认证方式优先级：BINGXI_SSH_KEY（密钥，推荐）> BINGXI_SSH_PASS（密码，过渡回退）
#   - StrictHostKeyChecking=accept-new：首次接受，后续校验
#   - 健康检查端点改为 /health（与 routes/mod.rs:359 一致）
# =============================================================================

set -euo pipefail

# 批次 28 v7 P0-6 修复：移除硬编码生产服务器 IP 默认值（fail-secure）
PROD_IP="${BINGXI_SERVER_IP:?必须设置 BINGXI_SERVER_IP 环境变量（生产服务器 IP）}"
SSH_USER="${BINGXI_SSH_USER:-root}"
SSH_KEY="${BINGXI_SSH_KEY:-}"
SSH_PASS="${BINGXI_SSH_PASS:-}"
CONFIG_PATH="/opt/bingxi-erp/backend/config.yaml"
BACKUP_PATH="/opt/bingxi-erp/backend/config.yaml.bak.$(date +%Y%m%d_%H%M%S)"

# 认证方式选择：密钥优先，密码回退
if [[ -n "$SSH_KEY" ]]; then
    SSH_AUTH_MODE="key"
elif [[ -n "$SSH_PASS" ]]; then
    # 密码认证回退（不推荐，仅用于过渡）
    SSH_AUTH_MODE="password"
    echo "警告：使用密码认证，建议尽快迁移到 SSH 密钥认证（设置 BINGXI_SSH_KEY 环境变量）" >&2
else
    echo "错误：请设置 BINGXI_SSH_KEY（推荐，SSH 密钥认证）或 BINGXI_SSH_PASS（密码，过渡回退）环境变量" >&2
    exit 1
fi

# 检查依赖（密钥认证无需 sshpass，密码认证才需要）
if [[ "$SSH_AUTH_MODE" == "password" ]]; then
    if ! command -v sshpass >/dev/null 2>&1; then
        echo "ERROR: 缺少 sshpass，请先安装: apt-get install -y sshpass，或改用 SSH 密钥认证（设置 BINGXI_SSH_KEY）" >&2
        exit 1
    fi
fi

# SSH 命令封装（密钥认证优先，密码认证回退）
# StrictHostKeyChecking=accept-new：首次连接自动接受主机密钥，后续校验防止中间人攻击
remote_exec() {
    if [[ "$SSH_AUTH_MODE" == "key" ]]; then
        # 使用 SSH 密钥认证（推荐）
        ssh -i "$SSH_KEY" -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 "$SSH_USER@$PROD_IP" "$1"
    else
        # 密码认证回退（不推荐，仅用于过渡）
        sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 "$SSH_USER@$PROD_IP" "$1"
    fi
}

# SCP 命令封装（与 remote_exec 同样的认证方式）
remote_copy() {
    if [[ "$SSH_AUTH_MODE" == "key" ]]; then
        scp -i "$SSH_KEY" -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 \
            "$1" "$SSH_USER@$PROD_IP:$2"
    else
        sshpass -p "$SSH_PASS" scp -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 \
            "$1" "$SSH_USER@$PROD_IP:$2"
    fi
}

echo "═══════════════════════════════════════════════════════════════"
echo " 1. 备份现有 config.yaml"
echo "═══════════════════════════════════════════════════════════════"
remote_exec "cp -a ${CONFIG_PATH} ${BACKUP_PATH} && ls -lh ${BACKUP_PATH}"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 2. 检查 systemd 服务当前状态"
echo "═══════════════════════════════════════════════════════════════"
remote_exec "systemctl is-active bingxi-backend || true; \
    systemctl status bingxi-backend --no-pager | head -5 || true"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 3. 修补 config.yaml（用 Python 安全改写）"
echo "═══════════════════════════════════════════════════════════════"
# 传输修补脚本到服务器并执行
remote_copy "$(dirname "$0")/patch_cors_config.py" "/tmp/patch_cors_config.py"

remote_exec "python3 /tmp/patch_cors_config.py ${CONFIG_PATH} && \
    echo '--- 校验结果 ---' && \
    python3 -c \"import yaml; d=yaml.safe_load(open('${CONFIG_PATH}')); print('cors.allow_credentials:', d.get('cors', {}).get('allow_credentials')); print('cors.allowed_methods:', d.get('cors', {}).get('allowed_methods')); print('cors.allowed_headers:', d.get('cors', {}).get('allowed_headers')); print('cors.max_age_secs:', d.get('cors', {}).get('max_age_secs'))\""

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 4. 等待新二进制部署完成（CI v2026.66.918+ 发布）"
echo "═══════════════════════════════════════════════════════════════"
echo "请先确认 GitHub Actions run #775 已完成并发布新版本："
echo "  https://github.com/57231307/1/actions/runs/27052828219"
echo "确认发布后回车继续（Ctrl+C 取消）"
read

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 5. 重启服务并验证"
echo "═══════════════════════════════════════════════════════════════"
# 批次 28 v7 P0-6 修复：健康检查端点从 /api/v1/erp/health/ 改为 /health。
# 实际路由注册在 routes/mod.rs:359 和 routes/system.rs:196，均为顶层 /health。
# 原 /api/v1/erp/health/ 末尾带斜杠且路径错误，curl 返回 404 误判为服务异常。
remote_exec "systemctl restart bingxi-backend && \
    sleep 3 && \
    echo '--- 服务状态 ---' && \
    systemctl is-active bingxi-backend && \
    echo '--- 健康检查 ---' && \
    curl -sS http://127.0.0.1:8082/health && echo"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 6. 验证完成"
echo "═══════════════════════════════════════════════════════════════"
echo "如服务仍在重启，查看日志："
echo "  ssh ${SSH_USER}@${PROD_IP} 'journalctl -u bingxi-backend -n 50 --no-pager'"

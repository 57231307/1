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
# =============================================================================

set -euo pipefail

PROD_IP="${PROD_IP:-111.230.99.236}"
SSH_USER="${SSH_USER:-root}"
CONFIG_PATH="/opt/bingxi-erp/backend/config.yaml"
BACKUP_PATH="/opt/bingxi-erp/backend/config.yaml.bak.$(date +%Y%m%d_%H%M%S)"

# 安全：禁止硬编码密码，必须通过环境变量 BINGXI_SSH_PASS 提供
if [[ -z "${BINGXI_SSH_PASS:-}" ]]; then
  echo "ERROR: 环境变量 BINGXI_SSH_PASS 未设置" >&2
  echo "请先 export BINGXI_SSH_PASS='你的SSH密码' 再执行" >&2
  exit 1
fi
if ! command -v sshpass >/dev/null 2>&1; then
  echo "ERROR: 缺少 sshpass，请先安装: apt-get install -y sshpass" >&2
  exit 1
fi

echo "═══════════════════════════════════════════════════════════════"
echo " 1. 备份现有 config.yaml"
echo "═══════════════════════════════════════════════════════════════"
sshpass -p "${BINGXI_SSH_PASS}" ssh -o StrictHostKeyChecking=no \
  ${SSH_USER}@${PROD_IP} "cp -a ${CONFIG_PATH} ${BACKUP_PATH} && ls -lh ${BACKUP_PATH}"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 2. 检查 systemd 服务当前状态"
echo "═══════════════════════════════════════════════════════════════"
sshpass -p "${BINGXI_SSH_PASS}" ssh -o StrictHostKeyChecking=no \
  ${SSH_USER}@${PROD_IP} "systemctl is-active bingxi-backend || true; \
    systemctl status bingxi-backend --no-pager | head -5 || true"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 3. 修补 config.yaml（用 Python 安全改写）"
echo "═══════════════════════════════════════════════════════════════"
# 传输修补脚本到服务器并执行
sshpass -p "${BINGXI_SSH_PASS}" scp -o StrictHostKeyChecking=no \
  $(dirname "$0")/patch_cors_config.py ${SSH_USER}@${PROD_IP}:/tmp/patch_cors_config.py

sshpass -p "${BINGXI_SSH_PASS}" ssh -o StrictHostKeyChecking=no \
  ${SSH_USER}@${PROD_IP} "python3 /tmp/patch_cors_config.py ${CONFIG_PATH} && \
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
sshpass -p "${BINGXI_SSH_PASS}" ssh -o StrictHostKeyChecking=no \
  ${SSH_USER}@${PROD_IP} "systemctl restart bingxi-backend && \
    sleep 3 && \
    echo '--- 服务状态 ---' && \
    systemctl is-active bingxi-backend && \
    echo '--- 健康检查 ---' && \
    curl -sS http://127.0.0.1:8082/api/v1/erp/health/ && echo"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " 6. 验证完成"
echo "═══════════════════════════════════════════════════════════════"
echo "如服务仍在重启，查看日志："
echo "  ssh ${SSH_USER}@${PROD_IP} 'journalctl -u bingxi-backend -n 50 --no-pager'"

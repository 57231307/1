// security 业务流程 composable
// 拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：登录安全（解锁账户 + 导出日志 + 查询）
// 批次 282：移除 handleSizeChange/handleCurrentChange（useTableApi watch 自动处理分页）
import { ElMessageBox } from 'element-plus';
import { useI18n } from 'vue-i18n';
import { msg } from '@/utils/message';
import {
  unlockAccount,
  exportLoginLogs,
  resolveSecurityAlert,
  type LockedAccount,
  type SecurityQueryParams,
  type SecurityAlert,
} from '@/api/security';
import { logger } from '@/utils/logger';

// v11 批次 181 P2-1 修复：定义 SecContext 接口替代 any
// 批次 282：适配 useTableApi（page 独立 ref，queryParams 为 Record<string, unknown>）
interface SecContext {
  page: number;
  queryParams: Record<string, unknown>;
  getLoginLogs: () => Promise<void>;
  getLockedAccounts: () => Promise<void>;
  getStats: () => Promise<void>;
  getSecurityAlerts: () => Promise<void>;
}

/** security 业务流程 composable */
export const useSecProc = () => {
  const { t } = useI18n({ useScope: 'global' });

  // 查询：重置页码 + 拉取日志（批次 282：page 独立 ref，refresh 别名 getLoginLogs）
  const handleQuery = (sec: SecContext) => {
    sec.page = 1;
    sec.getLoginLogs();
  };

  // 解锁账户
  const handleUnlock = async (row: LockedAccount, sec: SecContext) => {
    try {
      await ElMessageBox.confirm(
        t('security.lockTable.confirmUnlock', { username: row.username }),
        t('security.index.button.prompt'),
        { type: 'warning' }
      );
      await unlockAccount(row.id);
      msg.success('unlockSuccess');
      sec.getLockedAccounts();
      sec.getStats();
    } catch (error) {
      logger.error(t('security.message.unlockFailed'), error);
    }
  };

  // 处理安全告警（resolveSecurityAlert）
  const handleResolveAlert = async (row: SecurityAlert, sec: SecContext) => {
    try {
      await ElMessageBox.confirm(
        t('security.alertTable.confirmResolve', { id: row.id }),
        t('security.alertTable.title'),
        { type: 'warning' }
      );
      await resolveSecurityAlert(row.id);
      msg.success('resolveSuccess');
      sec.getSecurityAlerts();
      sec.getStats();
    } catch (error) {
      if (error !== 'cancel') {
        logger.error(t('security.message.resolveFailed'), error);
        msg.error('resolveFailed');
      }
    }
  };

  // 导出日志
  const handleExport = async (sec: SecContext) => {
    try {
      const res = await exportLoginLogs(sec.queryParams as SecurityQueryParams);
      const url = window.URL.createObjectURL(new Blob([res]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', t('security.logTable.downloadFilename'));
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
      msg.success('exportSuccess');
    } catch (error) {
      logger.error(t('security.message.exportFailed'), error);
    }
  };

  return {
    handleQuery,
    handleUnlock,
    handleResolveAlert,
    handleExport,
  };
};

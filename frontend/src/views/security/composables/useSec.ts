// security 主业务 composable
// 拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：登录安全（stats + loginLogs + lockedAccounts + securityAlerts + 过滤/分页）
// 批次 282：loginLogs 接入 useTableApi，lockedAccounts/securityAlerts 保持原样（无分页）
import { reactive, ref } from 'vue'
import { securityApi } from '@/api/security'
import type { LoginLog, LockedAccount, SecurityAlert } from '@/api/security'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/** security 主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useSec = () => {
  // 统计数据
  const stats = reactive({
    todayLogins: 0,
    todayFailures: 0,
    lockedAccounts: 0,
    securityAlerts: 0,
  })

  // 登录日志 - 接入 useTableApi（批次 282）
  const {
    data: loginLogs,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: getLoginLogs,
  } = useTableApi<LoginLog>({
    url: '/login-logs',
    defaultParams: { username: '', status: '', date_range: [] as string[] },
    onError: (err: unknown) => logger.error('获取登录日志失败:', err),
  })

  // 锁定账户
  const lockLoading = ref(false)
  const lockedAccounts = ref<LockedAccount[]>([])

  // 安全告警
  const alertLoading = ref(false)
  const securityAlerts = ref<SecurityAlert[]>([])

  // 获取统计数据
  const getStats = async () => {
    try {
      const res = await securityApi.getStats()
      if (res.data) {
        Object.assign(stats, res.data)
      }
    } catch (error) {
      logger.error('获取统计数据失败:', error)
    }
  }

  // 获取锁定账户
  const getLockedAccounts = async () => {
    lockLoading.value = true
    try {
      const res = await securityApi.getLockedAccounts()
      lockedAccounts.value = res.data || []
    } catch (error) {
      logger.error('获取锁定账户失败:', error)
    } finally {
      lockLoading.value = false
    }
  }

  // 获取安全告警
  const getSecurityAlerts = async () => {
    alertLoading.value = true
    try {
      const res = await securityApi.getSecurityAlerts()
      securityAlerts.value = res.data || []
    } catch (error) {
      logger.error('获取安全告警失败:', error)
    } finally {
      alertLoading.value = false
    }
  }

  return reactive({
    stats,
    // 登录日志（useTableApi 管理）
    loginLogs,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    getLoginLogs,
    // 锁定账户
    lockLoading,
    lockedAccounts,
    // 安全告警
    alertLoading,
    securityAlerts,
    // 方法
    getStats,
    getLockedAccounts,
    getSecurityAlerts,
  })
}

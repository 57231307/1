/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useApiLog.ts - API 网关调用日志 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供调用日志列表查询与详情查看业务方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { listApiLogs, type ApiLog } from '@/api/api-gateway'

/**
 * 调用日志 composable
 */
export function useApiLog() {
  const logs = ref<ApiLog[]>([])
  const logTotal = ref(0)
  const logLoading = ref(false)
  const logQuery = reactive({
    page: 1,
    page_size: 20,
    keyword: '',
    method: '',
    status_code: '',
    // 对齐 ApiLogTab.LogQuery 字段（vue-tsc 类型检查需要）
    status: '',
    date_range: null as [Date, Date] | null,
  })

  const logDetailVisible = ref(false)
  const currentLog = ref<ApiLog | null>(null)

  const fetchLogs = async () => {
    logLoading.value = true
    try {
      const res = await listApiLogs(logQuery)
      logs.value = res.data || []
      logTotal.value = res.total || 0
    } catch (error: any) {
      ElMessage.error(error.message || '获取日志失败')
    } finally {
      logLoading.value = false
    }
  }

  const viewLogDetail = (row: ApiLog) => {
    currentLog.value = row
    logDetailVisible.value = true
  }

  return {
    logs,
    logTotal,
    logLoading,
    logQuery,
    methodTypeMap: {
      GET: 'primary',
      POST: 'success',
      PUT: 'warning',
      DELETE: 'danger',
      PATCH: 'info',
    } as Record<string, string>,
    fetchLogs,
    logDetailVisible,
    currentLog,
    viewLogDetail,
  }
}

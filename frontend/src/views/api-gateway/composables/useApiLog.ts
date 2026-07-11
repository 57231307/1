/**
 * useApiLog.ts - API 网关调用日志 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供调用日志列表查询与详情查看业务方法
 * 行为完全保持一致（仅结构重构）
 * 批次 281：接入 useTableApi，移除手写 logs/logTotal/logLoading/logQuery + fetchLogs
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { type ApiLog } from '@/api/api-gateway'
import { useTableApi } from '@/composables/useTableApi'

/**
 * 调用日志 composable
 * 批次 281：返回 reactive 包装，父组件可直接 .字段 访问（无需 .value）
 */
export function useApiLog() {
  const {
    data: logs,
    total: logTotal,
    loading: logLoading,
    page,
    pageSize,
    queryParams: logQuery,
    refresh: fetchLogs,
  } = useTableApi<ApiLog>({
    url: '/api-gateway/logs',
    onError: (err: unknown) =>
      ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取日志失败'),
  })

  const logDetailVisible = ref(false)
  const currentLog = ref<ApiLog | null>(null)

  const viewLogDetail = (row: ApiLog) => {
    currentLog.value = row
    logDetailVisible.value = true
  }

  // 批次 281：reactive 包装返回值，ref 自动解包，父组件无需 .value
  // logQuery 是 ref<Record>，reactive 解包后为 reactive 对象，子组件可直接 Object.assign 修改属性
  // page/pageSize 暴露给子组件用于 v-model 分页绑定
  return reactive({
    logs,
    logTotal,
    logLoading,
    logQuery,
    page,
    pageSize,
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
  })
}

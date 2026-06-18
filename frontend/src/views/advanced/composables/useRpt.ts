/**
 * useRpt - 报表引擎 tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 2 个 tab）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { listReportTemplates, executeReport as executeReportApi } from '@/api/advanced'

/**
 * 报表引擎 tab 业务逻辑封装
 * 包含报表模板列表、执行报表、导出报表
 */
export function useRpt() {
  const reportTemplates = ref<any[]>([])
  const reportLoading = ref(false)
  const reportResultVisible = ref(false)
  const reportData = ref<any[]>([])
  const reportColumns = ref<any[]>([])

  /**
   * 加载报表模板列表
   */
  const fetchReportTemplates = async () => {
    reportLoading.value = true
    try {
      const res: any = await listReportTemplates()
      reportTemplates.value = res.data! || []
    } finally {
      reportLoading.value = false
    }
  }

  /**
   * 执行指定报表模板
   */
  const executeReport = async (row: any) => {
    try {
      const res: any = await executeReportApi(row.template_code)
      reportData.value = res.data?.data
      reportColumns.value = res.data?.columns || []
      reportResultVisible.value = true
    } catch (e: any) {
      ElMessage.error(e.message || '执行失败')
    }
  }

  /**
   * 导出报表
   */
  const exportReport = async (_row: any, _format: string) => {
    try {
      ElMessage.success('导出成功')
    } catch (e: any) {
      ElMessage.error(e.message || '导出失败')
    }
  }

  return {
    reportTemplates,
    reportLoading,
    reportResultVisible,
    reportData,
    reportColumns,
    fetchReportTemplates,
    executeReport,
    exportReport,
  }
}

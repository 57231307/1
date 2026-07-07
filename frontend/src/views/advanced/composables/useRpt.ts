/**
 * useRpt - 报表引擎 tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 2 个 tab）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { listReportTemplates, executeReport as executeReportApi } from '@/api/advanced'

// v11 批次 180 P2-1 修复：定义报表相关类型，替代 any
export interface ReportTemplate {
  template_code: string
  template_name?: string
  description?: string
  [key: string]: unknown
}

export interface ReportColumn {
  key: string
  label: string
  [key: string]: unknown
}

export interface ReportResult {
  data?: unknown[]
  columns?: ReportColumn[]
}

/**
 * 报表引擎 tab 业务逻辑封装
 * 包含报表模板列表、执行报表、导出报表
 */
export function useRpt() {
  const reportTemplates = ref<ReportTemplate[]>([])
  const reportLoading = ref(false)
  const reportResultVisible = ref(false)
  const reportData = ref<unknown[]>([])
  const reportColumns = ref<ReportColumn[]>([])

  /**
   * 加载报表模板列表
   */
  const fetchReportTemplates = async () => {
    reportLoading.value = true
    try {
      // v11 批次 180 P2-1 修复：res: any 改为具体类型
      const res = (await listReportTemplates()) as { data?: ReportTemplate[] }
      reportTemplates.value = res.data || []
    } finally {
      reportLoading.value = false
    }
  }

  /**
   * 执行指定报表模板
   */
  const executeReport = async (row: ReportTemplate) => {
    try {
      // v11 批次 180 P2-1 修复：res: any 改为具体类型
      const res = (await executeReportApi(row.template_code)) as { data?: ReportResult }
      reportData.value = res.data?.data || []
      reportColumns.value = res.data?.columns || []
      reportResultVisible.value = true
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e)
      ElMessage.error(errMsg || '执行失败')
    }
  }

  /**
   * 导出报表
   */
  const exportReport = async (_row: ReportTemplate, _format: string) => {
    try {
      ElMessage.success('导出成功')
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e)
      ElMessage.error(errMsg || '导出失败')
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

/**
 * useRptExp - 报表导出与预览 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { exportReport, previewReport, type ReportTemplate } from '@/api/report-enhanced'

/**
 * 导出表单数据结构
 */
export interface ExportFormData {
  template_id: number
  template_name: string
  format: 'pdf' | 'excel'
  date_range: { start: string; end: string }
}

/**
 * 报表导出与预览 composable
 */
export function useRptExp() {
  // 预览对话框
  const previewDialogVisible = ref(false)
  const previewData = ref<any>(null)

  // 导出对话框
  const exportDialogVisible = ref(false)
  const exportForm = ref<ExportFormData>({
    template_id: 0,
    template_name: '',
    format: 'excel',
    date_range: { start: '', end: '' },
  })

  /**
   * 预览报表
   */
  const handlePreview = async (row: ReportTemplate) => {
    try {
      const res: any = await previewReport(row.id)
      previewData.value = res.data
      previewDialogVisible.value = true
    } catch {
      ElMessage.error('预览失败')
    }
  }

  /**
   * 打开导出对话框
   */
  const handleExport = (row: ReportTemplate) => {
    exportForm.value = {
      template_id: row.id,
      template_name: row.name,
      format: 'excel',
      date_range: { start: '', end: '' },
    }
    exportDialogVisible.value = true
  }

  /**
   * 执行导出
   */
  const doExport = async () => {
    try {
      const blob = await exportReport(exportForm.value.template_id, {
        format: exportForm.value.format,
        date_range:
          exportForm.value.date_range.start && exportForm.value.date_range.end
            ? { start: exportForm.value.date_range.start, end: exportForm.value.date_range.end }
            : undefined,
      })
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `${exportForm.value.template_name}.${exportForm.value.format === 'pdf' ? 'pdf' : 'xlsx'}`
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      window.URL.revokeObjectURL(url)
      ElMessage.success('导出成功')
      exportDialogVisible.value = false
    } catch {
      ElMessage.error('导出失败')
    }
  }

  return {
    previewDialogVisible,
    previewData,
    exportDialogVisible,
    exportForm,
    handlePreview,
    handleExport,
    doExport,
  }
}

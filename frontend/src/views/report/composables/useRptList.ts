/**
 * useRptList - 报表模板列表与搜索 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue）
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listReportTemplates,
  deleteReportTemplate,
  type ReportTemplate,
} from '@/api/report-enhanced'

/**
 * 报表模板列表与搜索 composable
 */
export function useRptList() {
  const loading = ref(false)
  const templates = ref<ReportTemplate[]>([])
  const total = ref(0)
  const pagination = ref({ page: 1, pageSize: 20 })

  const searchForm = ref({
    name: '',
    type: '',
    category: '',
  })

  // 模板类型与分类选项
  const templateTypes = [
    { label: '销售报表', value: 'sales' },
    { label: '采购报表', value: 'purchase' },
    { label: '库存报表', value: 'inventory' },
    { label: '财务报表', value: 'finance' },
    { label: '应收报表', value: 'ar' },
    { label: '应付报表', value: 'ap' },
    { label: '自定义', value: 'custom' },
  ]

  const categories = [
    { label: '全部', value: '' },
    { label: '运营报表', value: 'operation' },
    { label: '财务报表', value: 'finance' },
    { label: '分析报表', value: 'analysis' },
    { label: '汇总报表', value: 'summary' },
  ]

  /**
   * 加载模板列表
   */
  const loadTemplates = async () => {
    loading.value = true
    try {
      const res: any = await listReportTemplates({
        page: pagination.value.page,
        pageSize: pagination.value.pageSize,
        ...searchForm.value,
      })
      templates.value = res.data?.list || []
      total.value = res.data?.total || 0
    } catch {
      ElMessage.error('加载模板列表失败')
    } finally {
      loading.value = false
    }
  }

  const handleSearch = () => {
    pagination.value.page = 1
    loadTemplates()
  }

  const handleReset = () => {
    searchForm.value = { name: '', type: '', category: '' }
    handleSearch()
  }

  const handlePageChange = (page: number) => {
    pagination.value.page = page
    loadTemplates()
  }

  const handlePageSizeChange = (pageSize: number) => {
    pagination.value.pageSize = pageSize
    loadTemplates()
  }

  /**
   * 删除模板
   */
  const handleDelete = async (row: ReportTemplate) => {
    if (row.is_system) {
      ElMessage.warning('系统模板不可删除')
      return
    }
    try {
      await ElMessageBox.confirm('确定要删除这个模板吗？', '提示', { type: 'warning' })
      await deleteReportTemplate(row.id)
      ElMessage.success('删除成功')
      loadTemplates()
    } catch (error: any) {
      if (error !== 'cancel') {
        ElMessage.error('删除失败')
      }
    }
  }

  return {
    loading,
    templates,
    total,
    pagination,
    searchForm,
    templateTypes,
    categories,
    loadTemplates,
    handleSearch,
    handleReset,
    handlePageChange,
    handlePageSizeChange,
    handleDelete,
  }
}

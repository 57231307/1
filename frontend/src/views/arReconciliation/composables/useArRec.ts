/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useArRec.ts - AR 对账核心 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供对账结果列表、详情、客户确认等业务方法
 * 争议管理由 useArDisp 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  autoReconcile,
  getAutoReconciliationResults,
  getReconciliationDetailItems,
  sendCustomerConfirmation,
  getCustomerConfirmations,
  updateConfirmationStatus,
  type AutoReconciliationResult,
  type ReconciliationDetailItem,
  type CustomerConfirmation,
} from '@/api/ar-reconciliation-enhanced'

/**
 * AR 对账核心 composable
 */
export function useArRec() {
  const loading = ref(false)
  const reconcileLoading = ref(false)
  const tableData = ref<AutoReconciliationResult[]>([])
  const total = ref(0)
  const pagination = ref({ page: 1, pageSize: 20 })

  const searchForm = ref({
    customer_name: '',
    match_status: '',
    start_date: '',
    end_date: '',
  })

  // 详情
  const detailDialogVisible = ref(false)
  const detailData = ref<ReconciliationDetailItem[]>([])
  const currentReconciliation = ref<AutoReconciliationResult | null>(null)

  // 客户确认
  const confirmDialogVisible = ref(false)
  const confirmData = ref<CustomerConfirmation[]>([])
  const confirmTotal = ref(0)

  /** 加载对账结果列表 */
  const loadData = async () => {
    loading.value = true
    try {
      const res: any = await getAutoReconciliationResults({
        page: pagination.value.page,
        pageSize: pagination.value.pageSize,
        customer_name: searchForm.value.customer_name || undefined,
        status: searchForm.value.match_status || undefined,
        start_date: searchForm.value.start_date || undefined,
        end_date: searchForm.value.end_date || undefined,
      })
      tableData.value = res.data?.list || []
      total.value = res.data?.total || 0
    } catch {
      ElMessage.error('加载对账结果失败')
    } finally {
      loading.value = false
    }
  }

  /** 启动自动对账 */
  const handleAutoReconcile = async () => {
    if (!searchForm.value.start_date || !searchForm.value.end_date) {
      ElMessage.warning('请选择对账日期范围')
      return
    }
    try {
      await ElMessageBox.confirm('确认启动自动对账？', '提示', { type: 'info' })
      reconcileLoading.value = true
      await autoReconcile({
        start_date: searchForm.value.start_date,
        end_date: searchForm.value.end_date,
      })
      ElMessage.success('自动对账任务已启动')
      await loadData()
    } catch (error: any) {
      if (error !== 'cancel') {
        ElMessage.error('启动对账失败')
      }
    } finally {
      reconcileLoading.value = false
    }
  }

  /** 查看对账明细 */
  const handleViewDetail = async (row: AutoReconciliationResult) => {
    try {
      const res: any = await getReconciliationDetailItems(row.id)
      detailData.value = res.data || []
      currentReconciliation.value = row
      detailDialogVisible.value = true
    } catch {
      ElMessage.error('获取对账明细失败')
    }
  }

  /** 发送客户对账确认 */
  const handleSendConfirmation = async (row: AutoReconciliationResult) => {
    try {
      await ElMessageBox.confirm('确认向客户发送对账确认请求？', '提示', { type: 'info' })
      await sendCustomerConfirmation(row.id)
      ElMessage.success('确认请求已发送')
    } catch (error: any) {
      if (error !== 'cancel') {
        ElMessage.error('发送确认请求失败')
      }
    }
  }

  /** 加载客户确认记录 */
  const handleViewConfirmations = async () => {
    try {
      const res: any = await getCustomerConfirmations({
        page: 1,
        pageSize: 20,
      })
      confirmData.value = res.data?.list || []
      confirmTotal.value = res.data?.total || 0
      confirmDialogVisible.value = true
    } catch {
      ElMessage.error('加载确认记录失败')
    }
  }

  /** 更新客户确认状态 */
  const handleConfirmStatus = async (
    row: CustomerConfirmation,
    status: 'confirmed' | 'disputed'
  ) => {
    const msg = status === 'confirmed' ? '确认此对账记录？' : '标记为争议？'
    try {
      await ElMessageBox.confirm(msg, '提示', { type: 'warning' })
      await updateConfirmationStatus(row.id, { status })
      ElMessage.success('操作成功')
      await handleViewConfirmations()
    } catch (error: any) {
      if (error !== 'cancel') {
        ElMessage.error('操作失败')
      }
    }
  }

  /** 搜索/重置 */
  const handleSearch = () => {
    pagination.value.page = 1
    loadData()
  }

  const handleReset = () => {
    searchForm.value = { customer_name: '', match_status: '', start_date: '', end_date: '' }
    handleSearch()
  }

  const handlePageChange = (page: number) => {
    pagination.value.page = page
    loadData()
  }

  const handlePageSizeChange = (pageSize: number) => {
    pagination.value.pageSize = pageSize
    loadData()
  }

  return {
    // 列表
    loading,
    reconcileLoading,
    tableData,
    total,
    pagination,
    searchForm,
    loadData,
    handleSearch,
    handleReset,
    handlePageChange,
    handlePageSizeChange,
    handleAutoReconcile,
    // 详情
    detailDialogVisible,
    detailData,
    currentReconciliation,
    handleViewDetail,
    handleSendConfirmation,
    // 确认
    confirmDialogVisible,
    confirmData,
    confirmTotal,
    handleViewConfirmations,
    handleConfirmStatus,
  }
}

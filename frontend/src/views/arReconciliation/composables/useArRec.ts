/**
 * useArRec.ts - AR 对账核心 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供对账结果列表、详情、客户确认等业务方法
 * 争议管理由 useArDisp 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { i18n } from '@/i18n'
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

const t = i18n.global.t.bind(i18n.global)

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
      const res = await getAutoReconciliationResults({
        page: pagination.value.page,
        page_size: pagination.value.pageSize,
        customer_name: searchForm.value.customer_name || undefined,
        status: searchForm.value.match_status || undefined,
        start_date: searchForm.value.start_date || undefined,
        end_date: searchForm.value.end_date || undefined,
      })
      tableData.value = res.data?.list || []
      total.value = res.data?.total || 0
    } catch {
      ElMessage.error(t('arReconciliationModule.loadResultsFailed'))
    } finally {
      loading.value = false
    }
  }

  /** 启动自动对账 */
  const handleAutoReconcile = async () => {
    if (!searchForm.value.start_date || !searchForm.value.end_date) {
      ElMessage.warning(t('arReconciliationModule.selectDateRange'))
      return
    }
    try {
      await ElMessageBox.confirm(t('arReconciliationModule.confirmAutoReconcile'), t('common.message.confirmTitle'), { type: 'info' })
      reconcileLoading.value = true
      await autoReconcile({
        start_date: searchForm.value.start_date,
        end_date: searchForm.value.end_date,
      })
      ElMessage.success(t('arReconciliationModule.autoReconcileStarted'))
      await loadData()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        ElMessage.error(t('arReconciliationModule.startReconcileFailed'))
      }
    } finally {
      reconcileLoading.value = false
    }
  }

  /** 查看对账明细 */
  const handleViewDetail = async (row: AutoReconciliationResult) => {
    try {
      const res = await getReconciliationDetailItems(row.id)
      detailData.value = res.data || []
      currentReconciliation.value = row
      detailDialogVisible.value = true
    } catch {
      ElMessage.error(t('arReconciliationModule.fetchDetailFailed'))
    }
  }

  /** 发送客户对账确认 */
  const handleSendConfirmation = async (row: AutoReconciliationResult) => {
    try {
      await ElMessageBox.confirm(t('arReconciliationModule.confirmSendConfirmation'), t('common.message.confirmTitle'), { type: 'info' })
      await sendCustomerConfirmation(row.id)
      ElMessage.success(t('arReconciliationModule.confirmationSent'))
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        ElMessage.error(t('arReconciliationModule.sendConfirmationFailed'))
      }
    }
  }

  /** 加载客户确认记录 */
  const handleViewConfirmations = async () => {
    try {
      const res = await getCustomerConfirmations({
        page: 1,
        page_size: 20,
      })
      confirmData.value = res.data?.list || []
      confirmTotal.value = res.data?.total || 0
      confirmDialogVisible.value = true
    } catch {
      ElMessage.error(t('arReconciliationModule.loadConfirmationsFailed'))
    }
  }

  /** 更新客户确认状态 */
  const handleConfirmStatus = async (
    row: CustomerConfirmation,
    status: 'confirmed' | 'disputed'
  ) => {
    const msg = status === 'confirmed'
      ? t('arReconciliationModule.confirmThisRecord')
      : t('arReconciliationModule.markAsDisputed')
    try {
      await ElMessageBox.confirm(msg, t('common.message.confirmTitle'), { type: 'warning' })
      await updateConfirmationStatus(row.id, { status })
      ElMessage.success(t('common.message.operationSuccess'))
      await handleViewConfirmations()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        ElMessage.error(t('common.failed'))
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

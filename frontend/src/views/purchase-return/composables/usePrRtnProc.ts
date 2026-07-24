/**
 * usePrRtnProc.ts - 采购退货业务流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货提交流程（提交审批/审批/拒绝/删除）操作
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  submitPurchaseReturn,
  approvePurchaseReturn,
  rejectPurchaseReturn,
  deletePurchaseReturn,
  type PurchaseReturn,
} from '@/api/purchase-return'
import { logger } from '@/utils/logger'

/**
 * 采购退货流程 composable
 * 集中管理审批对话框状态、审批/拒绝/提交/删除等业务流程
 */
export function usePrRtnProc(deps: { fetchData: () => Promise<void> }) {
  // 审批对话框
  const approveDialogVisible = ref(false)
  const approveForm = reactive({
    id: 0,
    remark: '',
  })

  /** 提交退货单（draft → pending） */
  const handleSubmit = async (row: PurchaseReturn) => {
    try {
      await ElMessageBox.confirm('确定要提交该退货单吗？', '提示', { type: 'warning' })
      await submitPurchaseReturn(row.id!)
      ElMessage.success('提交成功')
      await deps.fetchData()
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('提交失败:', error)
      }
    }
  }

  /** 打开审批对话框 */
  const openApprove = (row: PurchaseReturn) => {
    approveForm.id = row.id!
    approveForm.remark = ''
    approveDialogVisible.value = true
  }

  /** 审批通过 */
  const handleApproveConfirm = async () => {
    try {
      await approvePurchaseReturn(approveForm.id)
      ElMessage.success('审批通过')
      approveDialogVisible.value = false
      await deps.fetchData()
    } catch (error) {
      logger.error('审批失败:', error)
    }
  }

  /** 审批拒绝 */
  const handleReject = async () => {
    try {
      await rejectPurchaseReturn(approveForm.id, approveForm.remark)
      ElMessage.success('已拒绝')
      approveDialogVisible.value = false
      await deps.fetchData()
    } catch (error) {
      logger.error('拒绝失败:', error)
    }
  }

  /** 删除退货单 */
  const handleDelete = async (row: PurchaseReturn) => {
    try {
      await ElMessageBox.confirm('确定要删除该退货单吗？', '提示', { type: 'warning' })
      await deletePurchaseReturn(row.id!)
      ElMessage.success('删除成功')
      await deps.fetchData()
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('删除失败:', error)
      }
    }
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 审批对话框
    approveDialogVisible,
    approveForm,
    openApprove,
    handleApproveConfirm,
    handleReject,
    // 流程
    handleSubmit,
    handleDelete,
  })
}

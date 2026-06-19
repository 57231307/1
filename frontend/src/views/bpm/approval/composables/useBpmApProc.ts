/**
 * useBpmApProc.ts - BPM 审批流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 封装审批 / 转交 / 审批链等流程性方法与对话框状态
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { bpmEnhancedApi, type ApprovalTask, type ApprovalChainNode } from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'

/**
 * 刷新回调
 */
interface RefreshCallbacks {
  fetchPendingTasks: () => Promise<void>
}

/**
 * 审批流程操作方法集合
 */
export function useBpmApProc(refresh: RefreshCallbacks) {
  // 当前任务
  const currentTask = ref<ApprovalTask | null>(null)

  // 审批对话框
  const approveDialogVisible = ref(false)
  const approveAction = ref<'approve' | 'reject'>('approve')
  const submitLoading = ref(false)
  const approveForm = reactive({ comment: '' })

  // 转交对话框
  const transferDialogVisible = ref(false)
  const transferFormRef = ref<FormInstance>()
  const transferForm = reactive({ target_user_id: 1, comment: '' })
  const transferRules: FormRules = {
    target_user_id: [{ required: true, message: '请输入接收人 ID', trigger: 'blur' }],
  }

  // 审批链对话框
  const chainDialogVisible = ref(false)
  const approvalChain = ref<ApprovalChainNode[]>([])

  /** 打开同意审批对话框 */
  const handleApprove = (row: ApprovalTask) => {
    currentTask.value = row
    approveAction.value = 'approve'
    approveForm.comment = ''
    approveDialogVisible.value = true
  }

  /** 打开拒绝审批对话框 */
  const handleReject = (row: ApprovalTask) => {
    currentTask.value = row
    approveAction.value = 'reject'
    approveForm.comment = ''
    approveDialogVisible.value = true
  }

  /** 确认审批 */
  const confirmApproval = async () => {
    if (!currentTask.value) return
    submitLoading.value = true
    try {
      await bpmEnhancedApi.executeApproval({
        task_id: currentTask.value.task_id,
        action: approveAction.value,
        comment: approveForm.comment,
      })
      ElMessage.success(approveAction.value === 'approve' ? '审批通过' : '审批拒绝')
      approveDialogVisible.value = false
      refresh.fetchPendingTasks()
    } catch (e) {
      logger.error(String(e))
    } finally {
      submitLoading.value = false
    }
  }

  /** 打开转交对话框 */
  const handleTransfer = (row: ApprovalTask) => {
    currentTask.value = row
    transferForm.target_user_id = 1
    transferForm.comment = ''
    transferDialogVisible.value = true
  }

  /** 确认转交 */
  const confirmTransfer = async () => {
    if (!currentTask.value || !transferFormRef.value) return
    await transferFormRef.value.validate(async valid => {
      if (!valid) return
      submitLoading.value = true
      try {
        await bpmEnhancedApi.executeApproval({
          task_id: currentTask.value!.task_id,
          action: 'transfer',
          target_user_id: transferForm.target_user_id,
          comment: transferForm.comment,
        })
        ElMessage.success('转交成功')
        transferDialogVisible.value = false
        refresh.fetchPendingTasks()
      } catch (e) {
        logger.error(String(e))
      } finally {
        submitLoading.value = false
      }
    })
  }

  /** 打开审批链对话框并加载数据 */
  const handleViewChain = async (row: ApprovalTask) => {
    currentTask.value = row
    chainDialogVisible.value = true
    try {
      const res = await bpmEnhancedApi.getApprovalChain(row.process_instance_id)
      approvalChain.value = res.data
    } catch (e) {
      logger.error(String(e))
    }
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 当前任务
    currentTask,
    // 审批对话框
    approveDialogVisible,
    approveAction,
    submitLoading,
    approveForm,
    handleApprove,
    handleReject,
    confirmApproval,
    // 转交对话框
    transferDialogVisible,
    transferFormRef,
    transferForm,
    transferRules,
    handleTransfer,
    confirmTransfer,
    // 审批链对话框
    chainDialogVisible,
    approvalChain,
    handleViewChain,
  })
}

/**
 * useArDisp.ts - AR 对账争议管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供争议对话框、提交争议、解决争议等业务方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { i18n } from '@/i18n'
import {
  createDispute,
  getDisputes,
  resolveDispute,
  type AutoReconciliationResult,
  type DisputeRecord,
} from '@/api/ar-reconciliation-enhanced'
import { logger } from '@/utils/logger'

const t = i18n.global.t.bind(i18n.global)

/**
 * 争议管理 composable
 * @param loadData 提交争议后刷新列表方法
 */
export function useArDisp(loadData: () => Promise<void>) {
  const disputeDialogVisible = ref(false)
  const disputeForm = ref<Partial<DisputeRecord>>({
    dispute_type: 'amount',
    dispute_amount: 0,
    description: '',
    status: 'open',
  })
  const disputes = ref<DisputeRecord[]>([])
  const disputesTotal = ref(0)

  /** 打开争议对话框并加载已有争议 */
  const openDisputeDialog = async (row: AutoReconciliationResult) => {
    disputeForm.value = {
      dispute_type: 'amount',
      dispute_amount: 0,
      description: '',
      status: 'open',
      reconciliation_id: row.id,
    }
    disputes.value = []
    try {
      const res: Awaited<ReturnType<typeof getDisputes>> = await getDisputes({ page: 1, page_size: 10 })
      disputes.value = res.data?.list || []
      disputesTotal.value = res.data?.total || 0
    } catch {
      logger.warn(t('arReconciliationModule.loadDisputesFailed'))
    }
    disputeDialogVisible.value = true
  }

  /** 提交争议 */
  const handleSubmitDispute = async () => {
    if (!disputeForm.value.description) {
      ElMessage.warning(t('arReconciliationModule.disputeDescriptionRequired'))
      return
    }
    try {
      await createDispute(disputeForm.value)
      ElMessage.success(t('arReconciliationModule.disputeSubmitted'))
      disputeDialogVisible.value = false
      await loadData()
    } catch {
      ElMessage.error(t('arReconciliationModule.submitDisputeFailed'))
    }
  }

  /** 解决争议 */
  const handleResolveDispute = async (row: DisputeRecord) => {
    try {
      const { value } = await ElMessageBox.prompt(t('arReconciliationModule.enterResolution'), t('arReconciliationModule.resolveDisputeTitle'), {
        inputType: 'textarea',
        inputValidator: v => (!v ? t('arReconciliationModule.resolutionRequired') : true),
      })
      await resolveDispute(row.id, { resolution: value })
      ElMessage.success(t('arReconciliationModule.disputeResolved'))
      await openDisputeDialog({ id: row.reconciliation_id } as AutoReconciliationResult)
    } catch (error: unknown) {
      if (error !== 'cancel') {
        ElMessage.error(t('arReconciliationModule.resolveDisputeFailed'))
      }
    }
  }

  return {
    disputeDialogVisible,
    disputeForm,
    disputes,
    disputesTotal,
    openDisputeDialog,
    handleSubmitDispute,
    handleResolveDispute,
  }
}

/**
 * useArDisp.ts - AR 对账争议管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供争议对话框、提交争议、解决争议等业务方法
 */
import { ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { i18n } from '@/i18n';
import {
  createDispute,
  getDisputes,
  resolveDispute,
  type AutoReconciliationResult,
  type CreateDisputePayload,
  type DisputeRecord,
} from '@/api/ar-reconciliation-enhanced';
import { logger } from '@/utils/logger';

const t = i18n.global.t.bind(i18n.global);

/**
 * 争议管理 composable
 * @param loadData 提交争议后刷新列表方法
 */
export function useArDisp(loadData: () => Promise<void>) {
  const disputeDialogVisible = ref(false);
  // 后端 CreateDisputeApiRequest 只读 reconciliation_id / customer_id / reason / description，
  // 原 dispute_type/dispute_amount/status 采集键在后端 DTO 与数据表中均无对应（提交即被丢弃），
  // 已移除；争议原因唯一落库通道是 description -> dispute_reason
  const disputeForm = ref<CreateDisputePayload>({
    reconciliation_id: 0,
    description: '',
  });
  const disputes = ref<DisputeRecord[]>([]);
  const disputesTotal = ref(0);

  /** 打开争议对话框并加载已有争议 */
  const openDisputeDialog = async (row: AutoReconciliationResult) => {
    disputeForm.value = {
      reconciliation_id: row.id,
      description: '',
    };
    disputes.value = [];
    try {
      const res: Awaited<ReturnType<typeof getDisputes>> = await getDisputes({
        page: 1,
        page_size: 10,
      });
      // 后端 list_disputes 以 json!({"list","total",...}) 承载，显式读取 list
      disputes.value = res.data.list;
      disputesTotal.value = res.data.total;
    } catch {
      logger.warn(t('arReconciliationModule.loadDisputesFailed'));
    }
    disputeDialogVisible.value = true;
  };

  /** 提交争议 */
  const handleSubmitDispute = async () => {
    // reconciliation_id 虽为 Option 反序列化字段，handler 判空后直接 400；
    // 顶部按钮打开对话框时无行数据（id=0），必须先选中具体对账单
    if (!disputeForm.value.reconciliation_id) {
      ElMessage.warning(t('arReconciliationModule.disputeTargetRequired'));
      return;
    }
    if (!disputeForm.value.description) {
      ElMessage.warning(t('arReconciliationModule.disputeDescriptionRequired'));
      return;
    }
    try {
      await createDispute({
        reconciliation_id: disputeForm.value.reconciliation_id,
        description: disputeForm.value.description,
      });
      ElMessage.success(t('arReconciliationModule.disputeSubmitted'));
      disputeDialogVisible.value = false;
      await loadData();
    } catch {
      ElMessage.error(t('arReconciliationModule.submitDisputeFailed'));
    }
  };

  /** 解决争议：dispute 记录即对账单本身（id 为 reconciliation id） */
  const handleResolveDispute = async (row: DisputeRecord) => {
    try {
      const { value } = await ElMessageBox.prompt(
        t('arReconciliationModule.enterResolution'),
        t('arReconciliationModule.resolveDisputeTitle'),
        {
          inputType: 'textarea',
          inputValidator: v => (!v ? t('arReconciliationModule.resolutionRequired') : true),
        }
      );
      await resolveDispute(row.id, { resolution: value });
      ElMessage.success(t('arReconciliationModule.disputeResolved'));
      // 解决后刷新当前对账单的争议列表：row.id 即 reconciliation_id
      await openDisputeDialog({ id: row.id } as AutoReconciliationResult);
    } catch (error: unknown) {
      if (error !== 'cancel') {
        ElMessage.error(t('arReconciliationModule.resolveDisputeFailed'));
      }
    }
  };

  return {
    disputeDialogVisible,
    disputeForm,
    disputes,
    disputesTotal,
    openDisputeDialog,
    handleSubmitDispute,
    handleResolveDispute,
  };
}

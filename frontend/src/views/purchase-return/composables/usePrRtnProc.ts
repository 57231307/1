/**
 * usePrRtnProc.ts - 采购退货业务流程 composable
 * 提供采购退货提交流程（提交审批/审批/拒绝/删除）操作
 */
import { ref, reactive } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { promptApprovalReason, promptRejectReason } from '@/composables/useActionPrompts';
import {
  submitPurchaseReturn,
  approvePurchaseReturn,
  rejectPurchaseReturn,
  deletePurchaseReturn,
  type PurchaseReturn,
} from '@/api/purchase-return';
import { logger } from '@/utils/logger';

/**
 * 采购退货流程 composable
 * 集中管理审批对话框状态、审批/拒绝/提交/删除等业务流程
 */
export function usePrRtnProc(deps: { fetchData: () => Promise<void> }) {
  // 审批对话框
  const approveDialogVisible = ref(false);
  const approveForm = reactive({
    id: 0,
    remark: '',
  });

  /** 提交退货单（draft → pending） */
  const handleSubmit = async (row: PurchaseReturn) => {
    try {
      await ElMessageBox.confirm('确定要提交该退货单吗？', '提示', { type: 'warning' });
      await submitPurchaseReturn(row.id!);
      msg.success('submitSuccess');
      await deps.fetchData();
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('提交失败:', error);
      }
    }
  };

  /** 打开审批对话框 */
  const openApprove = (row: PurchaseReturn) => {
    approveForm.id = row.id!;
    approveForm.remark = '';
    approveDialogVisible.value = true;
  };

  /**
   * 审批通过（submitted → approved），理由选填：先经 promptApprovalReason(false) 采集，
   * 取消即中止（非错误，不记错误日志）。留空时省略 approval_reason 键，
   * purchase_return.approval_reason 列写 NULL（不伪造空串）。
   */
  const handleApproveConfirm = async () => {
    const approvalReason = await promptApprovalReason(false);
    if (approvalReason === null) return;
    try {
      await approvePurchaseReturn(approveForm.id, approvalReason);
      msg.success('approveSuccess');
      approveDialogVisible.value = false;
      await deps.fetchData();
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('approveFailed'));
      logger.error('审批失败:', error);
    }
  };

  /**
   * 审批拒绝（→ rejected）：拒绝理由后端必填（trim 非空），先经 promptRejectReason() 采集再提交，
   * 取消即中止；理由落 purchase_return.rejected_reason 专列，服务端定性 400 文案原样透出。
   */
  const handleReject = async () => {
    const reason = await promptRejectReason();
    if (reason === null) return;
    try {
      await rejectPurchaseReturn(approveForm.id, reason);
      msg.success('rejectSuccess');
      approveDialogVisible.value = false;
      await deps.fetchData();
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('rejectFailed'));
      logger.error('拒绝失败:', error);
    }
  };

  /** 删除退货单 */
  const handleDelete = async (row: PurchaseReturn) => {
    try {
      await ElMessageBox.confirm('确定要删除该退货单吗？', '提示', { type: 'warning' });
      await deletePurchaseReturn(row.id!);
      msg.success('deleteSuccess');
      await deps.fetchData();
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('删除失败:', error);
      }
    }
  };

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
  });
}

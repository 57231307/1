/**
 * usePrRtnProc.ts - 采购退货业务流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货提交流程（提交审批/审批/拒绝/删除）操作
 * 行为完全保持一致（仅结构重构）
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
   * 审批通过（submitted → approved）：通过理由选填。
   * 交互：先经统一采集器 promptApprovalReason(false) 采集理由（允许留空→省略该键，后端归一为 NULL），
   * 取消即中止整条链（非错误，不记错误日志）。
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
   * 审批拒绝（→ rejected，理由落 rejected_reason 专列 TEXT 无上限）：拒绝理由后端必填（trim 非空）。
   * 交互：改用统一采集器 promptRejectReason()（此前经对话框可空提交，会把空 reason 发给现已收口必填的
   * 服务端 → 必收 400），取消即中止；服务端定性 400 文案原样透出（只说该做什么，不带字段名/机制名词）。
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

/**
 * useSrProc.ts - 销售退货业务流程 composable
 * 任务编号: P14 批 2 I-3 第 7 批（拆分原 sales-returns/index.vue）
 * 提供审核、查看、提交、新建/编辑等业务流程方法
 * 列表/表单/CRUD 状态由 useSr 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import {
  approveSalesReturn,
  submitSalesReturn,
  rejectSalesReturn,
  executeSalesReturn,
  type SalesReturn,
} from '@/api/sales-return';

/**
 * 销售退货业务流程 composable
 * 接收 useSr 提供的状态与基础方法
 */
export function useSrProc(sr: ReturnType<typeof import('./useSr').useSr>) {
  // 详情对话框可见性（父组件可读取控制 v-model）
  const viewDialogVisible = ref(false);

  /**
   * 触发查看详情：拷贝行数据到 currentReturn，打开弹窗
   */
  const handleView = (row: SalesReturn) => {
    sr.currentReturn.value = { ...row };
    viewDialogVisible.value = true;
  };

  /**
   * 进入新建模式
   */
  const handleCreate = () => {
    sr.resetFormForCreate();
  };

  /**
   * 进入编辑模式
   */
  const handleEdit = (row: SalesReturn) => {
    sr.fillFormForEdit(row);
  };

  /**
   * 提交（新建或更新）
   */
  const handleSubmit = async (dialogMode: 'create' | 'edit') => {
    return await sr.submitForm(dialogMode);
  };

  /**
   * 审核通过退货单
   */
  const handleApprove = async (row: SalesReturn) => {
    if (!row.id) return;

    try {
      await ElMessageBox.confirm(`确定审核通过退货单 ${row.returnNo} 吗？`, '审核确认', {
        type: 'warning',
      });
      await approveSalesReturn(row.id);
      msg.success('auditSuccess');
      await sr.loadReturns();
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('auditFailed'));
      }
    }
  };

  /**
   * 提交退货单进入审批（DRAFT → SUBMITTED）
   */
  const handleSubmitForApproval = async (row: SalesReturn) => {
    if (!row.id) return;
    try {
      await ElMessageBox.confirm(`确定提交退货单 ${row.returnNo} 吗？`, '提交确认', {
        type: 'warning',
      });
      await submitSalesReturn(row.id);
      msg.success('auditSuccess');
      await sr.loadReturns();
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('auditFailed'));
      }
    }
  };

  /**
   * 驳回退货单（SUBMITTED → DRAFT）
   */
  const handleReject = async (row: SalesReturn) => {
    if (!row.id) return;
    try {
      const { value } = await ElMessageBox.prompt('请输入驳回原因', `驳回 ${row.returnNo}`, {
        type: 'warning',
        inputPattern: /\S+/,
        inputErrorMessage: '驳回原因不能为空',
      });
      await rejectSalesReturn(row.id, value);
      msg.success('auditSuccess');
      await sr.loadReturns();
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('auditFailed'));
      }
    }
  };

  /**
   * 执行退货（APPROVED → COMPLETED，真实扣减销售订单库存/生成入库）
   */
  const handleExecute = async (row: SalesReturn) => {
    if (!row.id) return;
    try {
      await ElMessageBox.confirm(`确定执行退货单 ${row.returnNo} 吗？`, '执行确认', {
        type: 'warning',
      });
      await executeSalesReturn(row.id);
      msg.success('auditSuccess');
      await sr.loadReturns();
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('auditFailed'));
      }
    }
  };

  return {
    viewDialogVisible,
    handleView,
    handleCreate,
    handleEdit,
    handleSubmit,
    handleApprove,
    handleSubmitForApproval,
    handleReject,
    handleExecute,
  };
}

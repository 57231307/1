/**
 * usePcProc.ts - 采购合同流程操作 composable
 * 封装采购合同提交审批/审批（通过/拒绝）/执行/删除/导出等流程性方法
 */
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  promptApprovalReason,
  promptRejectReason,
  promptContractExecute,
} from '@/composables/useActionPrompts';
import {
  deletePurchaseContract,
  approvePurchaseContract,
  rejectPurchaseContract,
  executePurchaseContract,
  exportPurchaseContracts,
  type PurchaseContract,
} from '@/api/purchase-contract';
import { logger } from '@/utils/logger';

/** 刷新回调 */
interface RefreshCallbacks {
  getList: () => Promise<void>;
}

/**
 * 采购合同流程操作方法集合
 */
export function usePcProc(refresh: RefreshCallbacks) {
  /**
   * 审批通过（draft → active，仅 draft 可审）：理由必填，先经 promptApprovalReason(true) 采集再提交，
   * 取消采集框即中止整条链（不是失败，不弹错误）。
   * 端点 POST /purchase/purchase-contracts/{id}/approve，理由落 purchase_contracts.approval_reason 列；
   * 拒绝走独立端点（见 handleReject），禁止用本端点兼职放行拒绝。
   * 端点成功出参是含记录 ID 的后端拼接文案 ⇒ 不外显，提示一律走 i18n。
   */
  const handleApprove = async (row: PurchaseContract) => {
    const approvalReason = await promptApprovalReason(true);
    if (approvalReason === null) return;
    try {
      await approvePurchaseContract(row.id, { approval_reason: approvalReason });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('采购合同审批通过失败:', error);
      msg.error('approveFailed');
    }
  };

  /**
   * 提交审批入口。后端合同状态机只有 draft→active（approve）与 draft→rejected（reject），
   * 无「待审批」中间态、也没有独立提交端点 ⇒ 本入口与「审批通过」是同一端点同一动作，
   * 直接复用 handleApprove 的理由采集与提交，不发空体（通过理由必填，缺体/空白一律 400）。
   */
  const handleSubmit = (row: PurchaseContract) => handleApprove(row);

  /**
   * 审批拒绝（draft → rejected 终态，与作废 cancelled 语义不可互替，仅 draft 可拒）：
   * 理由必填，先经 promptRejectReason() 采集再提交，取消采集框即中止整条链。
   * 端点 POST /purchase/purchase-contracts/{id}/reject（与 approve 各自独立），
   * 理由落 purchase_contracts.rejected_reason 列。
   * 端点成功出参是含记录 ID 的后端拼接文案 ⇒ 不外显，提示一律走 i18n。
   */
  const handleReject = async (row: PurchaseContract) => {
    const reason = await promptRejectReason();
    if (reason === null) return;
    try {
      await rejectPurchaseContract(row.id, { reason });
      msg.success('rejectSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('采购合同审批拒绝失败:', error);
      msg.error('rejectFailed');
    }
  };

  /** 执行 */
  const handleExecute = async (row: PurchaseContract) => {
    // 后端执行请求必填 execution_type/execution_amount/execution_date：
    // 逐项弹框真实采集，取消即中断，绝不塞默认值；execution_type 取值 = purchase_contract_execution 词表（PARTIAL/COMPLETE）。
    const form = await promptContractExecute(
      [
        { value: 'PARTIAL', label: i18n.global.t('actionForm.executeTypePartial') },
        { value: 'COMPLETE', label: i18n.global.t('actionForm.executeTypeComplete') },
      ],
      true
    );
    if (!form || !form.execution_date) return;
    try {
      await executePurchaseContract(row.id, {
        execution_type: form.execution_type,
        execution_amount: form.execution_amount,
        execution_date: form.execution_date,
        remark: form.remark,
      });
      msg.success('executeSuccess');
      await refresh.getList();
    } catch (error) {
      logger.error('执行失败:', error);
    }
  };

  /** 删除 */
  const handleDelete = async (row: PurchaseContract) => {
    try {
      await ElMessageBox.confirm('确认删除该合同？', '提示', { type: 'warning' });
      await deletePurchaseContract(row.id);
      msg.success('deleteSuccess');
      await refresh.getList();
    } catch (error) {
      logger.error('删除失败:', error);
    }
  };

  /** 导出采购合同 xlsx（GET /purchase/purchase-contracts/export 返回 blob，触发浏览器下载） */
  const handleExport = async () => {
    try {
      const blob = await exportPurchaseContracts();
      const url = window.URL.createObjectURL(new Blob([blob]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', `采购合同_${new Date().toISOString().split('T')[0]}.xlsx`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
      msg.success('exportSuccess');
    } catch (error) {
      logger.error('导出失败:', error);
      msg.error('exportFailed');
    }
  };

  return {
    handleSubmit,
    handleApprove,
    handleReject,
    handleExecute,
    handleDelete,
    handleExport,
  };
}

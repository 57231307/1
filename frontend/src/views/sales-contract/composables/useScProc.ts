/**
 * useScProc.ts - 销售合同流程操作 composable
 * 封装销售合同提交审批/审批（通过/拒绝）/执行/删除/打印/导出/查看等流程性方法
 */
import { isDialogDismissal } from '@/utils/monitor';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';
import {
  promptApprovalReason,
  promptRejectReason,
  promptContractExecute,
} from '@/composables/useActionPrompts';
import {
  deleteSalesContract,
  approveSalesContract,
  rejectSalesContract,
  executeSalesContract,
  type SalesContract,
} from '@/api/sales-contract';
import { formatCurrency, getStatusLabel } from './scFmts';
import { escapeHtml } from '@/utils/print';
// 导出走后端带水印 xlsx 接口
import { exportFromBackend } from '@/utils/export';

/**
 * 合同金额归一：后端 total_amount 为 Decimal ⇒ 出参 JSON 字符串（或 null），
 * formatCurrency 只接受 number，故消费处 Number() 归一、null 透传，禁止对字符串直接 .toFixed。
 */
const fmtContractAmount = (amount: string | null) =>
  formatCurrency(amount == null ? null : Number(amount));

/**
 * 刷新回调
 * getQueryParams：导出时透传列表筛选条件（keyword/status/customer_id），保证导出与列表同口径
 */
interface RefreshCallbacks {
  getList: () => Promise<void>;
  getQueryParams?: () => { keyword?: string; status?: string; customer_id?: number };
}

/**
 * 销售合同流程操作方法集合
 */
export function useScProc(refresh: RefreshCallbacks) {
  /**
   * 审批通过（draft → active，仅 draft 可审）：理由必填，先经 promptApprovalReason(true) 采集再提交，
   * 取消采集框即中止整条链（不是失败，不弹错误）。
   * 端点 POST /sales/sales-contracts/{id}/approve，理由落 sales_contracts.approval_reason 列；
   * 拒绝走独立端点（见 handleReject），禁止用本端点兼职放行拒绝。
   * 端点成功出参是含记录 ID 的后端拼接文案 ⇒ 不外显，提示一律走 i18n。
   */
  const handleApprove = async (row: SalesContract) => {
    const approvalReason = await promptApprovalReason(true);
    if (approvalReason === null) return;
    try {
      await approveSalesContract(row.id, { approval_reason: approvalReason });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('销售合同审批通过失败:', error);
      msg.error('approveFailed');
    }
  };

  /**
   * 提交审批入口。后端合同状态机只有 draft→active（approve）与 draft→rejected（reject），
   * 无「待审批」中间态、也没有独立提交端点 ⇒ 本入口与「审批通过」是同一端点同一动作，
   * 直接复用 handleApprove 的理由采集与提交，不发空体（通过理由必填，缺体/空白一律 400）。
   */
  const handleSubmitForApproval = (row: SalesContract) => handleApprove(row);

  /**
   * 审批拒绝（draft → rejected 终态，与作废 cancelled 语义不可互替，仅 draft 可拒）：
   * 理由必填，先经 promptRejectReason() 采集再提交，取消采集框即中止整条链。
   * 端点 POST /sales/sales-contracts/{id}/reject（与 approve 各自独立），
   * 理由落 sales_contracts.rejected_reason 列。
   * 端点成功出参是含记录 ID 的后端拼接文案 ⇒ 不外显，提示一律走 i18n。
   */
  const handleReject = async (row: SalesContract) => {
    const reason = await promptRejectReason();
    if (reason === null) return;
    try {
      await rejectSalesContract(row.id, { reason });
      msg.success('rejectSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('销售合同审批拒绝失败:', error);
      msg.error('rejectFailed');
    }
  };

  /** 执行 */
  const handleExecute = async (row: SalesContract) => {
    // 后端执行请求必填 execution_type/execution_amount（无执行日期字段）。
    // execution_type 后端强校验，仅接受 delivery（出库）/ payment（收款）。
    const form = await promptContractExecute(
      [
        { value: 'delivery', label: i18n.global.t('actionForm.executeTypeDelivery') },
        { value: 'payment', label: i18n.global.t('actionForm.executeTypePayment') },
      ],
      false
    );
    if (!form) return;
    try {
      await executeSalesContract(row.id, {
        execution_type: form.execution_type,
        execution_amount: form.execution_amount,
        remark: form.remark,
      });
      msg.success('executeSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('executeFailed'));
    }
  };

  /** 删除 */
  const handleDelete = async (row: SalesContract) => {
    try {
      await ElMessageBox.confirm('确认删除该合同？', '提示', { type: 'warning' });
      await deleteSalesContract(row.id);
      msg.success('deleteSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      if (!isDialogDismissal(error)) {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('deleteFailed'));
      }
    }
  };

  /** 查看详情（弹出 ElMessageBox） */
  const handleView = (row: SalesContract) => {
    ElMessageBox.alert(
      `<div>
        <p><strong>合同编号：</strong>${row.contract_no}</p>
        <p><strong>合同名称：</strong>${row.contract_name}</p>
        <p><strong>客户：</strong>${row.customer_name || '-'}</p>
        <p><strong>合同金额：</strong>${fmtContractAmount(row.total_amount)}</p>
        <p><strong>签订日期：</strong>${row.signed_date || '-'}</p>
        <p><strong>生效日期：</strong>${row.effective_date || '-'}</p>
        <p><strong>到期日期：</strong>${row.expiry_date || '-'}</p>
        <p><strong>付款条件：</strong>${row.payment_terms || '-'}</p>
        <p><strong>付款方式：</strong>${row.payment_method || '-'}</p>
        <p><strong>交货日期：</strong>${row.delivery_date || '-'}</p>
        <p><strong>交货地点：</strong>${row.delivery_location || '-'}</p>
      </div>`,
      '合同详情',
      { dangerouslyUseHTMLString: true, confirmButtonText: '关闭' }
    );
  };

  /** 打印当前列表 */
  const handlePrint = (contractList: { value: SalesContract[] } | SalesContract[]) => {
    const list = Array.isArray(contractList) ? contractList : contractList.value;
    const printWindow = window.open('', '_blank');
    if (!printWindow) {
      msg.error('printWindowBlocked');
      return;
    }
    const rows = list
      .map(
        (item: SalesContract) => `
      <tr>
        <td>${escapeHtml(item.contract_no)}</td>
        <td>${escapeHtml(item.contract_name)}</td>
        <td>${escapeHtml(item.customer_name)}</td>
        <td style="text-align:right">${fmtContractAmount(item.total_amount)}</td>
        <td>${escapeHtml(item.signed_date || '-')}</td>
        <td>${escapeHtml(getStatusLabel(item.status))}</td>
      </tr>
    `
      )
      .join('');
    const now = new Date().toISOString().split('T')[0];
    printWindow.document.write(`
      <html><head><meta charset="utf-8"><title>销售合同列表</title>
      <style>
        @media print { @page { size: landscape; } }
        body { font-family: "Microsoft YaHei", sans-serif; font-size: 12px; }
        h1 { text-align: center; }
        table { width: 100%; border-collapse: collapse; margin-top: 12px; }
        th, td { border: 1px solid #333; padding: 6px 8px; }
        th { background: #f5f5f5; }
        .meta { text-align: center; color: #666; font-size: 11px; }
      </style></head><body>
      <h1>销售合同列表</h1>
      <div class="meta">打印日期: ${now} | 共 ${list.length} 条</div>
      <table>
        <thead><tr><th>合同编号</th><th>合同名称</th><th>客户</th><th>金额</th><th>签订日期</th><th>状态</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
      </body></html>
    `);
    printWindow.document.close();
    printWindow.onload = () => printWindow.print();
  };

  /**
   * 导出 Excel（统一 xlsx，禁止 CSV 交付）：
   * 调用后端 GET /sales/sales-contracts/export（后端注入水印 + 异步审计日志），
   * 透传当前列表筛选条件（keyword/status/customer_id），与列表同口径。
   */
  const handleExport = async () => {
    const filters = refresh.getQueryParams?.() ?? {};
    const params: Record<string, unknown> = {
      keyword: filters.keyword || undefined,
      status: filters.status || undefined,
      customer_id: filters.customer_id,
    };
    await exportFromBackend('/sales/sales-contracts/export', params, 'sales_contracts_export');
  };

  return {
    handleSubmitForApproval,
    handleApprove,
    handleReject,
    handleExecute,
    handleDelete,
    handleView,
    handlePrint,
    handleExport,
  };
}

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
import { h, type VNode } from 'vue';

/**
 * 合同金额归一：后端 total_amount 为 Decimal ⇒ 出参 JSON 字符串（或 null），
 * formatCurrency 只接受 number，故消费处 Number() 归一、null 透传，禁止对字符串直接 .toFixed。
 */
const fmtContractAmount = (amount: string | null) =>
  formatCurrency(amount == null ? null : Number(amount));

/**
/**
 * composable 内取 i18n 文案（非组件环境无 setup 作用域，走 i18n.global，
 * 与本文件执行类型选项 actionForm.executeType* 的取法同源）。
 * @param key 文案键；named 可选具名插值参数
 * @returns 当前语言下的文案
 */
const tr = (key: string, named?: Record<string, string | number>): string =>
  named === undefined ? i18n.global.t(key) : i18n.global.t(key, named);

/**
 * 构造销售合同详情弹层的消息 VNode。
 * 功能：把标签与值逐字段渲染成行，标签取 salesContract.proc.label* 文案键，值一律走 Vue
 * 文本节点进 DOM、绝不拼 HTML 字符串，使人工程录入字段（编号/名称/客户/付款条件/付款方式/
 * 交货地点等）携带标签片段时只按字面显示，杜绝弹层被当 HTML 解析执行的结构型注入面。
 * 调用方：handleView（经 ElMessageBox.alert 的 VNode 消息通道渲染）。
 * 入参：row 销售合同行对象；文本字段可空（空值显示 '-'），total_amount 为 Decimal 字符串，
 * 经 fmtContractAmount 归一后作文本值渲染；状态取 utils/sales-contract-status 权威词表
 * （经 scFmts::getStatusLabel），不外显裸枚举。
 * 传给谁：ElMessageBox.alert 的 message 形参。
 * 存什么：不保存状态；同入参必同出参。
 * 存哪里：本文件导出；后续结构化弹层复用本入口，禁止在调用点另写私有拼装副本。
 */
export const buildScContractDetailMessage = (row: SalesContract): VNode => {
  /** 单字段行：strong 标签 + 文本节点值（空值占位 '-'） */
  const fieldRow = (labelKey: string, value: string | null | undefined) =>
    h('p', [h('strong', `${tr(labelKey)}: `), value || '-']);
  return h('div', [
    fieldRow('salesContract.proc.labelContractNo', row.contract_no),
    fieldRow('salesContract.proc.labelContractName', row.contract_name),
    fieldRow('salesContract.proc.labelCustomer', row.customer_name),
    fieldRow('salesContract.proc.labelTotalAmount', fmtContractAmount(row.total_amount)),
    fieldRow('salesContract.proc.labelSignedDate', row.signed_date),
    fieldRow('salesContract.proc.labelEffectiveDate', row.effective_date),
    fieldRow('salesContract.proc.labelExpiryDate', row.expiry_date),
    fieldRow('salesContract.proc.labelStatus', getStatusLabel(row.status)),
    fieldRow('salesContract.proc.labelPaymentTerms', row.payment_terms),
    fieldRow('salesContract.proc.labelPaymentMethod', row.payment_method),
    fieldRow('salesContract.proc.labelDeliveryDate', row.delivery_date),
    fieldRow('salesContract.proc.labelDeliveryLocation', row.delivery_location),
  ]);
};

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

  /** 删除：确认框标题与按钮文案走 i18n（common.message.confirmTitle 与既有各域确认框同源） */
  const handleDelete = async (row: SalesContract) => {
    try {
      await ElMessageBox.confirm(
        tr('salesContract.proc.deleteConfirm'),
        tr('common.message.confirmTitle'),
        { type: 'warning' }
      );
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

  /**
  /**
   * 查看详情（弹出 ElMessageBox）：消息走 buildScContractDetailMessage 的 VNode 通道，
   * 字段值以文本节点渲染；不得回退为「HTML 字符串 + dangerouslyUseHTMLString」拼接口径。
   * 标题与按钮文案走 i18n（salesContract.proc.detailTitle 与 common.close）。
   */
  const handleView = (row: SalesContract) => {
    ElMessageBox.alert(buildScContractDetailMessage(row), tr('salesContract.proc.detailTitle'), {
      confirmButtonText: tr('common.close'),
    });
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
    // 打印页标题、表头与统计行全部走 i18n（salesContract.proc.*），静态文案也过 escapeHtml 统一口径
    const docTitle = escapeHtml(tr('salesContract.proc.printDocTitle'));
    printWindow.document.write(`
      <html><head><meta charset="utf-8"><title>${docTitle}</title>
      <style>
        @media print { @page { size: landscape; } }
        body { font-family: "Microsoft YaHei", sans-serif; font-size: 12px; }
        h1 { text-align: center; }
        table { width: 100%; border-collapse: collapse; margin-top: 12px; }
        th, td { border: 1px solid #333; padding: 6px 8px; }
        th { background: #f5f5f5; }
        .meta { text-align: center; color: #666; font-size: 11px; }
      </style></head><body>
      <h1>${docTitle}</h1>
      <div class="meta">${escapeHtml(tr('salesContract.proc.printDateLabel'))}: ${escapeHtml(now)} | ${escapeHtml(tr('salesContract.proc.printTotalLabel', { count: list.length }))}</div>
      <table>
        <thead><tr><th>${escapeHtml(tr('salesContract.proc.thContractNo'))}</th><th>${escapeHtml(tr('salesContract.proc.thContractName'))}</th><th>${escapeHtml(tr('salesContract.proc.thCustomer'))}</th><th>${escapeHtml(tr('salesContract.proc.thAmount'))}</th><th>${escapeHtml(tr('salesContract.proc.thSignedDate'))}</th><th>${escapeHtml(tr('salesContract.proc.thStatus'))}</th></tr></thead>
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

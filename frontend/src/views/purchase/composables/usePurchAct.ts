/**
 * usePurchAct - 采购单业务操作 composable
 * 包含：审批、查看、打印、导出
 */
import { ref } from 'vue';
import { logger } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { isDialogDismissal } from '@/utils/monitor';
import { promptApprovalReason, promptRejectReason } from '@/composables/useActionPrompts';
import printJS from 'print-js';
import {
  getPurchaseOrderById,
  approvePurchaseOrder,
  submitPurchaseOrder,
  rejectPurchaseOrder,
  updatePurchaseOrder,
  deletePurchaseOrder,
  receivePurchaseItems,
  generatePurchaseOrderNo,
  type PurchaseOrder,
  type PurchaseOrderItem,
} from '@/api/purchase';
// 导出走后端带水印 xlsx 接口（资源前缀为单数 /purchase，复数 /purchases 无挂载）
import { exportFromBackend } from '@/utils/export';

/**
 * 采购单业务操作 composable
 * 第 4 参数 getQueryParams：导出时透传列表筛选条件（status/supplier_id），导出与列表同口径
 */
export function usePurchAct(
  orders: () => PurchaseOrder[],
  getStatusText: (s: string) => string,
  onRefresh: () => void,
  getQueryParams: () => { status?: string; supplier_id?: number } = () => ({})
) {
  // 查看对话框
  const viewDialogVisible = ref(false);
  const viewData = ref<PurchaseOrder | null>(null);

  /**
   * 查看采购单详情
   */
  const handleView = async (row: PurchaseOrder) => {
    try {
      const res = await getPurchaseOrderById(row.id);
      viewData.value = res.data || row;
    } catch (error) {
      logger.error(msg.translate('loadPurchaseOrderDetailFailed'), error);
      viewData.value = row;
    }
    viewDialogVisible.value = true;
  };

  /**
   * 明细行保存成功后回源刷新对话框数据：派生金额列（subtotal/discount_amount/
   * tax_amount/total_amount）与供应商保密快照列均由后端权威口径重算/反查，前端不得本地拼算；
   * 取不到新数据时如实记日志并保留旧值，不清空。
   */
  const refreshViewData = async () => {
    const current = viewData.value;
    if (!current) return;
    try {
      const res = await getPurchaseOrderById(current.id);
      if (res.data) viewData.value = res.data;
    } catch (error) {
      logger.error(msg.translate('loadPurchaseOrderDetailFailed'), error);
    }
  };

  /**
   * 审批通过（pending_approval → approved），理由选填：先经 promptApprovalReason(false) 采集，
   * 取消即中止（非错误）。留空时省略 approval_reason 键，后端 purchase_orders.approval_reason 列写 NULL（不伪造空串）。
   */
  const handleApprove = async (row: PurchaseOrder) => {
    const approvalReason = await promptApprovalReason(false);
    if (approvalReason === null) return;
    try {
      await approvePurchaseOrder(row.id, approvalReason);
      msg.success('purchaseOrderApproved', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      if (isDialogDismissal(error)) return;
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('approveFailed'));
    }
  };

  /**
   * 打印采购订单列表
   */
  const handlePrint = () => {
    const printData = orders().map((item: PurchaseOrder, index: number) => ({
      序号: index + 1,
      订单号: item.order_no,
      供应商: item.supplier_name,
      金额: `¥${item.total_amount}`,
      状态: getStatusText(item.status),
      创建时间: item.created_at,
    }));
    printJS({
      printable: printData,
      properties: Object.keys(printData[0] || {}),
      type: 'json',
      header: '采购订单列表',
      style: 'padding: 20px; font-size: 14px;',
      headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
      gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
      gridStyle: 'border-collapse: collapse; width: 100%;',
    });
  };

  /**
   * 导出采购订单 xlsx（禁止 CSV 交付）：
   * 调用后端 GET /purchase/orders/export（后端注入水印 + 行级数据权限 + 异步审计日志），
   * 透传当前列表筛选条件（status/supplier_id），与列表同口径。
   */
  const handleExport = async () => {
    const filters = getQueryParams();
    const params: Record<string, unknown> = {
      status: filters.status || undefined,
      supplier_id: filters.supplier_id,
    };
    await exportFromBackend('/purchase/orders/export', params, 'purchase_orders_export');
  };

  /**
   * 提交采购单（draft → pending_approval 状态机）
   */
  const handleSubmitOrder = async (row: PurchaseOrder) => {
    try {
      await ElMessageBox.confirm(`确定提交采购单 ${row.order_no} 进入审批流程吗？`, '提交确认', {
        type: 'info',
      });
      await submitPurchaseOrder(row.id);
      msg.success('purchaseOrderSubmitted', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      if (!isDialogDismissal(error)) {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('operationFailed'));
      }
    }
  };

  /**
   * 驳回采购单：理由必填，先经 promptRejectReason() 采集再提交，取消即中止（非错误）。
   * 理由落 purchase_orders.rejected_reason 专列；后端对空白/超列宽返回定性 400 文案，此处原样透出。
   */
  const handleReject = async (row: PurchaseOrder) => {
    const reason = await promptRejectReason();
    if (reason === null) return;
    try {
      await rejectPurchaseOrder(row.id, reason);
      msg.success('purchaseOrderRejected', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('operationFailed'));
    }
  };

  /**
   * 编辑采购单（对齐后端 UpdatePurchaseOrderRequest）
   */
  const handleEdit = async (row: PurchaseOrder) => {
    try {
      await updatePurchaseOrder(row.id, { notes: row.notes } as Partial<PurchaseOrder>);
      msg.success('purchaseOrderUpdated', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('operationFailed'));
    }
  };

  /**
   * 删除采购单（仅草稿态，确认后执行）
   */
  const handleDeleteOrder = async (row: PurchaseOrder) => {
    try {
      await ElMessageBox.confirm(
        `删除后采购单 ${row.order_no} 不可恢复，确认删除吗？`,
        '删除确认',
        { type: 'warning' }
      );
      await deletePurchaseOrder(row.id);
      msg.success('purchaseOrderDeleted', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      if (!isDialogDismissal(error)) {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('operationFailed'));
      }
    }
  };

  /**
   * 收货登记（跳转收货对话框的前置数据加载）
   */
  const handleReceive = async (row: PurchaseOrder) => {
    try {
      const detail = await getPurchaseOrderById(row.id);
      const items =
        (detail.data as unknown as { items?: Partial<PurchaseOrderItem>[] })?.items || [];
      await receivePurchaseItems(row.id, items as never);
      msg.success('purchaseOrderReceived', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('operationFailed'));
    }
  };

  /**
   * 生成采购单号（创建表单预填用）
   */
  const handleGenerateOrderNo = async (): Promise<string> => {
    const res = await generatePurchaseOrderNo();
    return (res.data as unknown as { order_no?: string })?.order_no || '';
  };

  return {
    viewDialogVisible,
    viewData,
    handleView,
    refreshViewData,
    handleApprove,
    handlePrint,
    handleExport,
    handleSubmitOrder,
    handleReject,
    handleEdit,
    handleDeleteOrder,
    handleReceive,
    handleGenerateOrderNo,
  };
}

/**
 * usePurchAct - 采购单业务操作 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue）
 * 包含：审批、查看、打印、导出
 */
import { ref } from 'vue';
import { logger } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
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
// V15 P0-S12 修复（Batch 475b）：导出改用后端带水印 xlsx 接口
// 后端 GET /purchase/orders/export 已就绪（含行级数据权限 + 异步审计日志 + 水印）
// 前缀必须是单数 purchase：routes/mod.rs:507 nest("/api/v1/erp/purchase") +
// routes/purchase.rs:62 route("/orders/export")。此前写成 /purchases/ 恒 404，
// 导出按钮点了没有任何反应（exportFromBackend 抛错、无 blob 故无 download 事件）。
import { exportFromBackend } from '@/utils/export';

/**
 * 采购单业务操作 composable
 *
 * V15 P0-S12 修复（Batch 475b）：新增第 4 参数 getQueryParams，用于导出时传递列表筛选条件
 * 保证导出数据与当前列表筛选一致（status/supplier_id）
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
   * 审批采购单
   */
  const handleApprove = async (row: PurchaseOrder) => {
    try {
      await ElMessageBox.confirm(`确定审批通过采购单 ${row.order_no} 吗？`, '审批确认', {
        type: 'success',
      });
      await approvePurchaseOrder(row.id);
      msg.success('purchaseOrderApproved', { orderNo: row.order_no });
      onRefresh();
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('approveFailed'));
      }
    }
  };

  /**
   * 打印采购订单列表
   */
  const handlePrint = () => {
    // v11 批次 177 P2-1 修复：(item: any) 改为 (item: PurchaseOrder)
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
   * 导出采购订单列表为 xlsx（V15 P0-S12 修复 Batch 475b）
   *
   * 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
   * 改为调用后端 GET /purchase/orders/export，后端注入水印 + 行级数据权限 + 异步审计日志
   * 传入当前列表筛选条件（status/supplier_id），保证导出与列表一致
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
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error);
        ElMessage.error(errMsg || msg.translate('operationFailed'));
      }
    }
  };

  /**
   * 驳回采购单（原因必填）
   */
  const handleReject = async (row: PurchaseOrder) => {
    let reason = '';
    try {
      const { value } = await ElMessageBox.prompt('请输入驳回原因', `驳回 ${row.order_no}`, {
        type: 'warning',
        inputPattern: /\S+/,
        inputErrorMessage: '驳回原因不能为空',
      });
      reason = value;
    } catch {
      return;
    }
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
      if (error !== 'cancel') {
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

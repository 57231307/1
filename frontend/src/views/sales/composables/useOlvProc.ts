/**
 * useOlvProc.ts - 销售订单列表流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales/views/OrderListView.vue）
 * 封装销售订单审批/取消/发货/表单提交等业务流程
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  approveSalesOrder,
  cancelSalesOrder,
  updateSalesOrder,
  createSalesOrder,
  deleteSalesOrder,
  submitSalesOrder,
  rejectSalesOrder,
  getSalesDeliveryList,
  getSalesOrderStatistics,
  generateSalesOrderNo,
  shipSalesOrder,
  type SalesOrder,
  type SalesDelivery,
} from '@/api/sales';
import type { OrderForm } from './useOlv';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';

/** 刷新回调 */
interface RefreshCallbacks {
  refresh: () => Promise<void>;
}

/** 发货对话框表单中与出库四维扣减相关的字段（与 useOlv deliveryForm 结构兼容） */
interface SalesShipForm {
  order_id: number;
  items: {
    product_id: number;
    deliver_quantity: number;
    color_no: string;
    dye_lot_no: string;
    batch_no: string;
  }[];
}

/**
 * 销售订单列表流程操作方法集合
 */
export function useOlvProc(refresh: RefreshCallbacks) {
  /** 审批订单 */
  const handleApprove = async (row: SalesOrder) => {
    try {
      await ElMessageBox.confirm('确定审批此订单吗？', '确认', { type: 'info' });
      await approveSalesOrder(row.id);
      msg.success('approveSuccess');
      await refresh.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as { message?: string };
        ElMessage.error(err.message || msg.translate('operationFailed'));
      }
    }
  };

  /** 取消订单 */
  const handleCancel = async (row: SalesOrder) => {
    try {
      await ElMessageBox.confirm('确定取消此订单吗？', '确认', { type: 'warning' });
      await cancelSalesOrder(row.id);
      msg.success('cancelSuccess');
      await refresh.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as { message?: string };
        ElMessage.error(err.message || msg.translate('operationFailed'));
      }
    }
  };

  /** 提交订单表单 */
  const handleFormSubmit = async (data: OrderForm) => {
    try {
      if (data.id) {
        await updateSalesOrder(data.id, data as unknown as Partial<SalesOrder>);
        msg.success('updateSuccess');
      } else {
        await createSalesOrder(data as unknown as Partial<SalesOrder>);
        msg.success('createSuccess');
      }
      await refresh.refresh();
      return true;
    } catch (error) {
      const err = error as { message?: string };
      ElMessage.error(err.message || msg.translate('operationFailed'));
      return false;
    }
  };

  /**
   * 提交发货（DeliveryDialog 调用）：走真实出库端点 POST /sales/orders/{id}/ship，
   * 后端按"款号+色号+缸号+批次"四维匹配扣减库存（指定缸不足才显式跨缸回退）。
   * warehouse_code 由调用方从已选仓库带出（后端按编码查仓）。
   */
  const handleDeliverySubmit = async (
    form: SalesShipForm,
    warehouseCode: string
  ): Promise<boolean> => {
    try {
      await shipSalesOrder(form.order_id, {
        order_id: form.order_id,
        warehouse_code: warehouseCode,
        remarks: undefined,
        items: form.items
          .filter(i => i.deliver_quantity > 0)
          .map(i => ({
            product_id: i.product_id,
            quantity: i.deliver_quantity,
            color_no: i.color_no,
            dye_lot_no: i.dye_lot_no,
            batch_no: i.batch_no,
          })),
      });
      msg.success('shipSuccess');
      await refresh.refresh();
      return true;
    } catch (error) {
      const err = error as { message?: string };
      ElMessage.error(err.message || msg.translate('shipFailed'));
      logger.error('发货失败:', error);
      return false;
    }
  };

  /** 提交订单（后端 submit 写入 pending，approve/reject 都以 pending 为前置状态） */
  const handleSubmitOrder = async (row: SalesOrder) => {
    try {
      await ElMessageBox.confirm('确定提交此订单进入审批流程吗？', '确认', { type: 'info' });
      await submitSalesOrder(row.id);
      msg.success('submitSuccess');
      await refresh.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as { message?: string };
        ElMessage.error(err.message || msg.translate('operationFailed'));
      }
    }
  };

  /** 驳回订单（提交后退回，需填写原因） */
  const handleReject = async (row: SalesOrder) => {
    let reason = '';
    try {
      const { value } = await ElMessageBox.prompt('请输入驳回原因', '驳回订单', {
        type: 'warning',
        inputPattern: /\S+/,
        inputErrorMessage: '驳回原因不能为空',
      });
      reason = value;
    } catch {
      return;
    }
    try {
      await rejectSalesOrder(row.id, reason);
      msg.success('rejectSuccess');
      await refresh.refresh();
    } catch (error) {
      const err = error as { message?: string };
      ElMessage.error(err.message || msg.translate('operationFailed'));
    }
  };

  /** 删除订单（仅草稿态可删） */
  const handleDelete = async (row: SalesOrder) => {
    try {
      await ElMessageBox.confirm('删除后订单不可恢复，确认删除吗？', '删除', {
        type: 'warning',
      });
      await deleteSalesOrder(row.id);
      msg.success('deleteSuccess');
      await refresh.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as { message?: string };
        ElMessage.error(err.message || msg.translate('operationFailed'));
      }
    }
  };

  /** 订单详情（含发货记录回源） */
  const handleDetail = async (row: SalesOrder) => {
    try {
      const res = await getSalesDeliveryList(row.id);
      const deliveries =
        (res.data as unknown as SalesDelivery[] | { list?: SalesDelivery[] }) || [];
      const list = Array.isArray(deliveries) ? deliveries : deliveries.list || [];
      const lines = [
        `发货记录 ${list.length} 条`,
        ...list.slice(0, 5).map(d => `${d.delivery_no || d.id}（${d.status}）`),
      ];
      ElMessageBox.alert(lines.join('\n'), `订单 ${row.order_no} 详情`, { type: 'info' });
    } catch (error) {
      const err = error as { message?: string };
      ElMessage.error(err.message || msg.translate('operationFailed'));
    }
  };

  /** 销售统计汇总（弹窗展示） */
  const handleStatistics = async () => {
    try {
      const res = await getSalesOrderStatistics({});
      const d = (res.data ?? {}) as unknown as Record<string, unknown>;
      const lines = Object.entries(d).map(([k, v]) => `${k}: ${v}`);
      ElMessageBox.alert(lines.join('\n') || '暂无统计数据', '销售统计', { type: 'info' });
    } catch (error) {
      const err = error as { message?: string };
      ElMessage.error(err.message || msg.translate('operationFailed'));
    }
  };

  /** 生成订单号（创建表单预填用） */
  const handleGenerateOrderNo = async (): Promise<string> => {
    const res = await generateSalesOrderNo();
    return (res.data as unknown as { order_no?: string; no?: string })?.order_no || '';
  };

  return {
    handleApprove,
    handleCancel,
    handleFormSubmit,
    handleDeliverySubmit,
    handleSubmitOrder,
    handleReject,
    handleDelete,
    handleDetail,
    handleStatistics,
    handleGenerateOrderNo,
  };
}

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
  createSalesDelivery,
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

  /** 提交发货（DeliveryDialog 调用） */
  const handleDeliverySubmit = async (form: Partial<SalesDelivery> & { order_id: number }) => {
    try {
      await createSalesDelivery(form.order_id, form as Partial<SalesDelivery>);
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

  return {
    handleApprove,
    handleCancel,
    handleFormSubmit,
    handleDeliverySubmit,
  };
}

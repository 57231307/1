import { defineStore } from 'pinia';
import { ref } from 'vue';
import {
  getSalesOrderList,
  getSalesOrderById,
  createSalesOrder,
  updateSalesOrder,
  submitSalesOrder,
  approveSalesOrder,
  type SalesOrder,
  type SalesOrderQueryParams,
  type CreateSalesOrderPayload,
  type UpdateSalesOrderPayload,
} from '@/api/sales';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';

export const useSalesStore = defineStore('sales', () => {
  const orders = ref<SalesOrder[]>([]);
  const total = ref(0);
  const loading = ref(false);
  const currentOrder = ref<SalesOrder | null>(null);

  const fetchOrders = async (params?: SalesOrderQueryParams) => {
    loading.value = true;
    try {
      const res = await getSalesOrderList(params);
      // 兼容 PaginatedResponse { items, total }（当前后端格式）与历史 { list, total }
      const payload = res.data as {
        items?: SalesOrder[];
        list?: SalesOrder[];
        total?: number;
      } | null;
      if (payload) {
        orders.value = payload.items || payload.list || [];
        total.value = payload.total || orders.value.length;
      }
    } catch (error) {
      logger.error('获取订单列表失败:', error);
      msg.error('sales.fetchFailed');
    } finally {
      loading.value = false;
    }
  };

  const getOrderById = async (id: number) => {
    try {
      const res = await getSalesOrderById(id);
      // 仅在后端返回有效数据时更新并返回，data 为 null 时返回 null
      if (res.data) {
        currentOrder.value = res.data;
        return res.data;
      }
      return null;
    } catch (error) {
      logger.error('获取订单详情失败:', error);
      msg.error('sales.fetchDetailFailed');
      return null;
    }
  };

  // 签名对齐 api 契约载荷（唯一真相 services/so/mod.rs CreateSalesOrderRequest/
  // UpdateSalesOrderRequest）：响应实体 SalesOrder 含 order_no/customer_name/total_amount
  // 等生成列，Partial<SalesOrder> 当载荷即把出参键混入请求（原 TS2345 根因）。
  const createOrder = async (data: CreateSalesOrderPayload) => {
    try {
      const res = await createSalesOrder(data);
      await fetchOrders();
      return res;
    } catch (error) {
      logger.error('创建订单失败:', error);
      return null;
    }
  };

  const updateOrder = async (id: number, data: UpdateSalesOrderPayload) => {
    try {
      const res = await updateSalesOrder(id, data);
      await fetchOrders();
      return res;
    } catch (error) {
      logger.error('更新订单失败:', error);
      return null;
    }
  };

  const submitOrder = async (id: number) => {
    try {
      await submitSalesOrder(id);
      await fetchOrders();
      return true;
    } catch (error) {
      logger.error('提交订单失败:', error);
      return false;
    }
  };

  const approveOrder = async (id: number) => {
    try {
      await approveSalesOrder(id);
      await fetchOrders();
      return true;
    } catch (error) {
      logger.error('审批订单失败:', error);
      return false;
    }
  };

  return {
    orders,
    total,
    loading,
    currentOrder,
    fetchOrders,
    getOrderById,
    createOrder,
    updateOrder,
    submitOrder,
    approveOrder,
  };
});

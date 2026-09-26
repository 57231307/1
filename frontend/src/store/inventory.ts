import { defineStore } from 'pinia';
import { ref } from 'vue';
import {
  getStockList,
  getStockAlertList,
  STOCK_ALERT_PAGE_SIZE,
  createStockAdjustment,
  type InventoryStock,
  type StockAlert,
  type InventoryQueryParams,
} from '@/api/inventory';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';

export const useInventoryStore = defineStore('inventory', () => {
  const stocks = ref<InventoryStock[]>([]);
  const alerts = ref<StockAlert[]>([]);
  const total = ref(0);
  const loading = ref(false);

  const fetchStocks = async (params?: InventoryQueryParams) => {
    loading.value = true;
    try {
      const res = await getStockList(params);
      // 兼容 PaginatedResponse { items, total }（当前后端格式）与历史 { list, total }
      const payload = res.data as {
        items?: InventoryStock[];
        list?: InventoryStock[];
        total?: number;
      } | null;
      if (payload) {
        stocks.value = payload.items || payload.list || [];
        total.value = payload.total || stocks.value.length;
      }
    } catch (error) {
      logger.error('获取库存列表失败:', error);
      msg.error('inventory.fetchFailed');
    } finally {
      loading.value = false;
    }
  };

  const fetchAlerts = async () => {
    try {
      const res = await getStockAlertList({ page: 1, page_size: STOCK_ALERT_PAGE_SIZE });
      const payload = res.data;
      if (!payload || !Array.isArray(payload.items)) {
        // 出参是 PaginatedResponse{items,total,...}；把它当数组赋值会让预警列表静默变空
        throw new Error('库存预警出参缺少 items 数组');
      }
      alerts.value = payload.items;
    } catch (error) {
      logger.error('获取库存预警失败:', error);
      msg.error('inventory.fetchAlertsFailed');
    }
  };

  // P2-11a 修复（批次 83 v1 复审）：收紧 createAdjustment 参数类型，对齐 StockAdjustmentData 契约
  const createAdjustment = async (data: import('@/api/inventory').StockAdjustmentData) => {
    try {
      await createStockAdjustment(data);
      await fetchStocks();
      return true;
    } catch (error) {
      logger.error('创建库存调整失败:', error);
      return false;
    }
  };

  return {
    stocks,
    alerts,
    total,
    loading,
    fetchStocks,
    fetchAlerts,
    createAdjustment,
  };
});

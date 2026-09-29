import { defineStore } from 'pinia';
import { ref } from 'vue';
import {
  getDashboardOverview,
  getDashboardSalesStats,
  getDashboardInventoryStats,
  type DashboardOverview,
  type SalesStatistics,
  type InventoryStatistics,
} from '@/api/dashboard';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';

export const useDashboardStore = defineStore('dashboard', () => {
  const stats = ref<DashboardOverview>({
    total_products: 0,
    total_warehouses: 0,
    total_orders: 0,
    total_sales: '0',
    low_stock_count: 0,
    pending_orders: 0,
    monthly_sales: '0',
    recent_activities: [],
  });

  const salesStatistics = ref<SalesStatistics>({
    daily_sales: [],
    weekly_sales: [],
    monthly_sales: [],
    by_customer: [],
    by_product: [],
    by_salesperson: [],
  });
  const inventoryStatistics = ref<InventoryStatistics>({
    total_inventory: '0',
    by_warehouse: [],
    by_category: [],
    turnover_rate: '0',
    aging_analysis: [],
  });
  const loading = ref(false);

  const fetchStats = async () => {
    loading.value = true;
    try {
      const res = await getDashboardOverview();
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) stats.value = res.data;
    } catch (error) {
      logger.error('获取仪表盘概览失败:', error);
      msg.error('dashboard.fetchFailed');
    } finally {
      loading.value = false;
    }
  };

  const fetchSalesStats = async () => {
    try {
      const res = await getDashboardSalesStats();
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) salesStatistics.value = res.data;
    } catch (error) {
      logger.error('获取销售统计失败:', error);
      msg.error('dashboard.fetchSalesFailed');
    }
  };

  const fetchInventoryStats = async () => {
    try {
      const res = await getDashboardInventoryStats();
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) inventoryStatistics.value = res.data;
    } catch (error) {
      logger.error('获取库存统计失败:', error);
      msg.error('dashboard.fetchInventoryFailed');
    }
  };

  return {
    stats,
    salesStatistics,
    inventoryStatistics,
    loading,
    fetchStats,
    fetchSalesStats,
    fetchInventoryStats,
  };
});

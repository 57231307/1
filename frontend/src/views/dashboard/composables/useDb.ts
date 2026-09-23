// Dashboard 主业务 composable
// 拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
// 业务领域：仪表板（overview + 2 图表数据 + 日期范围 + 趋势天数）
// 行为完全保持一致（仅结构重构）
import { reactive, ref, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import {
  getDashboardOverview,
  getDashboardSalesStats,
  getDashboardInventoryStats,
} from '@/api/dashboard';
import type { DashboardOverview, SalesDataPoint, InventoryByCategory } from '@/api/dashboard';
import { logger } from '@/utils/logger';

/** Dashboard 主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useDb = () => {
  // 日期范围 + 趋势天数
  const dateRange = ref<[Date, Date] | null>(null);
  const trendDays = ref(7);

  // 概览数据（后端 ApiResponse.data 为 Option，取不到时回落到零值概览）
  const overviewDefaults = (): DashboardOverview => ({
    total_products: 0,
    total_warehouses: 0,
    total_orders: 0,
    total_sales: '0',
    low_stock_count: 0,
    pending_orders: 0,
    monthly_sales: '0',
    recent_activities: [],
  });
  const stats = ref<DashboardOverview>(overviewDefaults());

  // 图表数据
  const trendData = ref<SalesDataPoint[]>([]);
  const categoryDistribution = ref<InventoryByCategory[]>([]);

  // 获取概览数据
  const fetchDashboardData = async () => {
    try {
      const res = await getDashboardOverview();
      stats.value = res.data ?? overviewDefaults();
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          msg.translate('loadDashboardDataFailed')
      );
      stats.value = overviewDefaults();
    }
  };

  // 获取图表数据
  const fetchChartData = async () => {
    try {
      const [salesRes, inventoryRes] = await Promise.all([
        getDashboardSalesStats(),
        getDashboardInventoryStats(),
      ]);
      trendData.value = salesRes.data?.daily_sales ?? [];
      categoryDistribution.value = inventoryRes.data?.by_category ?? [];
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      logger.error('获取图表数据失败:', error);
      trendData.value = [];
      categoryDistribution.value = [];
    }
  };

  // 刷新最新活动（同时刷新概览 + 图表）
  const refreshActivities = async () => {
    await Promise.all([fetchDashboardData(), fetchChartData()]);
    msg.success('refreshSuccess');
  };

  // 日期变化 → 重新拉取概览
  const handleDateChange = async () => {
    await fetchDashboardData();
  };

  // 趋势天数变化 → 重新拉取销售统计
  const handleTrendDaysChange = async () => {
    try {
      const res = await getDashboardSalesStats();
      trendData.value = res.data?.daily_sales ?? [];
    } catch (error) {
      logger.error('获取销售趋势失败:', error);
      trendData.value = [];
    }
  };

  // 监听趋势天数变化
  watch(trendDays, () => {
    handleTrendDaysChange();
  });

  return reactive({
    dateRange,
    trendDays,
    stats,
    trendData,
    categoryDistribution,
    fetchDashboardData,
    fetchChartData,
    refreshActivities,
    handleDateChange,
    handleTrendDaysChange,
  });
};

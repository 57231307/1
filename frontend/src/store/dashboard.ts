import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  dashboardApi,
  type DashboardOverview,
  type SalesStatistics,
  type InventoryStatistics,
} from '@/api/dashboard'
import { logger } from '@/utils/logger'

export const useDashboardStore = defineStore('dashboard', () => {
  const stats = ref<DashboardOverview>({
    fabricCount: 0,
    inventoryTotal: 0,
    monthOrders: 0,
    customerCount: 0,
    todayOrders: 0,
    pendingOrders: 0,
    lowStockProducts: 0,
    monthSales: 0,
    recentActivities: [],
  })

  const salesStatistics = ref<SalesStatistics>({})
  const inventoryStatistics = ref<InventoryStatistics>({})
  const loading = ref(false)

  const fetchStats = async () => {
    loading.value = true
    try {
      const res = await dashboardApi.getOverview()
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) stats.value = res.data
    } catch (error) {
      logger.error('获取仪表盘概览失败:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchSalesStats = async () => {
    try {
      const res = await dashboardApi.getSalesStats()
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) salesStatistics.value = res.data
    } catch (error) {
      logger.error('获取销售统计失败:', error)
    }
  }

  const fetchInventoryStats = async () => {
    try {
      const res = await dashboardApi.getInventoryStats()
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) inventoryStatistics.value = res.data
    } catch (error) {
      logger.error('获取库存统计失败:', error)
    }
  }

  return {
    stats,
    salesStatistics,
    inventoryStatistics,
    loading,
    fetchStats,
    fetchSalesStats,
    fetchInventoryStats,
  }
})

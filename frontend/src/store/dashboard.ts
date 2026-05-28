import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  dashboardApi,
  type DashboardOverview,
  type SalesStatistics,
  type InventoryStatistics,
} from '@/api/dashboard'

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
      stats.value = res.data!
    } catch (error) {
      console.error('Failed to fetch dashboard stats:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchSalesStats = async () => {
    try {
      const res = await dashboardApi.getSalesStats()
      salesStatistics.value = res.data!
    } catch (error) {
      console.error('Failed to fetch sales stats:', error)
    }
  }

  const fetchInventoryStats = async () => {
    try {
      const res = await dashboardApi.getInventoryStats()
      inventoryStatistics.value = res.data!
    } catch (error) {
      console.error('Failed to fetch inventory stats:', error)
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

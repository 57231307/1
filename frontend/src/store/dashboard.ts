import { defineStore } from 'pinia'
import { ref } from 'vue'
import { dashboardApi, type DashboardStats, type SalesTrend, type ChartData } from '@/api/dashboard'

export const useDashboardStore = defineStore('dashboard', () => {
  const stats = ref<DashboardStats>({
    fabricCount: 0,
    inventoryTotal: 0,
    monthOrders: 0,
    customerCount: 0,
    todayOrders: 0,
    pendingOrders: 0,
    lowStockProducts: 0,
    recentActivities: [],
  })

  const salesTrend = ref<SalesTrend[]>([])
  const inventoryDistribution = ref<ChartData[]>([])
  const loading = ref(false)

  const fetchStats = async () => {
    loading.value = true
    try {
      const res = await dashboardApi.getStats()
      stats.value = res.data
    } catch (error) {
      console.error('Failed to fetch dashboard stats:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchSalesTrend = async (days = 7) => {
    try {
      const res = await dashboardApi.getSalesTrend(days)
      salesTrend.value = res.data
    } catch (error) {
      console.error('Failed to fetch sales trend:', error)
    }
  }

  const fetchInventoryDistribution = async () => {
    try {
      const res = await dashboardApi.getInventoryDistribution()
      inventoryDistribution.value = res.data
    } catch (error) {
      console.error('Failed to fetch inventory distribution:', error)
    }
  }

  return {
    stats,
    salesTrend,
    inventoryDistribution,
    loading,
    fetchStats,
    fetchSalesTrend,
    fetchInventoryDistribution,
  }
})

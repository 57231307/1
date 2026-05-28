import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  inventoryApi,
  type InventoryStock,
  type StockAlert,
  type InventoryQueryParams,
} from '@/api/inventory'

export const useInventoryStore = defineStore('inventory', () => {
  const stocks = ref<InventoryStock[]>([])
  const alerts = ref<StockAlert[]>([])
  const total = ref(0)
  const loading = ref(false)

  const fetchStocks = async (params?: InventoryQueryParams) => {
    loading.value = true
    try {
      const res = await inventoryApi.getStockList(params)
      stocks.value = res.data!.list
      total.value = res.data!.total
    } catch (error) {
      console.error('Failed to fetch stocks:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchAlerts = async () => {
    try {
      const res = await inventoryApi.getStockAlerts()
      alerts.value = res.data!
    } catch (error) {
      console.error('Failed to fetch alerts:', error)
    }
  }

  const createAdjustment = async (data: any) => {
    try {
      await inventoryApi.createStockAdjustment(data)
      await fetchStocks()
      return true
    } catch (error) {
      console.error('Failed to create adjustment:', error)
      return false
    }
  }

  return {
    stocks,
    alerts,
    total,
    loading,
    fetchStocks,
    fetchAlerts,
    createAdjustment,
  }
})

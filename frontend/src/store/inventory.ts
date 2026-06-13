import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  inventoryApi,
  type InventoryStock,
  type StockAlert,
  type InventoryQueryParams,
} from '@/api/inventory'
import { logger } from '@/utils/logger'

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
      logger.error('获取库存列表失败:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchAlerts = async () => {
    try {
      const res = await inventoryApi.getStockAlerts()
      alerts.value = res.data!
    } catch (error) {
      logger.error('获取库存预警失败:', error)
    }
  }

  const createAdjustment = async (data: any) => {
    try {
      await inventoryApi.createStockAdjustment(data)
      await fetchStocks()
      return true
    } catch (error) {
      logger.error('创建库存调整失败:', error)
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

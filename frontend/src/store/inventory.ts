import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  getStockList,
  getStockAlertList,
  createStockAdjustment,
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
      const res = await getStockList(params)
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) {
        stocks.value = res.data.list
        total.value = res.data.total
      }
    } catch (error) {
      logger.error('获取库存列表失败:', error)
    } finally {
      loading.value = false
    }
  }

  const fetchAlerts = async () => {
    try {
      const res = await getStockAlertList()
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) alerts.value = res.data
    } catch (error) {
      logger.error('获取库存预警失败:', error)
    }
  }

  // P2-11a 修复（批次 83 v1 复审）：收紧 createAdjustment 参数类型，对齐 StockAdjustmentData 契约
  const createAdjustment = async (data: import('@/api/inventory').StockAdjustmentData) => {
    try {
      await createStockAdjustment(data)
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

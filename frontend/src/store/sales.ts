import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  getSalesOrderList,
  getSalesOrderById,
  createSalesOrder,
  updateSalesOrder,
  submitSalesOrder,
  approveSalesOrder,
  type SalesOrder,
  type SalesOrderQueryParams,
} from '@/api/sales'
import { logger } from '@/utils/logger'

export const useSalesStore = defineStore('sales', () => {
  const orders = ref<SalesOrder[]>([])
  const total = ref(0)
  const loading = ref(false)
  const currentOrder = ref<SalesOrder | null>(null)

  const fetchOrders = async (params?: SalesOrderQueryParams) => {
    loading.value = true
    try {
      const res = await getSalesOrderList(params)
      // 仅在后端返回有效数据时更新，防止 data 为 null 时崩溃
      if (res.data) {
        orders.value = res.data.list
        total.value = res.data.total
      }
    } catch (error) {
      logger.error('获取订单列表失败:', error)
    } finally {
      loading.value = false
    }
  }

  const getOrderById = async (id: number) => {
    try {
      const res = await getSalesOrderById(id)
      // 仅在后端返回有效数据时更新并返回，data 为 null 时返回 null
      if (res.data) {
        currentOrder.value = res.data
        return res.data
      }
      return null
    } catch (error) {
      logger.error('获取订单详情失败:', error)
      return null
    }
  }

  const createOrder = async (data: Partial<SalesOrder>) => {
    try {
      const res = await createSalesOrder(data)
      await fetchOrders()
      return res
    } catch (error) {
      logger.error('创建订单失败:', error)
      return null
    }
  }

  const updateOrder = async (id: number, data: Partial<SalesOrder>) => {
    try {
      const res = await updateSalesOrder(id, data)
      await fetchOrders()
      return res
    } catch (error) {
      logger.error('更新订单失败:', error)
      return null
    }
  }

  const submitOrder = async (id: number) => {
    try {
      await submitSalesOrder(id)
      await fetchOrders()
      return true
    } catch (error) {
      logger.error('提交订单失败:', error)
      return false
    }
  }

  const approveOrder = async (id: number) => {
    try {
      await approveSalesOrder(id)
      await fetchOrders()
      return true
    } catch (error) {
      logger.error('审批订单失败:', error)
      return false
    }
  }

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
  }
})

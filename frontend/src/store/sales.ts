import { defineStore } from 'pinia'
import { ref } from 'vue'
import { salesApi, type SalesOrder, type SalesOrderQueryParams } from '@/api/sales'
import { logger } from '@/utils/logger'

export const useSalesStore = defineStore('sales', () => {
  const orders = ref<SalesOrder[]>([])
  const total = ref(0)
  const loading = ref(false)
  const currentOrder = ref<SalesOrder | null>(null)

  const fetchOrders = async (params?: SalesOrderQueryParams) => {
    loading.value = true
    try {
      const res = await salesApi.getOrderList(params)
      orders.value = res.data!.list
      total.value = res.data!.total
    } catch (error) {
      logger.error('获取订单列表失败:', error)
    } finally {
      loading.value = false
    }
  }

  const getOrderById = async (id: number) => {
    try {
      const res = await salesApi.getOrderById(id)
      currentOrder.value = res.data!
      return res.data!
    } catch (error) {
      logger.error('获取订单详情失败:', error)
      return null
    }
  }

  const createOrder = async (data: Partial<SalesOrder>) => {
    try {
      const res = await salesApi.createOrder(data)
      await fetchOrders()
      return res
    } catch (error) {
      logger.error('创建订单失败:', error)
      return null
    }
  }

  const updateOrder = async (id: number, data: Partial<SalesOrder>) => {
    try {
      const res = await salesApi.updateOrder(id, data)
      await fetchOrders()
      return res
    } catch (error) {
      logger.error('更新订单失败:', error)
      return null
    }
  }

  const submitOrder = async (id: number) => {
    try {
      await salesApi.submitOrder(id)
      await fetchOrders()
      return true
    } catch (error) {
      logger.error('提交订单失败:', error)
      return false
    }
  }

  const approveOrder = async (id: number) => {
    try {
      await salesApi.approveOrder(id)
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

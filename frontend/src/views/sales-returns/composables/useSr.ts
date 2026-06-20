/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useSr.ts - 销售退货核心 composable
 * 任务编号: P14 批 2 I-3 第 7 批（拆分原 sales-returns/index.vue）
 * 提供退货单列表查询、表单管理、销售订单/客户/产品加载、CRUD 等核心方法
 * 审批流程由 useSrProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { FormInstance } from 'element-plus'
import { salesReturnApi } from '@/api/sales-return'
import { salesApi } from '@/api/sales'
import { listCustomers } from '@/api/customer'
import { productApi } from '@/api/product'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

/**
 * 销售退货 composable
 * 集中管理退货列表、表单、销售订单/客户/产品、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSr() {
  // 列表 loading
  const loading = ref(false)

  // 列表数据
  const returnList = ref<any[]>([])

  // 详情弹窗当前记录
  const currentReturn = ref<any>({ items: [] })

  // 销售订单/客户/产品下拉数据
  const salesOrderList = ref<any[]>([])
  const customerList = ref<any[]>([])
  const productList = ref<any[]>([])

  // 表单数据
  const formData = reactive<any>({
    id: null,
    salesOrderId: null,
    salesOrderNo: '',
    customerId: null,
    customerName: '',
    returnDate: '',
    reason: '',
    remarks: '',
    items: [] as any[],
    totalAmount: 0,
    status: 'PENDING',
  })

  // 表单引用
  const formRef = ref<FormInstance>()

  // 加载退货列表
  const loadReturns = async () => {
    loading.value = true
    try {
      const res = await salesReturnApi.list()
      returnList.value = res.data?.list || []
    } catch (error: any) {
      ElMessage.error(error.message || '加载退货列表失败')
    } finally {
      loading.value = false
    }
  }

  // 加载销售订单下拉
  const loadSalesOrders = async () => {
    try {
      const res = await salesApi.getOrderList({ status: 'completed' })
      salesOrderList.value = res.data?.list || []
    } catch (error: any) {
      // eslint-disable-next-line no-console
      console.error('加载销售订单失败:', error)
    }
  }

  // 加载客户下拉
  const loadCustomers = async () => {
    try {
      const res = await listCustomers()
      customerList.value = res.data?.list || []
    } catch (error: any) {
      // eslint-disable-next-line no-console
      console.error('加载客户列表失败:', error)
    }
  }

  // 加载产品下拉
  const loadProducts = async () => {
    try {
      const res = await productApi.list()
      productList.value = res.data?.list || []
    } catch (error: any) {
      // eslint-disable-next-line no-console
      console.error('加载产品列表失败:', error)
    }
  }

  // 重置表单为新建态
  const resetFormForCreate = () => {
    Object.assign(formData, {
      id: null,
      salesOrderId: null,
      salesOrderNo: '',
      customerId: null,
      customerName: '',
      returnDate: new Date().toISOString().split('T')[0],
      reason: '',
      remarks: '',
      items: [{ id: null, productId: null, quantity: 1, unitPrice: 0, reason: '' }],
      totalAmount: 0,
      status: 'PENDING',
    })
  }

  // 用行数据填充表单为编辑态
  const fillFormForEdit = (row: any) => {
    Object.assign(formData, {
      id: row.id,
      salesOrderId: row.salesOrderId,
      salesOrderNo: row.salesOrderNo,
      customerId: row.customerId,
      customerName: row.customerName,
      returnDate: row.returnDate,
      reason: row.reason,
      items: row.items ? [...row.items] : [],
      totalAmount: row.totalAmount,
      status: row.status,
    })
  }

  // 销售订单变更时联动客户与明细
  const onSalesOrderChange = (orderId: number) => {
    const order = salesOrderList.value.find(o => o.id === orderId)
    if (order) {
      formData.salesOrderNo = order.order_no
      formData.customerId = order.customer_id
      formData.customerName = order.customer_name
      if (order.items) {
        formData.items = order.items.map((item: any) => ({
          productId: item.product_id,
          productName: item.product_name,
          productCode: item.product_code,
          quantity: 0,
          unitPrice: item.unit_price,
          amount: 0,
          reason: '',
        }))
      }
      calculateTotal()
    }
  }

  // 添加空明细行
  const addItem = () => {
    formData.items.push({
      id: null,
      productId: null,
      productName: '',
      productCode: '',
      quantity: 1,
      unitPrice: 0,
      amount: 0,
      reason: '',
    })
  }

  // 删除明细行
  const removeItem = (index: number) => {
    formData.items.splice(index, 1)
    calculateTotal()
  }

  // 重算总金额
  const calculateTotal = () => {
    formData.totalAmount = formData.items.reduce((sum: number, item: any) => {
      return sum + item.quantity * item.unitPrice
    }, 0)
  }

  // 表单校验 + 提交
  const submitForm = async (dialogMode: 'create' | 'edit') => {
    if (!formRef.value) return false

    let valid = false
    await formRef.value.validate(v => {
      valid = v
    })
    if (!valid) return false

    if (formData.items.length === 0) {
      ElMessage.warning('请至少添加一条退货明细')
      return false
    }

    const submitData = {
      ...formData,
      items: formData.items.map((item: any) => ({
        productId: item.productId,
        quantity: item.quantity,
        unitPrice: item.unitPrice,
        amount: item.quantity * item.unitPrice,
        reason: item.reason,
      })),
    }

    try {
      if (dialogMode === 'create') {
        await salesReturnApi.create(submitData)
        ElMessage.success('创建成功')
      } else {
        await salesReturnApi.update(formData.id, submitData)
        ElMessage.success('更新成功')
      }
      return true
    } catch (error: any) {
      ElMessage.error(error.message || (dialogMode === 'create' ? '创建失败' : '更新失败'))
      return false
    }
  }

  // 表单弹窗关闭
  const onEditDialogClose = () => {
    formRef.value?.resetFields()
  }

  // 一次性初始化加载
  const initLoad = async () => {
    await Promise.all([loadReturns(), loadSalesOrders(), loadCustomers(), loadProducts()])
  }

  return {
    // 状态
    loading,
    returnList,
    currentReturn,
    salesOrderList,
    customerList,
    productList,
    formData,
    formRef,
    // 方法
    loadReturns,
    loadSalesOrders,
    loadCustomers,
    loadProducts,
    resetFormForCreate,
    fillFormForEdit,
    onSalesOrderChange,
    addItem,
    removeItem,
    calculateTotal,
    submitForm,
    onEditDialogClose,
    initLoad,
  }
}

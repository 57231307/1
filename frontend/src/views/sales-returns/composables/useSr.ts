/**
 * useSr.ts - 销售退货核心 composable
 * 任务编号: P14 批 2 I-3 第 7 批（拆分原 sales-returns/index.vue）
 * 提供退货单列表查询、表单管理、销售订单/客户/产品加载、CRUD 等核心方法
 * 审批流程由 useSrProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import type { FormInstance } from 'element-plus'
import { ElMessage } from 'element-plus'
import { salesReturnApi } from '@/api/sales-return'
import type { SalesReturn } from '@/api/sales-return'
import { salesApi } from '@/api/sales'
import { listCustomers } from '@/api/customer'
import { productApi } from '@/api/product'
import logger from '@/utils/logger'

// v11 批次 163 P2-1 修复：定义具体类型替代 any
interface SalesOrderOption {
  id: number
  order_no: string
  customer_id: number
  customer_name: string
  items?: Array<{
    product_id: number
    product_name: string
    product_code: string
    unit_price: number
  }>
}

interface CustomerOption {
  id: number
  name: string
  [key: string]: unknown
}

interface ProductOption {
  id: number
  name: string
  [key: string]: unknown
}

interface ReturnFormItem {
  id: number | null
  productId: number | null
  productName: string
  productCode: string
  quantity: number
  unitPrice: number
  amount: number
  reason: string
}

interface ReturnForm {
  id: number | null
  salesOrderId: number | null
  salesOrderNo: string
  customerId: number | null
  customerName: string
  returnDate: string
  reason: string
  remarks: string
  items: ReturnFormItem[]
  totalAmount: number
  status: string
}

/**
 * 销售退货 composable
 * 集中管理退货列表、表单、销售订单/客户/产品、对话框的业务状态
 */
export function useSr() {
  // 列表 loading
  const loading = ref(false)

  // v11 批次 163 P2-1 修复：any[] 改为具体类型 SalesReturn[]
  const returnList = ref<SalesReturn[]>([])

  // 详情弹窗当前记录
  const currentReturn = ref<SalesReturn | null>(null)

  // 销售订单/客户/产品下拉数据
  const salesOrderList = ref<SalesOrderOption[]>([])
  const customerList = ref<CustomerOption[]>([])
  const productList = ref<ProductOption[]>([])

  // 表单数据
  const formData = reactive<ReturnForm>({
    id: null,
    salesOrderId: null,
    salesOrderNo: '',
    customerId: null,
    customerName: '',
    returnDate: '',
    reason: '',
    remarks: '',
    items: [],
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
    } catch (error: unknown) {
      // v11 批次 163 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '加载退货列表失败')
    } finally {
      loading.value = false
    }
  }

  // 加载销售订单下拉
  const loadSalesOrders = async () => {
    try {
      const res = await salesApi.getOrderList({ status: 'completed' })
      salesOrderList.value = (res.data?.list || []) as unknown as SalesOrderOption[]
    } catch (error: unknown) {
      logger.error('加载销售订单失败', error instanceof Error ? error.message : String(error))
    }
  }

  // 加载客户下拉
  const loadCustomers = async () => {
    try {
      const res = await listCustomers()
      customerList.value = (res.data?.list || []) as unknown as CustomerOption[]
    } catch (error: unknown) {
      logger.error('加载客户列表失败', error instanceof Error ? error.message : String(error))
    }
  }

  // 加载产品下拉
  const loadProducts = async () => {
    try {
      const res = await productApi.list()
      productList.value = (res.data?.list || []) as unknown as ProductOption[]
    } catch (error: unknown) {
      logger.error('加载产品列表失败', error instanceof Error ? error.message : String(error))
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
      items: [{ id: null, productId: null, productName: '', productCode: '', quantity: 1, unitPrice: 0, amount: 0, reason: '' }],
      totalAmount: 0,
      status: 'PENDING',
    })
  }

  // 用行数据填充表单为编辑态
  const fillFormForEdit = (row: SalesReturn) => {
    Object.assign(formData, {
      id: row.id ?? null,
      salesOrderId: row.salesOrderId ?? null,
      salesOrderNo: row.salesOrderNo ?? '',
      customerId: row.customerId ?? null,
      customerName: row.customerName ?? '',
      returnDate: row.returnDate ?? '',
      reason: row.reason ?? '',
      items: row.items ? [...row.items] : [],
      totalAmount: row.totalAmount ?? 0,
      status: row.status ?? 'PENDING',
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
        formData.items = order.items.map(item => ({
          id: null,
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
    formData.totalAmount = formData.items.reduce((sum, item) => {
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

    // v11 批次 163 CI1 修复：submitData 使用 as unknown as Partial<SalesReturn> 类型转换
    const submitData = {
      ...formData,
      items: formData.items.map(item => ({
        productId: item.productId,
        quantity: item.quantity,
        unitPrice: item.unitPrice,
        amount: item.quantity * item.unitPrice,
        reason: item.reason,
      })),
    } as unknown as Partial<SalesReturn>

    try {
      if (dialogMode === 'create') {
        await salesReturnApi.create(submitData)
        ElMessage.success('创建成功')
      } else {
        await salesReturnApi.update(formData.id as number, submitData)
        ElMessage.success('更新成功')
      }
      return true
    } catch (error: unknown) {
      // v11 批次 163 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          (dialogMode === 'create' ? '创建失败' : '更新失败')
      )
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

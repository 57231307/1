/**
 * useCreate - 采购单创建表单 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 新建采购单对话框）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { purchaseApi } from '@/api/purchase'
import type { Product } from '@/api/product'

/**
 * 采购明细行数据结构
 */
export interface CreateItem {
  product_id: number | undefined
  quantity: number
  unit_price: number
  subtotal: number
}

/**
 * 新建采购单表单数据结构
 */
export interface CreateFormData {
  supplier_id: number | undefined
  order_date: string
  required_date: string
  remark: string
  items: CreateItem[]
}

/**
 * 新建采购单表单初始默认值
 */
const defaultForm = (): CreateFormData => ({
  supplier_id: undefined,
  order_date: new Date().toISOString().split('T')[0],
  required_date: '',
  remark: '',
  items: [{ product_id: undefined, quantity: 1, unit_price: 0, subtotal: 0 }],
})

/**
 * 采购单创建表单 composable
 */
export function useCreate(
  products: () => Product[],
  onSuccess: () => void
) {
  const createDialogVisible = ref(false)
  const createFormRef = ref()
  const createFormRules = {
    supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
    order_date: [{ required: true, message: '请选择订单日期', trigger: 'change' }],
  }
  const createForm = ref<CreateFormData>(defaultForm())

  /**
   * 打开创建采购单对话框
   */
  const handleCreate = () => {
    createForm.value = defaultForm()
    createDialogVisible.value = true
  }

  /**
   * 添加采购明细行
   */
  const addItem = () => {
    createForm.value.items.push({ product_id: undefined, quantity: 1, unit_price: 0, subtotal: 0 })
  }

  /**
   * 移除采购明细行
   */
  const removeItem = (index: number) => {
    if (createForm.value.items.length > 1) {
      createForm.value.items.splice(index, 1)
    }
  }

  /**
   * 选择产品时自动填入单价
   */
  const handleProductSelect = (index: number) => {
    const product = products().find(p => p.id === createForm.value.items[index].product_id)
    if (product) {
      createForm.value.items[index].unit_price = product.price || 0
      calculateSubtotal(createForm.value.items[index])
    }
  }

  /**
   * 重算小计金额
   */
  const calculateSubtotal = (item: CreateItem) => {
    item.subtotal = (item.quantity || 0) * (item.unit_price || 0)
  }

  /**
   * 计算总金额
   */
  const calculateTotal = () => {
    return createForm.value.items.reduce((sum: number, item: CreateItem) => sum + (item.subtotal || 0), 0)
  }

  /**
   * 提交新建采购单
   */
  const submitCreate = async () => {
    try {
      await createFormRef.value?.validate()
    } catch {
      return
    }
    const validItems = createForm.value.items.filter(item => item.product_id && item.quantity > 0)
    if (validItems.length === 0) {
      ElMessage.warning('请至少添加一条有效的采购明细')
      return
    }
    try {
      await purchaseApi.createOrder({
        ...createForm.value,
        items: validItems.map(item => ({
          id: 0,
          product_id: item.product_id!,
          product_name: '',
          product_code: '',
          quantity: item.quantity,
          unit_price: item.unit_price,
          subtotal: item.subtotal,
        })),
        total_amount: calculateTotal(),
      })
      ElMessage.success('采购单创建成功')
      createDialogVisible.value = false
      onSuccess()
    } catch (error: any) {
      ElMessage.error(error.message || '创建失败')
    }
  }

  return {
    createDialogVisible,
    createFormRef,
    createFormRules,
    createForm,
    handleCreate,
    addItem,
    removeItem,
    handleProductSelect,
    calculateSubtotal,
    calculateTotal,
    submitCreate,
  }
}

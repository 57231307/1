/**
 * usePurchList - 采购单列表与查询 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { getPurchaseOrderList, type PurchaseOrder } from '@/api/purchase'
import type { Supplier } from '@/api/supplier'
import type { Product } from '@/api/product'
import type { Warehouse } from '@/api/warehouse'
import { getSupplierList } from '@/api/supplier'
import { getProductList } from '@/api/product'
import { getWarehouseList } from '@/api/warehouse'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

/**
 * 订单状态/付款状态对应的 el-tag 类型与文本
 */
// el-tag 组件支持的 type 联合类型（element-plus 规范）
type TagType = '' | 'success' | 'warning' | 'info' | 'danger'

const statusTypeMap: Record<string, TagType> = {
  pending: 'warning',
  // 原值为 'primary'，不在 TagType 联合范围内，改为 ''（默认主题色，等价于 primary）
  approved: '',
  partial: 'info',
  completed: 'success',
  cancelled: 'danger',
}

const statusTextMap: Record<string, string> = {
  pending: '待审批',
  approved: '已审批',
  partial: '部分收货',
  completed: '已完成',
  cancelled: '已取消',
}

const paymentTypeMap: Record<string, TagType> = { unpaid: 'danger', partial: 'warning', paid: 'success' }
const paymentTextMap: Record<string, string> = {
  unpaid: '未付款',
  partial: '部分付款',
  paid: '已付款',
}

/**
 * 采购单列表与查询 composable
 */
export function usePurchList() {
  const hasLoaded = createLazyLoader()

  const loading = ref(false)
  const orders = ref<PurchaseOrder[]>([])
  const suppliers = ref<Supplier[]>([])
  const products = ref<Product[]>([])
  const warehouses = ref<Warehouse[]>([])
  const total = ref(0)

  const stats = ref({
    monthOrders: 0,
    monthAmount: 0,
    pendingReceipt: 0,
    supplierCount: 0,
  })

  const queryParams = reactive({
    page: 1,
    page_size: 20,
    keyword: '',
    supplier_id: undefined as number | undefined,
    status: '',
  })

  /**
   * 格式化货币为人民币字符串
   */
  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('zh-CN', {
      style: 'currency',
      currency: 'CNY',
      minimumFractionDigits: 0,
    }).format(amount)
  }

  /**
   * 订单状态对应的 el-tag 类型
   * 使用 ?? 而非 ||，避免空字符串（合法 TagType，等价于默认主题色）被错误 fallback 到 'info'
   */
  const getStatusType = (status: string): TagType => statusTypeMap[status] ?? 'info'

  /**
   * 订单状态显示文本
   */
  const getStatusText = (status: string) => statusTextMap[status] || status

  /**
   * 付款状态对应的 el-tag 类型
   */
  const getPaymentStatusType = (status: string): TagType => paymentTypeMap[status] ?? 'info'

  /**
   * 付款状态显示文本
   */
  const getPaymentStatusText = (status: string) => paymentTextMap[status] || status

  /**
   * 获取采购单列表
   */
  const fetchData = async () => {
    loading.value = true
    try {
      const res = await getPurchaseOrderList(queryParams)
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) orders.value = res.data.list || []
      total.value = res.data?.total || 0

      // 计算统计数据
      stats.value.monthOrders = total.value
      stats.value.monthAmount = orders.value.reduce((sum, o) => sum + (o.total_amount || 0), 0)
      stats.value.pendingReceipt = orders.value.filter(o => o.status === 'approved').length
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取采购单列表失败')
      orders.value = []
      total.value = 0
    } finally {
      loading.value = false
    }
  }

  /**
   * 获取供应商列表
   */
  const fetchSuppliers = async () => {
    try {
      const res = await getSupplierList({ page_size: 1000 })
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) suppliers.value = res.data.list || []
      stats.value.supplierCount = suppliers.value.length
    } catch (error) {
      logger.error('获取供应商列表失败:', error)
    }
  }

  /**
   * 获取产品列表
   */
  const fetchProducts = async () => {
    try {
      const res = await getProductList({ page_size: 1000 })
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) products.value = res.data.list || []
    } catch (error) {
      logger.error('获取产品列表失败:', error)
    }
  }

  /**
   * 获取仓库列表
   */
  const fetchWarehouses = async () => {
    try {
      const res = await getWarehouseList({ page_size: 1000 })
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) warehouses.value = res.data.list || []
    } catch (error) {
      logger.error('获取仓库列表失败:', error)
    }
  }

  const handleQuery = () => {
    queryParams.page = 1
    fetchData()
  }

  const handleReset = () => {
    queryParams.keyword = ''
    queryParams.supplier_id = undefined
    queryParams.status = ''
    handleQuery()
  }

  /**
   * 初始化页面（按需懒加载数据）
   */
  const initPage = () => {
    loadIfNot('fetchData', fetchData, hasLoaded)
    loadIfNot('fetchSuppliers', fetchSuppliers, hasLoaded)
    loadIfNot('fetchProducts', fetchProducts, hasLoaded)
    loadIfNot('fetchWarehouses', fetchWarehouses, hasLoaded)
  }

  return {
    loading,
    orders,
    suppliers,
    products,
    warehouses,
    total,
    stats,
    queryParams,
    formatCurrency,
    getStatusType,
    getStatusText,
    getPaymentStatusType,
    getPaymentStatusText,
    fetchData,
    handleQuery,
    handleReset,
    initPage,
  }
}

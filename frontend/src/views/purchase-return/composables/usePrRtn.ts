/**
 * usePrRtn.ts - 采购退货核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货列表查询、表单管理、供应商/采购单/产品加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/拒绝/删除）由 usePrRtnProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  purchaseReturnApi,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

/**
 * 采购退货 composable
 * 集中管理列表、表单、供应商、采购单、产品、详情对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePrRtn() {
  // 统计数据
  const stats = reactive({
    total: 0,
    pending: 0,
    approved: 0,
    amount: 0,
  })

  // 表格数据
  const tableData = ref<PurchaseReturn[]>([])
  const loading = ref(false)
  const total = ref(0)
  const dateRange = ref<[Date, Date] | null>(null)

  // 查询参数
  const queryParams = reactive({
    page: 1,
    pageSize: 20,
    keyword: '',
    supplierId: undefined as number | undefined,
    status: '',
  })

  // 供应商列表
  const suppliers = ref<{ id: number; name: string }[]>([])

  // 采购订单列表
  const purchaseOrders = ref<{ id: number; order_no: string }[]>([])

  // 产品列表
  const products = ref<{ id: number; name: string; price: number }[]>([])

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    purchaseOrderId: undefined as number | undefined,
    returnDate: '',
    reason: '',
    remarks: '',
    items: [] as Partial<PurchaseReturnItem>[],
  })

  // 表单校验规则
  const formRules = {
    purchaseOrderId: [{ required: true, message: '请选择采购订单', trigger: 'change' }],
    returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
    reason: [{ required: true, message: '请输入退货原因', trigger: 'blur' }],
  }

  // 详情数据
  const detailData = ref<PurchaseReturn>({} as PurchaseReturn)

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  /** 获取列表数据 */
  const fetchData = async () => {
    loading.value = true
    try {
      const params: any = { ...queryParams }
      if (dateRange.value) {
        params.startDate = dateRange.value[0].toISOString()
        params.endDate = dateRange.value[1].toISOString()
      }
      const res = await purchaseReturnApi.list(params)
      tableData.value = res.data?.list || []
      total.value = res.data?.total || 0

      // 更新统计
      stats.total = total.value
      stats.pending = tableData.value.filter(i => i.status === 'pending').length
      stats.approved = tableData.value.filter(i => i.status === 'approved').length
      stats.amount = tableData.value.reduce((sum, i) => sum + (i.totalAmount || 0), 0)
    } catch (error) {
      logger.error('获取数据失败:', error)
    } finally {
      loading.value = false
    }
  }

  /** 加载供应商列表（模拟） */
  const fetchSuppliers = async () => {
    suppliers.value = [
      { id: 1, name: '供应商A' },
      { id: 2, name: '供应商B' },
    ]
  }

  /** 加载采购订单列表（模拟） */
  const fetchPurchaseOrders = async () => {
    purchaseOrders.value = [
      { id: 1, order_no: 'CG20260101001' },
      { id: 2, order_no: 'CG20260101002' },
    ]
  }

  /** 加载产品列表（模拟） */
  const fetchProducts = async () => {
    products.value = [
      { id: 1, name: '产品A', price: 100 },
      { id: 2, name: '产品B', price: 200 },
    ]
  }

  /** 查询 */
  const handleQuery = () => {
    queryParams.page = 1
    fetchData()
  }

  /** 重置 */
  const handleReset = () => {
    queryParams.keyword = ''
    queryParams.supplierId = undefined
    queryParams.status = ''
    dateRange.value = null
    queryParams.page = 1
    fetchData()
  }

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    Object.assign(formData, {
      id: undefined,
      purchaseOrderId: undefined,
      returnDate: '',
      reason: '',
      remarks: '',
      items: [],
    })
  }

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: PurchaseReturn) => {
    Object.assign(formData, {
      id: row.id,
      purchaseOrderId: row.purchaseOrderId,
      returnDate: row.returnDate,
      reason: row.reason,
      remarks: row.remarks,
      items: row.items || [],
    })
  }

  /** 获取详情 */
  const fetchDetail = async (id: number) => {
    try {
      const res = await purchaseReturnApi.getById(id)
      detailData.value = res.data
    } catch (error) {
      logger.error('获取详情失败:', error)
    }
  }

  /** 采购订单变化（模拟加载明细） */
  const handleOrderChange = (orderId: number) => {
    const order = purchaseOrders.value.find(o => o.id === orderId)
    if (order) {
      formData.items = [
        { productId: 1, productName: '产品A', quantity: 10, unitPrice: 100, reason: '' },
      ]
    }
  }

  /** 添加明细 */
  const handleAddItem = () => {
    formData.items.push({
      productId: undefined,
      productName: '',
      quantity: 1,
      unitPrice: 0,
      reason: '',
    })
  }

  /** 删除明细 */
  const handleRemoveItem = (index: number) => {
    formData.items.splice(index, 1)
  }

  /** 产品变化（联动单价/名称） */
  const handleProductChange = (row: any, productId: number) => {
    const product = products.value.find(p => p.id === productId)
    if (product) {
      row.productName = product.name
      row.unitPrice = product.price
    }
  }

  /** 提交表单（新建/编辑） */
  const handleFormSubmit = async (isEdit: boolean): Promise<boolean> => {
    try {
      if (isEdit && formData.id) {
        await purchaseReturnApi.update(formData.id, formData as any)
        ElMessage.success('更新成功')
      } else {
        await purchaseReturnApi.create(formData as any)
        ElMessage.success('创建成功')
      }
      fetchData()
      return true
    } catch (error) {
      logger.error('提交失败:', error)
      return false
    }
  }

  /** 分页 - 每页大小 */
  const handleSizeChange = () => {
    fetchData()
  }

  /** 分页 - 当前页 */
  const handleCurrentChange = () => {
    fetchData()
  }

  /** 初始化加载 */
  const initLoad = () => {
    fetchData()
    loadIfNot('suppliers', fetchSuppliers, hasLoaded)
    loadIfNot('purchaseOrders', fetchPurchaseOrders, hasLoaded)
    loadIfNot('products', fetchProducts, hasLoaded)
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 统计
    stats,
    // 列表
    tableData,
    loading,
    total,
    dateRange,
    queryParams,
    handleQuery,
    handleReset,
    handleSizeChange,
    handleCurrentChange,
    fetchData,
    // 供应商/采购单/产品
    suppliers,
    purchaseOrders,
    products,
    // 表单
    formData,
    formRules,
    prepareCreate,
    prepareEdit,
    handleOrderChange,
    handleAddItem,
    handleRemoveItem,
    handleProductChange,
    handleFormSubmit,
    // 详情
    detailData,
    fetchDetail,
    // 初始化
    initLoad,
  })
}

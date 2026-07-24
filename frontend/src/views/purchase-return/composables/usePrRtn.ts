/**
 * usePrRtn.ts - 采购退货核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货列表查询、表单管理、供应商/采购单/产品加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/拒绝/删除）由 usePrRtnProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 */
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getPurchaseReturnById,
  updatePurchaseReturn,
  createPurchaseReturn,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return'
import { useTableApi } from '@/composables/useTableApi'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

/**
 * 采购退货 composable
 * 集中管理列表、表单、供应商、采购单、产品、详情对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePrRtn() {
  // 统计数据（依据列表数据动态计算）
  const stats = reactive({
    total: 0,
    pending: 0,
    approved: 0,
    amount: 0,
  })

  // 日期范围（独立 ref，便于 PurchaseReturnFilter 双向绑定；fetch 前注入 queryParams.startDate/endDate）
  const dateRange = ref<[Date, Date] | null>(null)

  // 列表数据接入 useTableApi
  // 采购退货 API 使用 camelCase 分页参数（pageSize），需显式配置 pageSizeKey
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchData,
  } = useTableApi<PurchaseReturn>({
    url: '/purchase/returns',
    defaultPageSize: 20,
    pageKey: 'page',
    pageSizeKey: 'pageSize',
    defaultParams: {
      keyword: '',
      supplierId: undefined as number | undefined,
      status: '',
      startDate: '',
      endDate: '',
    },
    onError: (err: unknown) => {
      logger.error('获取数据失败:', err)
    },
  })

  // 监听列表/总数变化，同步统计字段（保持原 fetchData 中 stats 更新行为）
  watch(
    [tableData, total],
    () => {
      stats.total = total.value
      stats.pending = tableData.value.filter(i => i.status === 'pending').length
      stats.approved = tableData.value.filter(i => i.status === 'approved').length
      stats.amount = tableData.value.reduce((sum, i) => sum + (i.totalAmount || 0), 0)
    },
    { deep: false },
  )

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

  /** 同步 dateRange 到 queryParams.startDate/endDate */
  const syncDateRangeToQuery = () => {
    if (dateRange.value) {
      queryParams.value = {
        ...queryParams.value,
        startDate: dateRange.value[0].toISOString(),
        endDate: dateRange.value[1].toISOString(),
      }
    } else {
      queryParams.value = {
        ...queryParams.value,
        startDate: '',
        endDate: '',
      }
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

  /** 查询：先同步日期范围，重置页码，触发加载 */
  const handleQuery = () => {
    syncDateRangeToQuery()
    page.value = 1
    fetchData()
  }

  /** 重置：清空筛选条件 + 日期 + 重置页码，触发加载 */
  const handleReset = () => {
    queryParams.value = {
      ...queryParams.value,
      keyword: '',
      supplierId: undefined,
      status: '',
      startDate: '',
      endDate: '',
    }
    dateRange.value = null
    page.value = 1
    fetchData()
  }

  /** 日期范围变化：同步 dateRange 后立即查询 */
  const handleDateChange = (v: [Date, Date] | null) => {
    dateRange.value = v
    handleQuery()
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
      const res = await getPurchaseReturnById(id)
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
  const handleProductChange = (row: Partial<PurchaseReturnItem>, productId: number) => {
    const product = products.value.find(p => p.id === productId)
    if (product) {
      row.productName = product.name
      row.unitPrice = product.price
    }
  }

  /** 提交表单（新建/编辑） */
  const handleFormSubmit = async (isEdit: boolean): Promise<boolean> => {
    try {
      // 显式字段映射（避免 as unknown as 双重断言）；items 单重 as：Partial<T>[] → T[] 合法
      const submitData: Partial<PurchaseReturn> = {
        id: formData.id,
        purchaseOrderId: formData.purchaseOrderId,
        returnDate: formData.returnDate,
        reason: formData.reason,
        remarks: formData.remarks,
        items: formData.items as PurchaseReturnItem[],
      }
      if (isEdit && formData.id) {
        await updatePurchaseReturn(formData.id, submitData)
        ElMessage.success('更新成功')
      } else {
        await createPurchaseReturn(submitData)
        ElMessage.success('创建成功')
      }
      fetchData()
      return true
    } catch (error) {
      logger.error('提交失败:', error)
      return false
    }
  }

  /** 初始化加载（仅加载辅助数据，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
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
    page,
    pageSize,
    queryParams,
    handleQuery,
    handleReset,
    handleDateChange,
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

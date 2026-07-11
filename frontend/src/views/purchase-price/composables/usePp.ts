/**
 * usePp.ts - 采购价格核心 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）
 * 提供采购价格列表查询、表单管理、供应商/产品加载、CRUD 等核心方法
 * 业务流程（停用/导出/查看/历史）由 usePpProc 提供
 * 批次 285：priceList 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { FormInstance } from 'element-plus'
import {
  createPurchasePrice,
  updatePurchasePrice,
  type PurchasePrice,
} from '@/api/purchase-price'
import { supplierApi, type Supplier } from '@/api/supplier'
import { productApi, type Product } from '@/api/product'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/**
 * 采购价格 composable
 * 集中管理列表、表单、供应商、产品、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePp() {
  // 列表 - 接入 useTableApi（批次 285）
  const {
    data: priceList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: getList,
  } = useTableApi<PurchasePrice>({
    url: '/purchase/purchase-prices',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      product_id: undefined as number | undefined,
      status: '',
    },
    onError: (err: unknown) => {
      // 使用类型守卫安全提取错误信息
      const errMsg = err instanceof Error ? err.message : ''
      ElMessage.error(errMsg || '获取采购价格列表失败')
    },
  })

  // 供应商和产品列表
  const suppliers = ref<Supplier[]>([])
  const products = ref<Product[]>([])

  // 对话框
  const dialogTitle = ref('')
  const formRef = ref<FormInstance>()

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    product_id: undefined as number | undefined,
    supplier_id: undefined as number | undefined,
    price: 0,
    currency: 'CNY',
    unit: 'meter',
    min_order_qty: 0,
    price_type: 'STANDARD',
    effective_date: '',
    expiry_date: '',
    remarks: '',
  })

  // 表单验证规则
  const formRules = {
    product_id: [{ required: true, message: '请选择产品', trigger: 'change' }],
    supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
    price: [{ required: true, message: '请输入采购价格', trigger: 'blur' }],
    currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
    unit: [{ required: true, message: '请选择单位', trigger: 'change' }],
    effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
  }

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  /** 获取供应商列表 */
  const getSuppliers = async () => {
    try {
      const res = await supplierApi.list({ page: 1, page_size: 1000 })
      suppliers.value = res.data?.list || []
    } catch (error) {
      logger.error('获取供应商列表失败:', error)
    }
  }

  /** 获取产品列表 */
  const getProducts = async () => {
    try {
      const res = await productApi.list({ page: 1, page_size: 1000 })
      products.value = res.data?.list || []
    } catch (error) {
      logger.error('获取产品列表失败:', error)
    }
  }

  /** 查询 */
  const handleQuery = () => {
    page.value = 1
    getList()
  }

  /** 重置 */
  const handleReset = () => {
    queryParams.value = {
      keyword: '',
      supplier_id: undefined,
      product_id: undefined,
      status: '',
    }
    page.value = 1
    getList()
  }

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = '新建采购价格'
    Object.assign(formData, {
      id: undefined,
      product_id: undefined,
      supplier_id: undefined,
      price: 0,
      currency: 'CNY',
      unit: 'meter',
      min_order_qty: 0,
      price_type: 'STANDARD',
      effective_date: '',
      expiry_date: '',
      remarks: '',
    })
  }

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: PurchasePrice) => {
    dialogTitle.value = '编辑采购价格'
    Object.assign(formData, row)
  }

  /** 提交表单 */
  const handleSubmitForm = async (): Promise<boolean> => {
    try {
      await formRef.value?.validate()
      if (formData.id) {
        await updatePurchasePrice(formData.id, formData)
      } else {
        await createPurchasePrice(formData)
      }
      ElMessage.success('保存成功')
      await getList()
      return true
    } catch (error: unknown) {
      // 使用类型守卫安全提取错误信息
      if (error instanceof Error && error.message) {
        ElMessage.error(error.message || '操作失败')
      }
      return false
    }
  }

  /** 初始化加载辅助数据（懒加载供应商/产品，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('suppliers', getSuppliers, hasLoaded)
    loadIfNot('products', getProducts, hasLoaded)
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查询与列表（useTableApi 管理）
    queryParams,
    page,
    pageSize,
    loading,
    priceList,
    total,
    getList,
    handleQuery,
    handleReset,
    // 供应商与产品
    suppliers,
    products,
    getSuppliers,
    getProducts,
    // 对话框与表单
    dialogTitle,
    formRef,
    formData,
    formRules,
    prepareCreate,
    prepareEdit,
    handleSubmitForm,
    // 初始化
    initLoad,
  })
}

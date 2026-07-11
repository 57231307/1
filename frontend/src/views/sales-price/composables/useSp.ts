/**
 * useSp.ts - 销售价格核心 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）
 * 提供销售价格列表查询、表单管理、客户/产品加载、CRUD 等核心方法
 * 业务流程（审批/导出/查看）由 useSpProc 提供
 * 批次 284：priceList 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { FormInstance } from 'element-plus'
import {
  createSalesPrice,
  updateSalesPrice,
  type SalesPrice,
} from '@/api/sales-price'
import { customerApi, type Customer } from '@/api/customer'
import { productApi, type Product } from '@/api/product'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/**
 * 销售价格 composable
 * 集中管理列表、表单、客户、产品、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSp() {
  // 列表 - 接入 useTableApi（批次 284）
  const {
    data: priceList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: getList,
  } = useTableApi<SalesPrice>({
    url: '/sales/sales-prices',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      customer_id: undefined as number | undefined,
      product_id: undefined as number | undefined,
      status: '',
    },
    onError: (err: unknown) => {
      // 使用类型守卫安全提取错误信息
      const errMsg = err instanceof Error ? err.message : ''
      ElMessage.error(errMsg || '获取销售价格列表失败')
    },
  })

  // 客户和产品列表
  const customers = ref<Customer[]>([])
  const products = ref<Product[]>([])

  // 对话框
  const dialogTitle = ref('')
  const formRef = ref<FormInstance>()

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    product_id: undefined as number | undefined,
    customer_id: undefined as number | undefined,
    price: 0,
    currency: 'CNY',
    unit: 'meter',
    min_order_qty: 0,
    price_type: 'STANDARD',
    price_level: '',
    effective_date: '',
    expiry_date: '',
    remarks: '',
  })

  // 表单验证规则
  const formRules = {
    product_id: [{ required: true, message: '请选择产品', trigger: 'change' }],
    price: [{ required: true, message: '请输入销售价格', trigger: 'blur' }],
    currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
    unit: [{ required: true, message: '请选择单位', trigger: 'change' }],
    effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
    price_type: [{ required: true, message: '请选择价格类型', trigger: 'change' }],
  }

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  /** 获取客户列表 */
  const getCustomers = async () => {
    try {
      const res = await customerApi.list({ page: 1, page_size: 1000 })
      customers.value = res.data?.list || []
    } catch (error) {
      logger.error('获取客户列表失败:', error)
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
      customer_id: undefined,
      product_id: undefined,
      status: '',
    }
    page.value = 1
    getList()
  }

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = '新建销售价格'
    Object.assign(formData, {
      id: undefined,
      product_id: undefined,
      customer_id: undefined,
      price: 0,
      currency: 'CNY',
      unit: 'meter',
      min_order_qty: 0,
      price_type: 'STANDARD',
      price_level: '',
      effective_date: '',
      expiry_date: '',
      remarks: '',
    })
  }

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: SalesPrice) => {
    dialogTitle.value = '编辑销售价格'
    Object.assign(formData, row)
  }

  /** 提交表单 */
  const handleSubmitForm = async (): Promise<boolean> => {
    try {
      await formRef.value?.validate()
      if (formData.id) {
        await updateSalesPrice(formData.id, formData)
      } else {
        await createSalesPrice(formData)
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

  /** 初始化加载辅助数据（懒加载客户/产品，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('customers', getCustomers, hasLoaded)
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
    // 客户与产品
    customers,
    products,
    getCustomers,
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

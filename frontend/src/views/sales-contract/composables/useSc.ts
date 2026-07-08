/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useSc.ts - 销售合同核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 sales-contract/index.vue）
 * 提供销售合同列表查询、表单管理、客户加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/执行/打印/导出）由 useScProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  listSalesContracts,
  createSalesContract,
  updateSalesContract,
  type SalesContract,
} from '@/api/sales-contract'
import { customerApi, type Customer } from '@/api/customer'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

/**
 * 销售合同 composable
 * 集中管理列表、表单、客户、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSc() {
  // 查询参数
  const queryParams = reactive({
    page: 1,
    page_size: 20,
    keyword: '',
    customer_id: undefined as number | undefined,
    status: '',
    signed_date_from: '',
    signed_date_to: '',
  })

  const dateRange = ref<[Date, Date] | null>(null)

  // 列表数据
  const loading = ref(false)
  const contractList = ref<SalesContract[]>([])
  const total = ref(0)

  // 客户列表
  const customers = ref<Customer[]>([])

  // 对话框
  const dialogTitle = ref('')

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    contract_no: '',
    contract_name: '',
    customer_id: undefined as number | undefined,
    contract_type: '',
    total_amount: 0,
    signed_date: '',
    effective_date: '',
    expiry_date: '',
    payment_terms: '',
    payment_method: '',
    delivery_date: '',
    delivery_location: '',
    remarks: '',
  })

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  /** 处理日期范围变化 */
  const handleDateChange = () => {
    if (dateRange.value) {
      queryParams.signed_date_from = dateRange.value[0].toISOString().split('T')[0]
      queryParams.signed_date_to = dateRange.value[1].toISOString().split('T')[0]
    } else {
      queryParams.signed_date_from = ''
      queryParams.signed_date_to = ''
    }
    handleQuery()
  }

  /** 获取列表 */
  const getList = async () => {
    loading.value = true
    try {
      const res = await listSalesContracts(queryParams)
      contractList.value = res.data?.list || []
      total.value = res.data?.total || 0
    } catch (error: unknown) {
      // 使用类型守卫安全提取错误信息
      const errMsg = error instanceof Error ? error.message : ''
      ElMessage.error(errMsg || '获取销售合同列表失败')
    } finally {
      loading.value = false
    }
  }

  /** 获取客户列表 */
  const getCustomers = async () => {
    try {
      const res = await customerApi.list()
      customers.value = res.data?.list || []
    } catch (error) {
      logger.error('获取客户列表失败:', error)
    }
  }

  /** 查询 */
  const handleQuery = () => {
    queryParams.page = 1
    getList()
  }

  /** 重置 */
  const handleReset = () => {
    queryParams.keyword = ''
    queryParams.customer_id = undefined
    queryParams.status = ''
    dateRange.value = null
    queryParams.signed_date_from = ''
    queryParams.signed_date_to = ''
    handleQuery()
  }

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = '新建销售合同'
    Object.assign(formData, {
      id: undefined,
      contract_no: '',
      contract_name: '',
      customer_id: undefined,
      contract_type: '',
      total_amount: 0,
      signed_date: '',
      effective_date: '',
      expiry_date: '',
      payment_terms: '',
      payment_method: '',
      delivery_date: '',
      delivery_location: '',
      remarks: '',
    })
  }

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: SalesContract) => {
    dialogTitle.value = '编辑销售合同'
    Object.assign(formData, row)
  }

  /** 提交表单 */
  const handleSubmitForm = async () => {
    try {
      if (formData.id) {
        await updateSalesContract(formData.id, formData)
      } else {
        await createSalesContract(formData)
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

  /** 分页 - 每页大小 */
  const handleSizeChange = (val: number) => {
    queryParams.page_size = val
    getList()
  }

  /** 分页 - 当前页 */
  const handleCurrentChange = (val: number) => {
    queryParams.page = val
    getList()
  }

  /** 初始化加载（懒加载客户） */
  const initLoad = () => {
    getList()
    loadIfNot('customers', getCustomers, hasLoaded)
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查询与列表
    queryParams,
    dateRange,
    handleDateChange,
    loading,
    contractList,
    total,
    getList,
    handleQuery,
    handleReset,
    handleSizeChange,
    handleCurrentChange,
    // 客户
    customers,
    getCustomers,
    // 对话框与表单
    dialogTitle,
    formData,
    prepareCreate,
    prepareEdit,
    handleSubmitForm,
    // 初始化
    initLoad,
  })
}

export type ScLazyLoader = Record<string, boolean>

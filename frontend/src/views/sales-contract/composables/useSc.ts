/**
 * useSc.ts - 销售合同核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 sales-contract/index.vue）
 * 提供销售合同列表查询、表单管理、客户加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/执行/打印/导出）由 useScProc 提供
 * 批次 284：contractList 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  createSalesContract,
  updateSalesContract,
  type SalesContract,
} from '@/api/sales-contract'
// D14 Batch 5b：原 customerApi 对象已转风格 B 函数
import { getCustomerList, type Customer } from '@/api/customer'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/**
 * 销售合同 composable
 * 集中管理列表、表单、客户、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSc() {
  // 列表 - 接入 useTableApi（批次 284）
  const {
    data: contractList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    setQueryParam,
    refresh: getList,
  } = useTableApi<SalesContract>({
    url: '/sales/sales-contracts',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      customer_id: undefined as number | undefined,
      status: '',
      signed_date_from: '',
      signed_date_to: '',
    },
    onError: (err: unknown) => {
      // 使用类型守卫安全提取错误信息
      const errMsg = err instanceof Error ? err.message : ''
      ElMessage.error(errMsg || '获取销售合同列表失败')
    },
  })

  // 日期范围（ScFilter 通过 date-change emit 回传，保留特殊处理）
  const dateRange = ref<[Date, Date] | null>(null)

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

  /** 处理日期范围变化（批次 284：改为 setQueryParam + page=1 + refresh） */
  const handleDateChange = () => {
    if (dateRange.value) {
      setQueryParam('signed_date_from', dateRange.value[0].toISOString().split('T')[0])
      setQueryParam('signed_date_to', dateRange.value[1].toISOString().split('T')[0])
    } else {
      setQueryParam('signed_date_from', '')
      setQueryParam('signed_date_to', '')
    }
    page.value = 1
    getList()
  }

  /** 获取客户列表 */
  const getCustomers = async () => {
    try {
      const res = await getCustomerList()
      customers.value = res.data?.list || []
    } catch (error) {
      logger.error('获取客户列表失败:', error)
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
      status: '',
      signed_date_from: '',
      signed_date_to: '',
    }
    dateRange.value = null
    page.value = 1
    getList()
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

  /** 初始化加载辅助数据（懒加载客户，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('customers', getCustomers, hasLoaded)
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查询与列表（useTableApi 管理）
    queryParams,
    page,
    pageSize,
    loading,
    contractList,
    total,
    getList,
    handleQuery,
    handleReset,
    // 日期范围
    dateRange,
    handleDateChange,
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

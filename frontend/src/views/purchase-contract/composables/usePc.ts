/**
 * usePc.ts - 采购合同核心 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-contract/index.vue）
 * 提供采购合同列表查询、表单管理、供应商加载、CRUD 等核心方法
 * 业务流程（提交/审批/执行/删除/导出）由 usePcProc 提供
 * 批次 284：contractList 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { FormInstance } from 'element-plus'
import {
  createPurchaseContract,
  updatePurchaseContract,
  type PurchaseContract,
} from '@/api/purchase-contract'
import { supplierApi, type Supplier } from '@/api/supplier'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/**
 * 采购合同 composable
 * 集中管理列表、表单、供应商、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePc() {
  // 列表 - 接入 useTableApi（批次 284）
  const {
    data: contractList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: getList,
  } = useTableApi<PurchaseContract>({
    url: '/purchase/purchase-contracts',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      status: '',
      date_range: [] as string[],
    },
    onError: (err: unknown) => {
      logger.error('获取采购合同列表失败:', err)
    },
  })

  // 供应商列表
  const suppliers = ref<Supplier[]>([])

  // 对话框
  const dialogTitle = ref('')
  const formRef = ref<FormInstance>()

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    contract_no: '',
    contract_name: '',
    supplier_id: undefined as number | undefined,
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

  // 表单验证规则
  const formRules = {
    contract_no: [{ required: true, message: '请输入合同编号', trigger: 'blur' }],
    contract_name: [{ required: true, message: '请输入合同名称', trigger: 'blur' }],
    supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  }

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  /** 获取供应商列表 */
  const getSuppliers = async () => {
    try {
      const res = await supplierApi.list()
      suppliers.value = res.data?.list || []
    } catch (error) {
      logger.error('获取供应商列表失败:', error)
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
      status: '',
      date_range: [],
    }
    page.value = 1
    getList()
  }

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = '新建采购合同'
    Object.assign(formData, {
      id: undefined,
      contract_no: '',
      contract_name: '',
      supplier_id: undefined,
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
  const prepareEdit = (row: PurchaseContract) => {
    dialogTitle.value = '编辑采购合同'
    Object.assign(formData, row)
  }

  /** 提交表单 */
  const handleSubmitForm = async (): Promise<boolean> => {
    try {
      await formRef.value?.validate()
      if (formData.id) {
        await updatePurchaseContract(formData.id, formData)
      } else {
        await createPurchaseContract(formData)
      }
      ElMessage.success('保存成功')
      await getList()
      return true
    } catch (error) {
      logger.error('表单验证失败:', error)
      return false
    }
  }

  /** 初始化加载辅助数据（懒加载供应商，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('suppliers', getSuppliers, hasLoaded)
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
    // 供应商
    suppliers,
    getSuppliers,
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

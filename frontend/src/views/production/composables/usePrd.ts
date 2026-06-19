/**
 * usePrd.ts - 生产管理核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 production/index.vue）
 * 提供列表数据加载（V2Table + useTableApi）+ 过滤表单 + 订单表单状态
 * 业务流程（CRUD + 状态变更 + 导出 + 打印）由 usePrdProc 提供
 * 行为完全保持一致（仅结构重构）
 *
 * 注意：返回值中包含 Ref（page/pageSize/data/loading/total 等）和 reactive 对象
 * （queryForm/orderForm），未用 reactive({...}) 包装，避免 ref 自动解包后无法回写
 */
import { ref, reactive } from 'vue'
import { useTableApi } from '@/composables/useTableApi'
import type { ProductionOrder } from '@/api/production'

/**
 * 订单表单字段类型（所有字段可选，兼容 Partial<ProductionOrder>）
 */
export interface PrdOrderForm {
  id?: number | undefined
  order_no?: string
  product_id?: number | undefined
  planned_quantity?: number | undefined
  scheduled_start_date?: string
  scheduled_end_date?: string
  status?: string
  priority?: number
  work_center_id?: number | undefined
  remark?: string
}

/**
 * 生产管理主业务 composable
 * 集中管理 V2Table 列表数据、过滤表单、订单表单
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePrd() {
  // V2Table 数据
  const {
    data,
    loading,
    page,
    pageSize,
    total,
    queryParams,
    refresh,
    reset,
    setQueryParam,
  } = useTableApi<ProductionOrder>('/production/orders')

  // 提交对话框 loading
  const submitLoading = ref(false)

  // 过滤表单（仅 UI 状态，提交时同步到 queryParams）
  const queryForm = reactive({
    order_no: '',
    status: '',
  })

  // 订单表单
  const orderForm = reactive<PrdOrderForm>({
    id: undefined,
    order_no: '',
    product_id: undefined,
    planned_quantity: undefined,
    scheduled_start_date: '',
    scheduled_end_date: '',
    status: 'draft',
    priority: 5,
    work_center_id: undefined,
    remark: '',
  })

  // 表单验证规则
  const orderRules = {
    order_no: [{ required: true, message: '请输入订单编号', trigger: 'blur' }],
    product_id: [{ required: true, message: '请输入产品ID', trigger: 'blur' }],
    planned_quantity: [{ required: true, message: '请输入计划数量', trigger: 'blur' }],
    priority: [{ required: true, message: '请选择优先级', trigger: 'change' }],
  }

  // 重置订单表单
  const resetOrderForm = () => {
    Object.assign(orderForm, {
      id: undefined,
      order_no: '',
      product_id: undefined,
      planned_quantity: undefined,
      scheduled_start_date: '',
      scheduled_end_date: '',
      status: 'draft',
      priority: 5,
      work_center_id: undefined,
      remark: '',
    })
  }

  /**
   * 将过滤表单同步到 queryParams 并刷新列表
   */
  const applyQuery = () => {
    setQueryParam('order_no', queryForm.order_no || undefined)
    setQueryParam('status', queryForm.status || undefined)
    page.value = 1
    refresh()
  }

  /** 重置过滤表单 + 列表 */
  const resetQuery = () => {
    queryForm.order_no = ''
    queryForm.status = ''
    reset()
    refresh()
  }

  // 直接返回（不包装为 reactive）保持 ref 行为一致
  return reactive({
    // V2Table 数据
    data,
    loading,
    page,
    pageSize,
    total,
    queryParams,
    refresh,
    reset,
    // 提交状态
    submitLoading,
    // 过滤表单
    queryForm,
    // 查询操作
    applyQuery,
    resetQuery,
    // 订单表单
    orderForm,
    orderRules,
    resetOrderForm,
  })
}

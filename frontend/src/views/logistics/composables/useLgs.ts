/**
 * useLgs.ts - 物流管理核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 logistics/index.vue）
 * 提供运单列表查询 / 统计 / 关联订单加载 / 过滤表单 / 订单表单状态
 * 业务流程（创建 / 编辑 / 查看 / 发货 / 更新状态 / 删除）由 useLgsProc 提供
 * 行为完全保持一致（仅结构重构）
 *
 * 注意：返回值中包含 Ref 和 reactive 对象，未用 reactive({...}) 包装，
 * 避免 ref 自动解包后无法回写
 */
import { ref, reactive } from 'vue'
import type { FormInstance } from 'element-plus'
import { logisticsApi, type LogisticsWaybill } from '@/api/logistics'
import { logger } from '@/utils/logger'

/**
 * 订单表单字段类型
 */
export interface LgsFormData {
  id?: number | undefined
  order_id?: number | undefined
  logistics_company?: string
  tracking_number?: string
  driver_name?: string
  driver_phone?: string
  freight_fee?: number
  expected_arrival?: string
  notes?: string
}

/**
 * 物流管理主业务 composable
 * 集中管理列表数据、统计、关联订单、过滤表单、订单表单
 * 对话框可见性由父组件本地 ref 管理
 */
export function useLgs() {
  // 统计数据
  const stats = reactive({
    total: 0,
    pending: 0,
    inTransit: 0,
    delivered: 0,
  })

  // 表格数据
  const tableData = ref<LogisticsWaybill[]>([])
  const loading = ref(false)
  const total = ref(0)
  const dateRange = ref<[Date, Date] | null>(null)

  // 查询参数
  const queryParams = reactive({
    page: 1,
    page_size: 20,
    keyword: '',
    logistics_company: '',
    status: '',
  })

  // 关联订单列表
  const orders = ref<{ id: number; order_no: string }[]>([])

  // 对话框
  const dialogVisible = ref(false)
  const isEdit = ref(false)
  const submitLoading = ref(false)
  const formRef = ref<FormInstance>()
  const formData = reactive<LgsFormData>({
    id: undefined,
    order_id: undefined,
    logistics_company: '',
    tracking_number: '',
    driver_name: '',
    driver_phone: '',
    freight_fee: 0,
    expected_arrival: '',
    notes: '',
  })
  const formRules = {
    logistics_company: [{ required: true, message: '请选择物流公司', trigger: 'change' }],
    tracking_number: [{ required: true, message: '请输入快递单号', trigger: 'blur' }],
  }

  // 详情对话框
  const detailDialogVisible = ref(false)
  const detailData = ref<LogisticsWaybill>({} as LogisticsWaybill)

  // 状态更新对话框
  const statusDialogVisible = ref(false)
  const statusForm = reactive({
    id: 0,
    currentStatus: '',
    newStatus: '',
  })

  /**
   * 拉取运单列表 + 更新统计
   */
  const fetchData = async () => {
    loading.value = true
    try {
      const params: Record<string, unknown> = { ...queryParams }
      if (dateRange.value) {
        params.start_date = dateRange.value[0].toISOString()
        params.end_date = dateRange.value[1].toISOString()
      }
      const res = await logisticsApi.list(params as Record<string, unknown>)
      tableData.value = res.data?.list || []
      total.value = res.data?.total || 0

      // 更新统计
      stats.total = total.value
      stats.pending = tableData.value.filter(i => i.status === 'pending').length
      stats.inTransit = tableData.value.filter(i => i.status === 'in_transit').length
      stats.delivered = tableData.value.filter(i => i.status === 'delivered').length
    } catch (error) {
      logger.error('获取数据失败:', error)
    } finally {
      loading.value = false
    }
  }

  /**
   * 加载关联订单列表（模拟）
   */
  const fetchOrders = async () => {
    orders.value = [
      { id: 1, order_no: 'SO20260101001' },
      { id: 2, order_no: 'SO20260101002' },
    ]
  }

  /** 查询 */
  const handleQuery = () => {
    queryParams.page = 1
    fetchData()
  }

  /** 重置查询 */
  const handleReset = () => {
    queryParams.keyword = ''
    queryParams.logistics_company = ''
    queryParams.status = ''
    dateRange.value = null
    queryParams.page = 1
    fetchData()
  }

  // 直接返回（不包装为 reactive）保持 ref 行为一致
  return {
    // 统计
    stats,
    // 表格
    tableData,
    loading,
    total,
    dateRange,
    // 查询
    queryParams,
    handleQuery,
    handleReset,
    // 关联订单
    orders,
    fetchOrders,
    // 运单表单
    dialogVisible,
    isEdit,
    submitLoading,
    formRef,
    formData,
    formRules,
    // 详情
    detailDialogVisible,
    detailData,
    // 状态
    statusDialogVisible,
    statusForm,
    // 列表拉取
    fetchData,
  }
}

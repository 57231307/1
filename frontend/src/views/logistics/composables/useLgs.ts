/**
 * useLgs.ts - 物流管理核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 logistics/index.vue）
 * 提供运单列表查询 / 统计 / 关联订单加载 / 过滤表单 / 订单表单状态
 * 业务流程（创建 / 编辑 / 查看 / 发货 / 更新状态 / 删除）由 useLgsProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 287：tableData 接入 useTableApi，移除手写分页逻辑
 *
 * 设计说明：返回 reactive({...})，父组件可直接访问字段；
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 * （日期范围由 LgsFilter 发出事件，父组件更新自身 dateRange）
 */
import { ref, reactive, watch } from 'vue'
import { logisticsApi, type LogisticsWaybill } from '@/api/logistics'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'
import type { LgsStatusForm } from './useLgsProc'

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
  // 统计数据（依据列表数据动态计算）
  const stats = reactive({
    total: 0,
    pending: 0,
    inTransit: 0,
    delivered: 0,
  })

  // 日期范围（独立 ref，便于 LgsFilter 双向绑定；fetch 前注入 queryParams.start_date/end_date）
  const dateRange = ref<[Date, Date] | null>(null)

  // 列表数据接入 useTableApi
  // 物流 API 使用 snake_case 分页参数（page/page_size），匹配 useTableApi 默认配置
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchData,
  } = useTableApi<LogisticsWaybill>({
    url: '/inventory/logistics',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      logistics_company: '',
      status: '',
      start_date: '',
      end_date: '',
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
      stats.inTransit = tableData.value.filter(i => i.status === 'in_transit').length
      stats.delivered = tableData.value.filter(i => i.status === 'delivered').length
    },
    { deep: false },
  )

  // 关联订单列表
  const orders = ref<{ id: number; order_no: string }[]>([])

  // 对话框
  const dialogVisible = ref(false)
  const isEdit = ref(false)
  const submitLoading = ref(false)
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
  const statusForm = reactive<LgsStatusForm>({
    id: 0,
    currentStatus: '',
    newStatus: '',
  })

  /** 同步 dateRange 到 queryParams.start_date/end_date */
  const syncDateRangeToQuery = () => {
    if (dateRange.value) {
      queryParams.value = {
        ...queryParams.value,
        start_date: dateRange.value[0].toISOString(),
        end_date: dateRange.value[1].toISOString(),
      }
    } else {
      queryParams.value = {
        ...queryParams.value,
        start_date: '',
        end_date: '',
      }
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
      logistics_company: '',
      status: '',
      start_date: '',
      end_date: '',
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

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 统计
    stats,
    // 表格
    tableData,
    loading,
    total,
    dateRange,
    page,
    pageSize,
    queryParams,
    // 查询
    handleQuery,
    handleReset,
    handleDateChange,
    syncDateRangeToQuery,
    // 关联订单
    orders,
    fetchOrders,
    // 运单表单
    dialogVisible,
    isEdit,
    submitLoading,
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
  })
}

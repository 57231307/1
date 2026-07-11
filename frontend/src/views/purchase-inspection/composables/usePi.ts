/**
 * usePi.ts - 采购验货核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 * 提供检验单列表 / 统计 / 分页 / 过滤 / 表单 / 详情 / 选项加载等核心方法
 * 业务流程（查询 / 重置 / 创建 / 编辑 / 查看 / 提交 / 完成）由 usePiProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 */
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { type PurchaseInspection, type PurchaseInspectionItem } from '@/api/purchase-inspection'
import { getReceiptItems, type ReceiptItem } from '@/api/purchaseReceipt'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

/**
 * 采购验货主业务 composable
 * 集中管理列表、统计、过滤、表单、详情、选项加载
 */
export function usePi() {
  // 统计数据（依据列表数据动态计算）
  const stats = reactive({
    total: 0,
    pending: 0,
    passed: 0,
    failed: 0,
  })

  // 日期范围（独立 ref，便于 PiFilter 双向绑定；fetch 前注入 queryParams.inspection_date_from/to）
  const dateRange = ref<[Date, Date] | null>(null)

  // 列表数据接入 useTableApi
  // 采购验货 API 使用 snake_case 分页参数（page/page_size），匹配 useTableApi 默认配置
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchData,
  } = useTableApi<PurchaseInspection>({
    url: '/purchase/inspections',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      status: '',
      result: '',
      inspection_date_from: '',
      inspection_date_to: '',
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
      stats.passed = tableData.value.filter(i => i.result === 'pass').length
      stats.failed = tableData.value.filter(i => i.result === 'fail').length
    },
    { deep: false },
  )

  // 选项
  const suppliers = ref<{ id: number; name: string }[]>([])
  const receipts = ref<{ id: number; receipt_no: string }[]>([])

  // 对话框
  const dialogVisible = ref(false)
  const isEdit = ref(false)
  const submitLoading = ref(false)
  const formData = reactive<{
    id?: number
    receipt_id?: number
    inspection_date: string
    remark: string
    items: Partial<PurchaseInspectionItem>[]
  }>({
    id: undefined,
    receipt_id: undefined,
    inspection_date: '',
    remark: '',
    items: [],
  })

  const formRules = {
    receipt_id: [{ required: true, message: '请选择入库单', trigger: 'change' }],
    inspection_date: [{ required: true, message: '请选择检验日期', trigger: 'change' }],
  }

  // 详情对话框
  const detailDialogVisible = ref(false)
  const detailData = ref<PurchaseInspection>({} as PurchaseInspection)

  // 入库单明细加载状态
  const receiptItemsLoading = ref(false)

  /** 同步 dateRange 到 queryParams.inspection_date_from/to */
  const syncDateRangeToQuery = () => {
    if (dateRange.value) {
      queryParams.value = {
        ...queryParams.value,
        inspection_date_from: dateRange.value[0].toISOString(),
        inspection_date_to: dateRange.value[1].toISOString(),
      }
    } else {
      queryParams.value = {
        ...queryParams.value,
        inspection_date_from: '',
        inspection_date_to: '',
      }
    }
  }

  /**
   * 加载供应商列表（懒加载）
   */
  const fetchSuppliers = async () => {
    suppliers.value = [
      { id: 1, name: '供应商A' },
      { id: 2, name: '供应商B' },
    ]
  }

  /**
   * 加载入库单列表（懒加载）
   */
  const fetchReceipts = async () => {
    receipts.value = [
      { id: 1, receipt_no: 'RK20260101001' },
      { id: 2, receipt_no: 'RK20260101002' },
    ]
  }

  /**
   * 入库单变化时加载明细
   */
  const handleReceiptChange = async (receiptId: number) => {
    if (!receiptId) {
      formData.items = []
      return
    }
    receiptItemsLoading.value = true
    try {
      const res = await getReceiptItems(receiptId)
      const items: ReceiptItem[] = res.data?.items || []
      if (items.length === 0) {
        ElMessage.info('该入库单暂无明细')
        formData.items = []
        return
      }
      // 将入库单明细映射为检验单明细，初始化各数量字段
      formData.items = items.map(item => ({
        product_id: item.product_id,
        product_name: item.product_name,
        product_code: item.product_code,
        expected_quantity: item.quantity,
        inspected_quantity: 0,
        passed_quantity: 0,
        failed_quantity: 0,
        defect_reason: '',
      }))
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : '获取入库单明细失败，请稍后重试'
      ElMessage.error(errMsg)
      logger.error('获取入库单明细失败:', error)
      formData.items = []
    } finally {
      receiptItemsLoading.value = false
    }
  }

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  // 使用 reactive 包装，父组件可直接访问字段
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
    // 选项
    suppliers,
    receipts,
    // 表单对话框
    dialogVisible,
    isEdit,
    submitLoading,
    formData,
    formRules,
    // 详情对话框
    detailDialogVisible,
    detailData,
    // 入库单明细加载
    receiptItemsLoading,
    // 加载方法
    fetchData,
    fetchSuppliers,
    fetchReceipts,
    handleReceiptChange,
    syncDateRangeToQuery,
    // 懒加载标记
    hasLoaded,
    // 兼容旧名（loadIfNot 接受字符串 key）
    loadIfNot,
  })
}

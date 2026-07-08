/**
 * useSpProc.ts - 销售价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）
 * 封装销售价格审批/查看/历史/导出等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  approveSalesPrice,
  getPriceHistory,
  listPricingStrategies,
  type SalesPrice,
  type PricingStrategy,
} from '@/api/sales-price'
import { getPriceTypeLabel, getStatusLabel } from './spFmts'
import { exportToExcel } from '@/utils/export'

/** 刷新回调 */
interface RefreshCallbacks {
  getList: () => Promise<void>
}

/**
 * 销售价格流程操作方法集合
 */
export function useSpProc(refresh: RefreshCallbacks) {
  // 查看详情对话框状态
  const viewDialogVisible = ref(false)
  // v11 批次 174 P2-1 修复：ref<any>({}) 改为 ref<SalesPrice>，初始空对象通过断言
  const viewData = ref<SalesPrice>({} as SalesPrice)

  // 历史记录对话框状态
  const historyVisible = ref(false)
  const historyList = ref<SalesPrice[]>([])

  // 价格策略对话框状态（批次 95 P3-17 修复）
  const strategyVisible = ref(false)
  const strategyList = ref<PricingStrategy[]>([])
  const strategyLoading = ref(false)

  /** 审批 */
  const handleApprove = async (row: SalesPrice) => {
    try {
      await ElMessageBox.confirm('确认审批通过该价格？', '提示', { type: 'warning' })
      await approveSalesPrice(row.id)
      ElMessage.success('审批成功')
      await refresh.getList()
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error)
        if (errMsg) ElMessage.error(errMsg || '审批失败')
      }
    }
  }

  /** 查看详情（弹出对话框） */
  const handleView = (row: SalesPrice) => {
    viewData.value = row
    viewDialogVisible.value = true
  }

  /** 历史记录 */
  const handleHistory = async (row: SalesPrice) => {
    try {
      const res = await getPriceHistory(row.product_id)
      historyList.value = res.data || []
      historyVisible.value = true
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '获取历史记录失败')
    }
  }

  /** 价格策略（批次 95 P3-17 修复：拉取策略列表并打开对话框） */
  const handleStrategy = async () => {
    strategyVisible.value = true
    strategyLoading.value = true
    try {
      const res = await listPricingStrategies()
      strategyList.value = res.data || []
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '获取价格策略失败')
    } finally {
      strategyLoading.value = false
    }
  }

  /** 导出 Excel（规则 3：禁止 CSV 作为最终交付格式） */
  const handleExport = (priceList: { value: SalesPrice[] } | SalesPrice[]) => {
    const list = Array.isArray(priceList) ? priceList : priceList.value
    exportToExcel({
      filename: '销售价格',
      format: 'excel',
      data: list.map((item): Record<string, unknown> => ({ ...item })),
      columns: [
        { key: 'product_name', title: '产品名称' },
        { key: 'customer_name', title: '客户' },
        { key: 'price', title: '价格' },
        { key: 'currency', title: '币种' },
        { key: 'unit', title: '单位' },
        {
          key: 'min_order_qty',
          title: '最小订购量',
          formatter: (value: unknown) => (value ? String(value) : ''),
        },
        {
          key: 'price_type',
          title: '价格类型',
          formatter: (value: unknown) => getPriceTypeLabel(value ? String(value) : ''),
        },
        {
          key: 'price_level',
          title: '价格等级',
          formatter: (value: unknown) => (value ? String(value) : ''),
        },
        { key: 'effective_date', title: '生效日期' },
        { key: 'expiry_date', title: '到期日期' },
        {
          key: 'status',
          title: '状态',
          formatter: (value: unknown) => getStatusLabel(value as SalesPrice['status']),
        },
      ],
    })
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查看
    viewDialogVisible,
    viewData,
    handleView,
    // 历史
    historyVisible,
    historyList,
    handleHistory,
    // 价格策略（批次 95 P3-17 修复）
    strategyVisible,
    strategyList,
    strategyLoading,
    handleStrategy,
    // 流程
    handleApprove,
    handleExport,
  })
}

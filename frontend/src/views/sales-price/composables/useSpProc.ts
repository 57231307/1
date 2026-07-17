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
// V15 P0-S12 修复（Batch 475d）：导出改用后端带水印 xlsx 接口
// 后端 GET /sales/sales-prices/export 已就绪（含异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export'

/**
 * 刷新回调
 *
 * V15 P0-S12 修复（Batch 475d）：新增 getQueryParams，用于导出时传递列表筛选条件
 * 保证导出数据与当前列表筛选一致（product_id/status）
 */
interface RefreshCallbacks {
  getList: () => Promise<void>
  // V15 P0-S12 修复（Batch 475d）：获取当前筛选条件（product_id/status），用于导出
  getQueryParams?: () => { product_id?: number; status?: string }
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

  /**
   * 导出 Excel（V15 P0-S12 修复 Batch 475d）
   *
   * 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
   * 改为调用后端 GET /sales/sales-prices/export，后端注入水印 + 异步审计日志
   * 传入当前列表筛选条件（product_id/status），保证导出与列表一致
   */
  const handleExport = async () => {
    const filters = refresh.getQueryParams?.() ?? {}
    const params: Record<string, unknown> = {
      product_id: filters.product_id,
      status: filters.status || undefined,
    }
    await exportFromBackend(
      '/sales/sales-prices/export',
      params,
      'sales_prices_export'
    )
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

/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * usePpProc.ts - 采购价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）
 * 封装采购价格停用/查看/历史/导出等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { updatePurchasePrice, getPurchasePriceHistory, type PurchasePrice } from '@/api/purchase-price'
import { logger } from '@/utils/logger'

/** 刷新回调 */
interface RefreshCallbacks {
  getList: () => Promise<void>
}

/**
 * 采购价格流程操作方法集合
 */
export function usePpProc(refresh: RefreshCallbacks) {
  // 查看详情对话框状态
  const viewDialogVisible = ref(false)
  const viewData = ref<Partial<PurchasePrice>>({})

  // 历史记录对话框状态
  const historyVisible = ref(false)
  const historyList = ref<PurchasePrice[]>([])

  /** 停用 */
  const handleDisable = async (row: PurchasePrice) => {
    try {
      await ElMessageBox.confirm('确认停用该价格？', '提示', { type: 'warning' })
      await updatePurchasePrice(row.id, { status: 'inactive' })
      ElMessage.success('停用成功')
      await refresh.getList()
    } catch (error) {
      logger.error('停用失败:', error)
    }
  }

  /** 查看详情（弹出对话框） */
  const handleView = (row: PurchasePrice) => {
    viewData.value = row
    viewDialogVisible.value = true
  }

  /** 历史记录 */
  const handleHistory = async (row: PurchasePrice) => {
    try {
      const res = await getPurchasePriceHistory(row.product_id)
      historyList.value = res.data || []
      historyVisible.value = true
    } catch (error) {
      logger.error('获取历史记录失败:', error)
    }
  }

  /** 导出（占位） */
  const handleExport = () => {
    ElMessage.success('导出成功')
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
    // 流程
    handleDisable,
    handleExport,
  })
}

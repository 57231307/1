/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useSpProc.ts - 销售价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）
 * 封装销售价格审批/查看/历史/导出等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { approveSalesPrice, getPriceHistory, type SalesPrice } from '@/api/sales-price'
import { getPriceTypeLabel, getStatusLabel } from './spFmts'

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
  const viewData = ref<any>({})

  // 历史记录对话框状态
  const historyVisible = ref(false)
  const historyList = ref<SalesPrice[]>([])

  /** 审批 */
  const handleApprove = async (row: SalesPrice) => {
    try {
      await ElMessageBox.confirm('确认审批通过该价格？', '提示', { type: 'warning' })
      await approveSalesPrice(row.id)
      ElMessage.success('审批成功')
      await refresh.getList()
    } catch (error: any) {
      if (error !== 'cancel' && error && error.message) {
        ElMessage.error(error.message || '审批失败')
      }
    }
  }

  /** 查看详情（弹出对话框） */
  const handleView = (row: any) => {
    viewData.value = row
    viewDialogVisible.value = true
  }

  /** 历史记录 */
  const handleHistory = async (row: SalesPrice) => {
    try {
      const res = await getPriceHistory(row.product_id)
      historyList.value = res.data || []
      historyVisible.value = true
    } catch (error: any) {
      ElMessage.error(error.message || '获取历史记录失败')
    }
  }

  /** 价格策略（占位功能） */
  const handleStrategy = () => {
    ElMessage.info('价格策略功能开发中')
  }

  /** 导出 CSV */
  const handleExport = (priceList: { value: SalesPrice[] } | SalesPrice[]) => {
    const list = Array.isArray(priceList) ? priceList : priceList.value
    const csvContent = [
      [
        '产品名称',
        '客户',
        '价格',
        '币种',
        '单位',
        '最小订购量',
        '价格类型',
        '价格等级',
        '生效日期',
        '到期日期',
        '状态',
      ],
      ...list.map((item: any) => [
        item.product_name,
        item.customer_name || '',
        item.price,
        item.currency,
        item.unit,
        item.min_order_qty || '',
        getPriceTypeLabel(item.price_type),
        item.price_level || '',
        item.effective_date || '',
        item.expiry_date || '',
        getStatusLabel(item.status),
      ]),
    ]
      .map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(','))
      .join('\n')
    const blob = new Blob(['\ufeff' + csvContent], { type: 'text/csv;charset=utf-8;' })
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `销售价格_${new Date().toISOString().split('T')[0]}.csv`
    link.click()
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
    handleApprove,
    handleStrategy,
    handleExport,
  })
}

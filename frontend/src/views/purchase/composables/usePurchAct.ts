/**
 * usePurchAct - 采购单业务操作 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue）
 * 包含：审批、查看、打印、导出
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import printJS from 'print-js'
import { purchaseApi, type PurchaseOrder } from '@/api/purchase'

/**
 * 采购单业务操作 composable
 */
export function usePurchAct(
  orders: () => PurchaseOrder[],
  getStatusText: (s: string) => string,
  onRefresh: () => void
) {
  // 查看对话框
  const viewDialogVisible = ref(false)
  const viewData = ref<PurchaseOrder | null>(null)

  /**
   * 查看采购单详情
   */
  const handleView = async (row: PurchaseOrder) => {
    try {
      const res = await purchaseApi.getOrderById(row.id)
      viewData.value = res.data || row
    } catch {
      viewData.value = row
    }
    viewDialogVisible.value = true
  }

  /**
   * 审批采购单
   */
  const handleApprove = async (row: PurchaseOrder) => {
    try {
      await ElMessageBox.confirm(`确定审批通过采购单 ${row.order_no} 吗？`, '审批确认', {
        type: 'success',
      })
      await purchaseApi.approveOrder(row.id)
      ElMessage.success(`采购单 ${row.order_no} 审批成功`)
      onRefresh()
    } catch (error: unknown) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error)
        ElMessage.error(errMsg || '审批失败')
      }
    }
  }

  /**
   * 打印采购订单列表
   */
  const handlePrint = () => {
    // v11 批次 177 P2-1 修复：(item: any) 改为 (item: PurchaseOrder)
    const printData = orders().map((item: PurchaseOrder, index: number) => ({
      序号: index + 1,
      订单号: item.order_no,
      供应商: item.supplier_name,
      金额: `¥${item.total_amount}`,
      状态: getStatusText(item.status),
      创建时间: item.created_at,
    }))
    printJS({
      printable: printData,
      properties: Object.keys(printData[0] || {}),
      type: 'json',
      header: '采购订单列表',
      style: 'padding: 20px; font-size: 14px;',
      headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
      gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
      gridStyle: 'border-collapse: collapse; width: 100%;',
    })
  }

  /**
   * 导出采购订单列表为 CSV
   */
  const handleExport = () => {
    // v11 批次 177 P2-1 修复：(item: any) 改为 (item: PurchaseOrder)，row/cell 改为 unknown
    const csvContent = [
      ['订单号', '供应商', '金额', '状态', '创建时间'],
      ...orders().map((item: PurchaseOrder) => [
        item.order_no,
        item.supplier_name,
        item.total_amount,
        getStatusText(item.status),
        item.created_at,
      ]),
    ]
      .map((row: unknown[]) => row.map((cell: unknown) => `"${String(cell ?? '')}"`).join(','))
      .join('\n')
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `采购订单_${new Date().toISOString().split('T')[0]}.csv`
    link.click()
    ElMessage.success('导出成功')
  }

  return {
    viewDialogVisible,
    viewData,
    handleView,
    handleApprove,
    handlePrint,
    handleExport,
  }
}

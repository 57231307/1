/**
 * usePurchAct - 采购单业务操作 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue）
 * 包含：审批、查看、打印、导出
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import printJS from 'print-js'
import { getPurchaseOrderById, approvePurchaseOrder, type PurchaseOrder } from '@/api/purchase'
// V15 P0-S12 修复（Batch 475b）：导出改用后端带水印 xlsx 接口
// 后端 GET /purchases/orders/export 已就绪（含行级数据权限 + 异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export'

/**
 * 采购单业务操作 composable
 *
 * V15 P0-S12 修复（Batch 475b）：新增第 4 参数 getQueryParams，用于导出时传递列表筛选条件
 * 保证导出数据与当前列表筛选一致（status/supplier_id）
 */
export function usePurchAct(
  orders: () => PurchaseOrder[],
  getStatusText: (s: string) => string,
  onRefresh: () => void,
  getQueryParams: () => { status?: string; supplier_id?: number } = () => ({})
) {
  // 查看对话框
  const viewDialogVisible = ref(false)
  const viewData = ref<PurchaseOrder | null>(null)

  /**
   * 查看采购单详情
   */
  const handleView = async (row: PurchaseOrder) => {
    try {
      const res = await getPurchaseOrderById(row.id)
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
      await approvePurchaseOrder(row.id)
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
   * 导出采购订单列表为 xlsx（V15 P0-S12 修复 Batch 475b）
   *
   * 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
   * 改为调用后端 GET /purchases/orders/export，后端注入水印 + 行级数据权限 + 异步审计日志
   * 传入当前列表筛选条件（status/supplier_id），保证导出与列表一致
   */
  const handleExport = async () => {
    const filters = getQueryParams()
    const params: Record<string, unknown> = {
      status: filters.status || undefined,
      supplier_id: filters.supplier_id,
    }
    await exportFromBackend('/purchases/orders/export', params, 'purchase_orders_export')
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

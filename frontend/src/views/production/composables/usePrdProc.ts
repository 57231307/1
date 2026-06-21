/**
 * usePrdProc.ts - 生产管理流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 production/index.vue）
 * 封装状态变更 / 导出 CSV / 打印等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 usePrd 的状态引用（Reactive 包装层）；
 * 由于 usePrd 返回 reactive({...})，父组件传入 prd.data 等会自动解包为值
 */
import { reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  deleteProductionOrder,
  updateProductionOrderStatus,
  type ProductionOrder,
  PRODUCTION_ORDER_STATUS,
} from '@/api/production'
import { getStatusLabel } from './prdFmts'
import { escapeHtml } from '@/utils/print'

/**
 * 流程回调（接收 usePrd 返回的状态，自动解包后的值类型）
 */
interface PrdCallbacks {
  // 列表数据
  data: ProductionOrder[]
  // 刷新列表
  refresh: () => Promise<void>
}

/**
 * 生产管理流程操作方法集合
 */
export function usePrdProc(cb: PrdCallbacks) {
  /** 状态变更 */
  const handleStatusChange = async (row: ProductionOrder, status: string) => {
    try {
      await ElMessageBox.confirm(
        `确认将订单 ${row.order_no} 状态更改为 ${
          PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label
        } 吗？`,
        '确认',
        { type: 'warning' }
      )
      await updateProductionOrderStatus(row.id, status)
      ElMessage.success('状态更新成功')
      await cb.refresh()
    } catch (e: unknown) {
      if (e !== 'cancel') {
        const err = e as { message?: string }
        ElMessage.error(err.message || '状态更新失败')
      }
    }
  }

  /** 删除订单 */
  const handleDelete = async (row: ProductionOrder) => {
    try {
      await ElMessageBox.confirm(
        `确认删除订单 ${row.order_no} 吗？此操作不可恢复。`,
        '删除确认',
        { type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消' }
      )
      await deleteProductionOrder(row.id)
      ElMessage.success('删除成功')
      await cb.refresh()
    } catch (e: unknown) {
      if (e !== 'cancel') {
        const err = e as { message?: string }
        ElMessage.error(err.message || '删除失败')
      }
    }
  }

  /** 导出 CSV */
  const handleExport = () => {
    const csvContent = [
      ['订单编号', '产品名称', '计划数量', '实际数量', '计划开始', '计划结束', '状态', '优先级'],
      ...cb.data.map((item: ProductionOrder) => [
        item.order_no,
        item.product_name,
        item.planned_quantity,
        item.actual_quantity || '-',
        item.scheduled_start_date?.substring(0, 10) || '-',
        item.scheduled_end_date?.substring(0, 10) || '-',
        getStatusLabel(item.status),
        item.priority,
      ]),
    ]
      .map(row => row.map(cell => `"${cell ?? ''}"`).join(','))
      .join('\n')
    const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `生产订单_${new Date().toISOString().split('T')[0]}.csv`
    link.click()
    ElMessage.success('导出成功')
  }

  /** 打印 */
  const handlePrint = () => {
    const printWindow = window.open('', '_blank')
    if (!printWindow) {
      ElMessage.error('无法打开打印窗口')
      return
    }
    const rows = cb.data
      .map(
        (item: ProductionOrder) => `
    <tr>
      <td>${escapeHtml(item.order_no)}</td><td>${escapeHtml(item.product_name || '-')}</td>
      <td style="text-align:right">${escapeHtml(item.planned_quantity)}</td>
      <td style="text-align:right">${escapeHtml(item.actual_quantity || '-')}</td>
      <td>${escapeHtml(item.scheduled_start_date?.substring(0, 10) || '-')}</td>
      <td>${escapeHtml(item.scheduled_end_date?.substring(0, 10) || '-')}</td>
      <td>${escapeHtml(getStatusLabel(item.status))}</td><td>${escapeHtml(item.priority)}</td>
    </tr>
  `
      )
      .join('')
    printWindow.document.write(`<html><head><meta charset="utf-8"><title>生产订单</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>生产订单列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${cb.data.length} 条</div>
    <table><thead><tr><th>订单编号</th><th>产品名称</th><th>计划数量</th><th>实际数量</th><th>计划开始</th><th>计划结束</th><th>状态</th><th>优先级</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
    printWindow.document.close()
    printWindow.onload = () => printWindow.print()
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    handleStatusChange,
    handleDelete,
    handleExport,
    handlePrint,
  })
}

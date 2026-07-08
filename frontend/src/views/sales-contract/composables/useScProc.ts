/**
 * useScProc.ts - 销售合同流程操作 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 sales-contract/index.vue）
 * 封装销售合同提交审批/审批/执行/删除/打印/导出/查看等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  deleteSalesContract,
  approveSalesContract,
  executeSalesContract,
  type SalesContract,
} from '@/api/sales-contract'
import { formatCurrency, getStatusLabel } from './scFmts'
import { escapeHtml } from '@/utils/print'
import { exportToExcel } from '@/utils/export'

/** 刷新回调 */
interface RefreshCallbacks {
  getList: () => Promise<void>
}

/**
 * 销售合同流程操作方法集合
 */
export function useScProc(refresh: RefreshCallbacks) {
  /** 提交审批 */
  const handleSubmitForApproval = async (row: SalesContract) => {
    try {
      await ElMessageBox.confirm('确认提交该合同审批？', '提示', { type: 'warning' })
      await approveSalesContract(row.id)
      ElMessage.success('提交成功')
      await refresh.getList()
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error)
        ElMessage.error(errMsg || '提交失败')
      }
    }
  }

  /** 审批 */
  const handleApprove = async (row: SalesContract) => {
    try {
      await ElMessageBox.confirm('确认审批通过该合同？', '提示', { type: 'warning' })
      await approveSalesContract(row.id)
      ElMessage.success('审批成功')
      await refresh.getList()
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error)
        ElMessage.error(errMsg || '审批失败')
      }
    }
  }

  /** 执行 */
  const handleExecute = async (row: SalesContract) => {
    try {
      await ElMessageBox.confirm('确认执行该合同？', '提示', { type: 'warning' })
      await executeSalesContract(row.id)
      ElMessage.success('执行成功')
      await refresh.getList()
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error)
        ElMessage.error(errMsg || '执行失败')
      }
    }
  }

  /** 删除 */
  const handleDelete = async (row: SalesContract) => {
    try {
      await ElMessageBox.confirm('确认删除该合同？', '提示', { type: 'warning' })
      await deleteSalesContract(row.id)
      ElMessage.success('删除成功')
      await refresh.getList()
    } catch (error: unknown) {
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : String(error)
        ElMessage.error(errMsg || '删除失败')
      }
    }
  }

  /** 查看详情（弹出 ElMessageBox） */
  const handleView = (row: SalesContract) => {
    ElMessageBox.alert(
      `<div>
        <p><strong>合同编号：</strong>${row.contract_no}</p>
        <p><strong>合同名称：</strong>${row.contract_name}</p>
        <p><strong>客户：</strong>${row.customer_name}</p>
        <p><strong>合同金额：</strong>${formatCurrency(row.total_amount)}</p>
        <p><strong>签订日期：</strong>${row.signed_date || '-'}</p>
        <p><strong>生效日期：</strong>${row.effective_date || '-'}</p>
        <p><strong>到期日期：</strong>${row.expiry_date || '-'}</p>
        <p><strong>付款条件：</strong>${row.payment_terms || '-'}</p>
        <p><strong>付款方式：</strong>${row.payment_method || '-'}</p>
        <p><strong>交货日期：</strong>${row.delivery_date || '-'}</p>
        <p><strong>交货地点：</strong>${row.delivery_location || '-'}</p>
        <p><strong>备注：</strong>${row.remarks || '-'}</p>
      </div>`,
      '合同详情',
      { dangerouslyUseHTMLString: true, confirmButtonText: '关闭' }
    )
  }

  /** 打印当前列表 */
  const handlePrint = (contractList: { value: SalesContract[] } | SalesContract[]) => {
    const list = Array.isArray(contractList) ? contractList : contractList.value
    const printWindow = window.open('', '_blank')
    if (!printWindow) {
      ElMessage.error('无法打开打印窗口')
      return
    }
    const rows = list
      .map(
        // v11 批次 174 P2-1 修复：(item: any) 改为 (item: SalesContract)
        (item: SalesContract) => `
      <tr>
        <td>${escapeHtml(item.contract_no)}</td>
        <td>${escapeHtml(item.contract_name)}</td>
        <td>${escapeHtml(item.customer_name)}</td>
        <td style="text-align:right">${formatCurrency(item.total_amount)}</td>
        <td>${escapeHtml(item.signed_date || '-')}</td>
        <td>${escapeHtml(getStatusLabel(item.status))}</td>
      </tr>
    `
      )
      .join('')
    const now = new Date().toISOString().split('T')[0]
    printWindow.document.write(`
      <html><head><meta charset="utf-8"><title>销售合同列表</title>
      <style>
        @media print { @page { size: landscape; } }
        body { font-family: "Microsoft YaHei", sans-serif; font-size: 12px; }
        h1 { text-align: center; }
        table { width: 100%; border-collapse: collapse; margin-top: 12px; }
        th, td { border: 1px solid #333; padding: 6px 8px; }
        th { background: #f5f5f5; }
        .meta { text-align: center; color: #666; font-size: 11px; }
      </style></head><body>
      <h1>销售合同列表</h1>
      <div class="meta">打印日期: ${now} | 共 ${list.length} 条</div>
      <table>
        <thead><tr><th>合同编号</th><th>合同名称</th><th>客户</th><th>金额</th><th>签订日期</th><th>状态</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
      </body></html>
    `)
    printWindow.document.close()
    printWindow.onload = () => printWindow.print()
  }

  /** 导出 Excel（规则 3：禁止 CSV 作为最终交付格式） */
  const handleExport = (contractList: { value: SalesContract[] } | SalesContract[]) => {
    const list = Array.isArray(contractList) ? contractList : contractList.value
    exportToExcel({
      filename: '销售合同',
      format: 'excel',
      data: list.map((item): Record<string, unknown> => ({ ...item })),
      columns: [
        { key: 'contract_no', title: '合同编号' },
        { key: 'contract_name', title: '合同名称' },
        { key: 'customer_name', title: '客户' },
        { key: 'total_amount', title: '金额' },
        { key: 'signed_date', title: '签订日期' },
        {
          key: 'status',
          title: '状态',
          formatter: (_v: unknown, row: Record<string, unknown>) =>
            getStatusLabel(row.status as SalesContract['status']),
        },
      ],
    })
  }

  return {
    handleSubmitForApproval,
    handleApprove,
    handleExecute,
    handleDelete,
    handleView,
    handlePrint,
    handleExport,
  }
}

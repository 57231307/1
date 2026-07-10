/**
 * useVchrProc.ts - 凭证流程操作 composable
 * 任务编号: P14 批 1 B3 I-2
 * 封装凭证查看、提交、审核、过账、导出、打印等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  submitVoucher as submitVoucherApi,
  reviewVoucher as reviewVoucherApi,
  postVoucher as postVoucherApi,
  type Voucher,
} from '@/api/finance'
import { formatMoney, getVchrStatusLabel } from './vchrFmts'
import { escapeHtml } from '@/utils/print'
import { exportToExcel } from '@/utils/export'

/**
 * 创建凭证流程操作方法集合
 * @param vouchers 凭证列表 ref（用于导出/打印）
 * @param fetchVouchers 重新拉取列表方法
 */
export function useVchrProc(
  vouchers: { value: Voucher[] },
  fetchVouchers: () => Promise<void>
) {
  /** 提交凭证 */
  const submitVoucher = async (row: Voucher) => {
    try {
      await ElMessageBox.confirm('确定提交该凭证吗？', '提交确认', { type: 'info' })
      await submitVoucherApi(row.id)
      ElMessage.success('提交成功')
      await fetchVouchers()
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as Error
        ElMessage.error(err.message || '操作失败')
      }
    }
  }

  /** 审核凭证 */
  const reviewVoucher = async (row: Voucher) => {
    try {
      await ElMessageBox.confirm('确定审核该凭证吗？', '审核确认', { type: 'info' })
      await reviewVoucherApi(row.id)
      ElMessage.success('审核成功')
      await fetchVouchers()
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as Error
        ElMessage.error(err.message || '操作失败')
      }
    }
  }

  /** 过账凭证 */
  const postVoucher = async (row: Voucher) => {
    try {
      await ElMessageBox.confirm('确定过账该凭证吗？过账后不可修改。', '过账确认', {
        type: 'warning',
      })
      await postVoucherApi(row.id)
      ElMessage.success('过账成功')
      await fetchVouchers()
    } catch (error) {
      if (error !== 'cancel') {
        const err = error as Error
        ElMessage.error(err.message || '操作失败')
      }
    }
  }

  /** 导出 Excel（规则 3：禁止 CSV 作为最终交付格式） */
  const handleExportVouchers = () => {
    exportToExcel({
      filename: '凭证列表',
      format: 'excel',
      data: vouchers.value.map((item): Record<string, unknown> => ({ ...item })),
      columns: [
        { key: 'voucher_no', title: '凭证号' },
        { key: 'voucher_date', title: '凭证日期' },
        { key: 'voucher_type', title: '凭证类型' },
        { key: 'total_debit', title: '借方金额' },
        { key: 'total_credit', title: '贷方金额' },
        {
          key: 'status',
          title: '状态',
          formatter: (value: unknown) => getVchrStatusLabel(value as Voucher['status']),
        },
        { key: 'created_by_name', title: '制单人' },
        { key: 'created_at', title: '创建时间' },
      ],
    })
  }

  /** 打印当前凭证列表 */
  const handlePrintVouchers = () => {
    const printWindow = window.open('', '_blank')
    if (!printWindow) {
      ElMessage.error('无法打开打印窗口')
      return
    }
    const rows = vouchers.value
      .map(
        item => `
      <tr>
        <td>${escapeHtml(item.voucher_no)}</td><td>${escapeHtml(item.voucher_date)}</td><td>${escapeHtml(item.voucher_type)}</td>
        <td style="text-align:right">${formatMoney(item.total_debit)}</td>
        <td style="text-align:right">${formatMoney(item.total_credit)}</td>
        <td>${escapeHtml(getVchrStatusLabel(item.status))}</td><td>${escapeHtml(item.created_by_name || '-')}</td>
      </tr>
    `
      )
      .join('')
    printWindow.document.write(`<html><head><meta charset="utf-8"><title>凭证列表</title>
      <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
      <h1>凭证列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${vouchers.value.length} 条</div>
      <table><thead><tr><th>凭证号</th><th>凭证日期</th><th>凭证类型</th><th>借方金额</th><th>贷方金额</th><th>状态</th><th>制单人</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
    printWindow.document.close()
    printWindow.onload = () => printWindow.print()
  }

  return {
    submitVoucher,
    reviewVoucher,
    postVoucher,
    handleExportVouchers,
    handlePrintVouchers,
  }
}

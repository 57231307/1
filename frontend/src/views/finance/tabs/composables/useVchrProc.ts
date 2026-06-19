/* eslint-disable @typescript-eslint/no-explicit-any */
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

  /** 导出 CSV */
  const handleExportVouchers = () => {
    const csvContent = [
      ['凭证号', '凭证日期', '凭证类型', '借方金额', '贷方金额', '状态', '制单人', '创建时间'],
      ...vouchers.value.map(item => [
        item.voucher_no,
        item.voucher_date,
        item.voucher_type,
        item.total_debit,
        item.total_credit,
        getVchrStatusLabel(item.status),
        item.created_by_name,
        item.created_at,
      ]),
    ]
      .map(row => row.map(cell => `"${cell ?? ''}"`).join(','))
      .join('\n')
    const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `凭证列表_${new Date().toISOString().split('T')[0]}.csv`
    link.click()
    ElMessage.success('导出成功')
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
        <td>${item.voucher_no}</td><td>${item.voucher_date}</td><td>${item.voucher_type}</td>
        <td style="text-align:right">${formatMoney(item.total_debit)}</td>
        <td style="text-align:right">${formatMoney(item.total_credit)}</td>
        <td>${getVchrStatusLabel(item.status)}</td><td>${item.created_by_name || '-'}</td>
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

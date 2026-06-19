/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useVchrLstProc.ts - 凭证列表流程操作 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 VoucherListTab.vue）
 * 封装凭证打印、导出、审核、记账、反记账、删除等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import printJS from 'print-js'
import {
  deleteVoucher,
  approveVoucher,
  postVoucher,
  unpostVoucher,
  type VoucherEntity,
} from '@/api/voucher'
import { getStatusLabel, getTypeLabel } from './vchrLstFmts'

/** 接收的列表数据（支持 ref 和 plain value） */
type ContractListLike = { value: VoucherEntity[] } | VoucherEntity[]

/**
 * 创建凭证流程操作方法集合
 * @param tableData 列表 ref 或 plain value
 * @param loadData 重新拉取列表方法
 */
export function useVchrLstProc(
  tableData: ContractListLike,
  loadData: () => Promise<void>
) {
  /** 取出底层数组（兼容 ref 和 plain value） */
  const getList = (): VoucherEntity[] => {
    return Array.isArray(tableData) ? tableData : tableData.value
  }

  /** 打印当前列表 */
  const handlePrint = () => {
    const list = getList()
    const printData = list.map((item, index) => ({
      序号: index + 1,
      凭证号: item.voucher_no,
      日期: item.voucher_date,
      类型: getTypeLabel(item.type),
      摘要: item.description || '-',
      借方金额: `¥${item.total_debit}`,
      贷方金额: `¥${item.total_credit}`,
      状态: getStatusLabel(item.status),
    }))
    printJS({
      printable: printData,
      properties: Object.keys(printData[0] || {}) as string[],
      type: 'table',
      header: '会计凭证列表',
      style: 'padding: 20px; font-size: 14px;',
      headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
      gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
      gridStyle: 'border-collapse: collapse; width: 100%;',
    } as never)
  }

  /** 导出 CSV */
  const handleExport = () => {
    const list = getList()
    const csvContent = [
      ['凭证号', '日期', '类型', '摘要', '借方金额', '贷方金额', '状态'],
      ...list.map(item => [
        item.voucher_no,
        item.voucher_date,
        getTypeLabel(item.type),
        item.description || '-',
        item.total_debit,
        item.total_credit,
        getStatusLabel(item.status),
      ]),
    ]
      .map(row => row.map(cell => `"${cell}"`).join(','))
      .join('\n')
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `会计凭证_${new Date().toISOString().split('T')[0]}.csv`
    link.click()
    ElMessage.success('导出成功')
  }

  /** 删除凭证 */
  const handleDelete = async (row: VoucherEntity) => {
    if (row.status === 'posted') {
      ElMessage.warning('已记账的凭证不能删除')
      return
    }
    try {
      await ElMessageBox.confirm('确定要删除这个凭证吗？', '提示', {
        type: 'warning',
      })
      await deleteVoucher(row.id!)
      ElMessage.success('删除成功')
      await loadData()
    } catch (error) {
      ElMessage.info('取消删除')
    }
  }

  /** 审核凭证 */
  const handleApprove = async (row: VoucherEntity) => {
    try {
      await ElMessageBox.confirm('确定要审核这个凭证吗？', '提示', {
        type: 'warning',
      })
      await approveVoucher(row.id!)
      ElMessage.success('审核成功')
      await loadData()
    } catch (error) {
      ElMessage.info('取消操作')
    }
  }

  /** 记账凭证 */
  const handlePost = async (row: VoucherEntity) => {
    try {
      await ElMessageBox.confirm('确定要记账这个凭证吗？', '提示', {
        type: 'warning',
      })
      await postVoucher(row.id!)
      ElMessage.success('记账成功')
      await loadData()
    } catch (error) {
      ElMessage.info('取消操作')
    }
  }

  /** 反记账 */
  const handleUnpost = async (row: VoucherEntity) => {
    try {
      await ElMessageBox.confirm('确定要反记账这个凭证吗？', '提示', {
        type: 'warning',
      })
      await unpostVoucher(row.id!)
      ElMessage.success('反记账成功')
      await loadData()
    } catch (error) {
      ElMessage.info('取消操作')
    }
  }

  return {
    handlePrint,
    handleExport,
    handleDelete,
    handleApprove,
    handlePost,
    handleUnpost,
  }
}

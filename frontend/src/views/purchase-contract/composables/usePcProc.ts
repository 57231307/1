/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * usePcProc.ts - 采购合同流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-contract/index.vue）
 * 封装采购合同提交审批/审批/执行/删除/导出等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  deletePurchaseContract,
  approvePurchaseContract,
  executePurchaseContract,
  // 批次 94 P2-12 修复：导入 exportPurchaseContracts 用于实现真实导出
  exportPurchaseContracts,
  type PurchaseContract,
} from '@/api/purchase-contract'
import { logger } from '@/utils/logger'

/** 刷新回调 */
interface RefreshCallbacks {
  getList: () => Promise<void>
}

/**
 * 采购合同流程操作方法集合
 */
export function usePcProc(refresh: RefreshCallbacks) {
  /** 提交审批 */
  const handleSubmit = async (row: PurchaseContract) => {
    try {
      await ElMessageBox.confirm('确认提交该合同审批？', '提示', { type: 'warning' })
      await approvePurchaseContract(row.id)
      ElMessage.success('提交成功')
      await refresh.getList()
    } catch (error) {
      logger.error('提交失败:', error)
    }
  }

  /** 审批 */
  const handleApprove = async (row: PurchaseContract) => {
    try {
      await ElMessageBox.confirm('确认审批通过该合同？', '提示', { type: 'warning' })
      await approvePurchaseContract(row.id)
      ElMessage.success('审批成功')
      await refresh.getList()
    } catch (error) {
      logger.error('审批失败:', error)
    }
  }

  /** 执行 */
  const handleExecute = async (row: PurchaseContract) => {
    try {
      await ElMessageBox.confirm('确认执行该合同？', '提示', { type: 'warning' })
      await executePurchaseContract(row.id)
      ElMessage.success('执行成功')
      await refresh.getList()
    } catch (error) {
      logger.error('执行失败:', error)
    }
  }

  /** 删除 */
  const handleDelete = async (row: PurchaseContract) => {
    try {
      await ElMessageBox.confirm('确认删除该合同？', '提示', { type: 'warning' })
      await deletePurchaseContract(row.id)
      ElMessage.success('删除成功')
      await refresh.getList()
    } catch (error) {
      logger.error('删除失败:', error)
    }
  }

  /** 导出（批次 94 P2-12 修复：原占位假成功，现接入真实导出 API 并触发浏览器下载） */
  const handleExport = async () => {
    try {
      const blob = await exportPurchaseContracts()
      const url = window.URL.createObjectURL(new Blob([blob]))
      const link = document.createElement('a')
      link.href = url
      link.setAttribute('download', `采购合同_${new Date().toISOString().split('T')[0]}.xlsx`)
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      window.URL.revokeObjectURL(url)
      ElMessage.success('导出成功')
    } catch (error) {
      logger.error('导出失败:', error)
      ElMessage.error('导出失败')
    }
  }

  return {
    handleSubmit,
    handleApprove,
    handleExecute,
    handleDelete,
    handleExport,
  }
}

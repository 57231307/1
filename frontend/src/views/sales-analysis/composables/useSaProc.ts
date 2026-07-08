// sales-analysis 业务流程 composable
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：销售分析（编辑目标 + 导出报表 + 排名类型切换）
// 行为完全保持一致（仅结构重构）
import { ElMessage, ElMessageBox } from 'element-plus'
import { salesAnalysisApi } from '@/api/sales-analysis'
import { logger } from '@/utils/logger'
import { useSa } from './useSa'

/** 销售分析业务流程 composable（编辑目标 + 导出报表 + 排名类型切换） */
export const useSaProc = () => {
  // 编辑目标（批次 95 P3-18 修复：拉取目标列表 + prompt 输入新金额 + 调用 updateSalesTarget）
  const handleEditTarget = async () => {
    try {
      const res = await salesAnalysisApi.getSalesTargets()
      const targets = res.data || []
      if (!targets.length) {
        ElMessage.warning('暂无销售目标可编辑')
        return
      }
      // 取最近一个周期目标进行编辑（多目标场景可后续扩展为选择式对话框）
      const target = targets[0]
      const { value } = await ElMessageBox.prompt(
        `请输入周期「${target.period}」的新目标金额（当前：¥${target.target_amount}）`,
        '编辑销售目标',
        {
          inputValue: String(target.target_amount),
          inputValidator: (v: string) =>
            (!isNaN(Number(v)) && Number(v) >= 0) || '请输入有效的非负数字',
        }
      )
      await salesAnalysisApi.updateSalesTarget(target.period, {
        target_amount: Number(value),
      })
      ElMessage.success('更新成功')
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('编辑目标失败:', error)
        ElMessage.error('编辑目标失败')
      }
    }
  }

  // 导出报表
  const handleExport = async () => {
    try {
      const res = await salesAnalysisApi.exportReport()
      const url = window.URL.createObjectURL(new Blob([res]))
      const link = document.createElement('a')
      link.href = url
      link.setAttribute('download', '销售分析报表.xlsx')
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      window.URL.revokeObjectURL(url)
      ElMessage.success('导出成功')
    } catch (error) {
      logger.error('导出失败:', error)
    }
  }

  // 产品排名类型切换：更新 productRankType + 重新拉取数据
  const handleProductRankTypeChange = (v: string, sa: ReturnType<typeof useSa>) => {
    sa.productRankType = v
    sa.getProductRanking()
  }

  // 客户排名类型切换：更新 customerRankType + 重新拉取数据
  const handleCustomerRankTypeChange = (v: string, sa: ReturnType<typeof useSa>) => {
    sa.customerRankType = v
    sa.getCustomerRanking()
  }

  return {
    handleEditTarget,
    handleExport,
    handleProductRankTypeChange,
    handleCustomerRankTypeChange,
  }
}

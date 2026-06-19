// sales-analysis 业务流程 composable
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：销售分析（编辑目标 + 导出报表 + 排名类型切换）
// 行为完全保持一致（仅结构重构）
import { ElMessage } from 'element-plus'
import { salesAnalysisApi } from '@/api/sales-analysis'
import { logger } from '@/utils/logger'

/** 编辑目标（占位提示） */
export const useSaProc = () => {
  // 编辑目标
  const handleEditTarget = () => {
    ElMessage.info('编辑目标功能开发中')
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
  const handleProductRankTypeChange = (v: string, sa: any) => {
    sa.productRankType = v
    sa.getProductRanking()
  }

  // 客户排名类型切换：更新 customerRankType + 重新拉取数据
  const handleCustomerRankTypeChange = (v: string, sa: any) => {
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

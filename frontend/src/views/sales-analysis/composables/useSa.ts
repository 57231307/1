// sales-analysis 主业务 composable
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：销售分析（stats + 4 个排行榜 + 趋势周期 + 排名类型 + 销售目标）
// 行为完全保持一致（仅结构重构）
import { reactive, ref, watch } from 'vue'
import {
  getSalesAnalysisStats,
  getProductRanking as fetchProductRanking,
  getCustomerRanking as fetchCustomerRanking,
  getSalesTargetList as fetchSalesTargetList,
  getSalesTrendData as fetchSalesTrendData,
  type ProductRanking,
  type CustomerRanking,
  type SalesTarget,
  type SalesTrendResult,
} from '@/api/sales-analysis'
import { logger } from '@/utils/logger'

/** 销售分析主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useSa = () => {
  // 统计数据
  const stats = reactive({
    monthOrders: 0,
    monthAmount: 0,
    grossProfitRate: 0,
    activeCustomers: 0,
    orderTrend: 0,
    amountTrend: 0,
    profitTrend: 0,
    customerTrend: 0,
  })

  // 趋势周期
  const trendPeriod = ref('month')

  // 排名类型
  const productRankType = ref('amount')
  const customerRankType = ref('amount')

  // 产品排名
  const productRanking = ref<ProductRanking[]>([])

  // 客户排名
  const customerRanking = ref<CustomerRanking[]>([])

  // 销售目标
  const salesTargets = ref<SalesTarget[]>([])

  // 销售趋势数据（批次 95 P3-20 修复：供 SaTrend 折线图渲染）
  const trendData = ref<SalesTrendResult[]>([])

  // 获取统计数据
  const getStats = async () => {
    try {
      const res = await getSalesAnalysisStats()
      if (res.data) {
        Object.assign(stats, res.data)
      }
    } catch (error) {
      logger.error('获取统计数据失败:', error)
    }
  }

  // 获取产品排名
  const getProductRanking = async () => {
    try {
      const res = await fetchProductRanking({ type: productRankType.value })
      productRanking.value = res.data || []
    } catch (error) {
      logger.error('获取产品排名失败:', error)
    }
  }

  // 获取客户排名
  const getCustomerRanking = async () => {
    try {
      const res = await fetchCustomerRanking({ type: customerRankType.value })
      customerRanking.value = res.data || []
    } catch (error) {
      logger.error('获取客户排名失败:', error)
    }
  }

  // 获取销售目标
  const getSalesTargets = async () => {
    try {
      const res = await fetchSalesTargetList()
      salesTargets.value = res.data || []
    } catch (error) {
      logger.error('获取销售目标失败:', error)
    }
  }

  // 获取销售趋势数据（批次 95 P3-20 修复：按当前趋势周期拉取）
  const getTrendData = async () => {
    try {
      const res = await fetchSalesTrendData({ period: trendPeriod.value })
      trendData.value = res.data || []
    } catch (error) {
      logger.error('获取销售趋势数据失败:', error)
    }
  }

  // 趋势周期变化时重新拉取趋势数据（批次 95 P3-20 修复）
  watch(trendPeriod, () => {
    getTrendData()
  })

  return reactive({
    stats,
    trendPeriod,
    productRankType,
    customerRankType,
    productRanking,
    customerRanking,
    salesTargets,
    trendData,
    getStats,
    getProductRanking,
    getCustomerRanking,
    getSalesTargets,
    getTrendData,
  })
}

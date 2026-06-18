/**
 * useAi - AI 智能分析 tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 1 个 tab）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import {
  forecastSales,
  optimizeInventory,
  detectAnomalies,
  getRecommendations as getRecommendationsApi,
} from '@/api/advanced'

/**
 * AI 智能分析 tab 业务逻辑封装
 * 包含销售预测、库存优化、异常检测、智能推荐四个子模块
 */
export function useAi() {
  // 销售预测状态
  const forecastPeriod = ref('3m')
  const forecastLoading = ref(false)
  const forecastResult = ref<any>(null)

  // 库存优化状态
  const inventoryLoading = ref(false)
  const inventoryResult = ref<any>(null)

  // 异常检测状态
  const anomalyType = ref('sales')
  const anomalyLoading = ref(false)
  const anomalyResult = ref<any>(null)

  // 智能推荐状态
  const recommendLoading = ref(false)
  const recommendationResult = ref<any>(null)

  /**
   * 格式化金额为人民币字符串
   */
  const formatMoney = (amount: number) =>
    '¥' + (amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00')

  /**
   * 执行销售预测
   */
  const runSalesForecast = async () => {
    forecastLoading.value = true
    try {
      const res: any = await forecastSales({ period: forecastPeriod.value })
      forecastResult.value = res.data!
      ElMessage.success('预测完成')
    } catch (e: any) {
      ElMessage.error(e.message || '预测失败')
    } finally {
      forecastLoading.value = false
    }
  }

  /**
   * 执行库存优化
   */
  const runInventoryOptimization = async () => {
    inventoryLoading.value = true
    try {
      const res: any = await optimizeInventory()
      inventoryResult.value = res.data!
      ElMessage.success('优化建议生成完成')
    } catch (e: any) {
      ElMessage.error(e.message || '生成失败')
    } finally {
      inventoryLoading.value = false
    }
  }

  /**
   * 执行异常检测
   */
  const runAnomalyDetection = async () => {
    anomalyLoading.value = true
    try {
      const res: any = await detectAnomalies({ data_type: anomalyType.value })
      anomalyResult.value = res.data!
      ElMessage.success('检测完成')
    } catch (e: any) {
      ElMessage.error(e.message || '检测失败')
    } finally {
      anomalyLoading.value = false
    }
  }

  /**
   * 获取智能推荐
   */
  const getRecommendations = async () => {
    recommendLoading.value = true
    try {
      const res: any = await getRecommendationsApi()
      recommendationResult.value = res.data!
      ElMessage.success('推荐获取完成')
    } catch (e: any) {
      ElMessage.error(e.message || '获取失败')
    } finally {
      recommendLoading.value = false
    }
  }

  return {
    // 销售预测
    forecastPeriod,
    forecastLoading,
    forecastResult,
    runSalesForecast,
    // 库存优化
    inventoryLoading,
    inventoryResult,
    runInventoryOptimization,
    // 异常检测
    anomalyType,
    anomalyLoading,
    anomalyResult,
    runAnomalyDetection,
    // 智能推荐
    recommendLoading,
    recommendationResult,
    getRecommendations,
    // 工具函数
    formatMoney,
  }
}

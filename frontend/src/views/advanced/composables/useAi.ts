/**
 * useAi - AI 智能分析 tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 1 个 tab）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { ApiResponse } from '@/types/api'
import {
  forecastSales,
  optimizeInventory,
  detectAnomalies,
  getRecommendations as getRecommendationsApi,
} from '@/api/advanced'

// v11 批次 171 P2-1 修复：定义 AI 分析结果接口类型，替代 any
export interface ForecastResult {
  sales_amount: number
  order_count: number
  confidence: number
  trend: string
}

export interface InventoryOptimizationItem {
  product_name: string
  suggestion: string
  priority: string
}

export interface InventoryResult {
  summary: string
  items: InventoryOptimizationItem[]
}

export interface AnomalyItem {
  item: string
  type: string
  description: string
  severity: string
}

export interface RecommendationItem {
  type: string
  created_at: string
  content: string
}

/**
 * AI 智能分析 tab 业务逻辑封装
 * 包含销售预测、库存优化、异常检测、智能推荐四个子模块
 */
export function useAi() {
  // 销售预测状态
  const forecastPeriod = ref('3m')
  const forecastLoading = ref(false)
  // v11 批次 171 P2-1 修复：ref<any> 改为具体接口类型
  const forecastResult = ref<ForecastResult | null>(null)

  // 库存优化状态
  const inventoryLoading = ref(false)
  const inventoryResult = ref<InventoryResult | null>(null)

  // 异常检测状态
  const anomalyType = ref('sales')
  const anomalyLoading = ref(false)
  const anomalyResult = ref<AnomalyItem[] | null>(null)

  // 智能推荐状态
  const recommendLoading = ref(false)
  const recommendationResult = ref<RecommendationItem[] | null>(null)

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
      // v11 批次 171 P2-1 修复：res: any 改为 ApiResponse<ForecastResult>
      const res = (await forecastSales({ period: forecastPeriod.value })) as ApiResponse<ForecastResult>
      forecastResult.value = res.data
      ElMessage.success('预测完成')
    } catch (e: unknown) {
      // v11 批次 171 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error((e instanceof Error ? e.message : String(e)) || '预测失败')
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
      const res = (await optimizeInventory()) as ApiResponse<InventoryResult>
      inventoryResult.value = res.data
      ElMessage.success('优化建议生成完成')
    } catch (e: unknown) {
      ElMessage.error((e instanceof Error ? e.message : String(e)) || '生成失败')
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
      const res = (await detectAnomalies({ data_type: anomalyType.value })) as ApiResponse<AnomalyItem[]>
      anomalyResult.value = res.data
      ElMessage.success('检测完成')
    } catch (e: unknown) {
      ElMessage.error((e instanceof Error ? e.message : String(e)) || '检测失败')
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
      const res = (await getRecommendationsApi()) as ApiResponse<RecommendationItem[]>
      recommendationResult.value = res.data
      ElMessage.success('推荐获取完成')
    } catch (e: unknown) {
      ElMessage.error((e instanceof Error ? e.message : String(e)) || '获取失败')
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

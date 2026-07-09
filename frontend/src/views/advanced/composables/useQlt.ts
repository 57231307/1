/**
 * useQlt - 质量预测 tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 5 个 tab）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { predictQuality, type QualityPredParams } from '@/api/advanced'
import type { ApiResponse } from '@/types/api'

/**
 * 质量预测表单数据结构
 */
export interface QualityFormData {
  product_id: number | null
  inspection_type: string
  window_days: number
}

/** 质量预测结果（与后端响应结构对齐） */
export interface QualityResult {
  total_inspections: number
  avg_qualification_rate: number
  trend: string
  trend_rate: number
  risk_level: string
  risk_score: number
  confidence: number
  top_issues: Array<{
    issue_type: string
    occurrences: number
    percentage: number
  }>
  recommendations: string[]
  period_breakdown: Array<{
    period: string
    inspections: number
    avg_qualification_rate: number
  }>
  source: string
}

/**
 * 质量预测 tab 业务逻辑封装
 * 质量预测（A2-2）
 */
export function useQlt() {
  const qualityForm = ref<QualityFormData>({
    product_id: null,
    inspection_type: '',
    window_days: 90,
  })
  const qualityLoading = ref(false)
  const qualityResult = ref<QualityResult | null>(null)

  /**
   * 执行质量预测
   */
  const runQualityPrediction = async () => {
    qualityLoading.value = true
    try {
      const payload: QualityPredParams = {
        window_days: qualityForm.value.window_days,
      }
      if (qualityForm.value.product_id !== null && qualityForm.value.product_id !== undefined) {
        payload.product_id = qualityForm.value.product_id
      }
      if (qualityForm.value.inspection_type && qualityForm.value.inspection_type.trim()) {
        payload.inspection_type = qualityForm.value.inspection_type.trim()
      }
      const res = await predictQuality(payload) as ApiResponse<QualityResult>
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) qualityResult.value = res.data
      ElMessage.success('预测完成')
    } catch (e: unknown) {
      ElMessage.error((e instanceof Error ? e.message : '') || '预测失败')
    } finally {
      qualityLoading.value = false
    }
  }

  return {
    qualityForm,
    qualityLoading,
    qualityResult,
    runQualityPrediction,
  }
}

// P2-4 AI 分析深化 API 客户端（16 端点）
// 工艺优化（7）+ 质量预测（7）+ 看板 / 健康（2）
// 创建时间: 2026-06-17

import { request } from './request'

// =====================================================
// 公共类型
// =====================================================

export interface ProcessOptRequest {
  color_no: string
  color_name?: string
  fabric_type: string
  dye_type?: string
  k?: number
}

export interface ProcessOptParams {
  temperature: number
  time_minutes: number
  ph_value: number
  liquor_ratio: number
}

export interface ProcessOptCandidate {
  case_id: number
  color_no: string
  fabric_type: string
  similarity: number
  temperature: number
  time_minutes: number
  ph_value: number
  liquor_ratio: number
}

export interface ProcessOptResponse {
  recommended_params: ProcessOptParams
  confidence: number
  source: 'knn' | 'fallback'
  similar_cases: number
  reason: string
  candidates: ProcessOptCandidate[]
}

export interface QualityPredRequest {
  product_id?: number
  inspection_type?: 'all' | 'incoming' | 'inprocess' | 'final' | 'outgoing'
  window_days?: number
}

export interface QualityTopIssue {
  issue: string
  count: number
  percentage: number
}

export interface QualityPeriodItem {
  period: string
  inspections: number
  avg_qualification_rate: number
}

export interface QualityPredResponse {
  product_id: number | null
  inspection_type: string
  window_days: number
  total_inspections: number
  avg_qualification_rate: number
  trend: '上升' | '平稳' | '下降' | '无数据'
  trend_rate: number
  risk_score: number
  risk_level: '低' | '中' | '高'
  confidence: number
  top_issues: QualityTopIssue[]
  recommendations: string[]
  period_breakdown: QualityPeriodItem[]
  source: 'history' | 'fallback'
}

export interface AiProcessOptimization {
  id: number
  request_id: string
  color_no: string
  color_name: string | null
  fabric_type: string
  dye_type: string | null
  recommended_temperature: string
  recommended_time_minutes: number
  recommended_ph_value: string
  recommended_liquor_ratio: string
  similar_cases: number
  confidence: string
  source: string
  reason: string | null
  candidates_json: unknown
  is_applied: boolean
  applied_at: string | null
  applied_by: number | null
  feedback_score: number | null
  feedback_remark: string | null
  tenant_id: number
  created_by: number | null
  created_at: string
  updated_at: string
}

export interface AiQualityPrediction {
  id: number
  request_id: string
  product_id: number | null
  inspection_type: string
  window_days: number
  total_inspections: number
  avg_qualification_rate: string
  trend: 'up' | 'flat' | 'down' | 'nodata'
  trend_rate: string
  risk_score: number
  risk_level: 'low' | 'medium' | 'high'
  confidence: string
  top_issues_json: unknown
  recommendations_json: unknown
  period_breakdown_json: unknown
  source: string
  is_acknowledged: boolean
  acknowledged_at: string | null
  acknowledged_by: number | null
  tenant_id: number
  created_by: number | null
  created_at: string
  updated_at: string
}

export interface AiSummary {
  process_optimization: {
    total: number
    applied: number
    knn_recommended: number
    apply_rate: number
  }
  quality_prediction: {
    total: number
    high_risk: number
    unacknowledged: number
  }
  latest_process_optimizations: AiProcessOptimization[]
  latest_quality_predictions: AiQualityPrediction[]
}

export interface PageResult<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

// =====================================================
// 工艺优化（7 端点）
// =====================================================

/** 触发工艺优化（算法 + 落库） */
export function createProcessOptimization(request_body: ProcessOptRequest) {
  return request.post<{ id: number; response: ProcessOptResponse }>('/ai/process-optimizations', {
    request: request_body,
  })
}

/** 工艺优化列表 */
export function listProcessOptimizations(params: {
  page?: number
  page_size?: number
  color_no?: string
  fabric_type?: string
  is_applied?: boolean
  source?: string
} = {}) {
  return request.get<PageResult<AiProcessOptimization>>('/ai/process-optimizations', { params })
}

/** 工艺优化详情 */
export function getProcessOptimization(id: number) {
  return request.get<AiProcessOptimization>(`/ai/process-optimizations/${id}`)
}

/** 应用工艺优化 + 反馈打分 */
export function applyProcessOptimization(
  id: number,
  data: { feedback_score?: number; feedback_remark?: string },
) {
  return request.post<AiProcessOptimization>(`/ai/process-optimizations/${id}/apply`, data)
}

/** 删除工艺优化记录 */
export function deleteProcessOptimization(id: number) {
  return request.delete<{ deleted: boolean; id: number }>(`/ai/process-optimizations/${id}`)
}

/** 按色号 + 布类查询历史 */
export function listProcessOptimizationsByColor(params: {
  color_no: string
  fabric_type: string
  limit?: number
}) {
  return request.get<{ items: AiProcessOptimization[] }>('/ai/process-optimizations/by-color', {
    params,
  })
}

/** 批量工艺优化（最多 20 条） */
export function batchCreateProcessOptimizations(requests: ProcessOptRequest[]) {
  return request.post<{
    total: number
    succeeded: number
    failed: number
    results: unknown[]
  }>('/ai/process-optimizations/batch', { requests: requests.map((r) => ({ request: r })) })
}

// =====================================================
// 质量预测（7 端点）
// =====================================================

/** 触发质量预测（算法 + 落库） */
export function createQualityPrediction(request_body: QualityPredRequest) {
  return request.post<{ id: number; response: QualityPredResponse }>('/ai/quality-predictions', {
    request: request_body,
  })
}

/** 质量预测列表 */
export function listQualityPredictions(params: {
  page?: number
  page_size?: number
  product_id?: number
  inspection_type?: string
  risk_level?: string
  is_acknowledged?: boolean
} = {}) {
  return request.get<PageResult<AiQualityPrediction>>('/ai/quality-predictions', { params })
}

/** 质量预测详情 */
export function getQualityPrediction(id: number) {
  return request.get<AiQualityPrediction>(`/ai/quality-predictions/${id}`)
}

/** 确认质量预测 */
export function acknowledgeQualityPrediction(id: number) {
  return request.post<AiQualityPrediction>(`/ai/quality-predictions/${id}/acknowledge`, {})
}

/** 删除质量预测记录 */
export function deleteQualityPrediction(id: number) {
  return request.delete<{ deleted: boolean; id: number }>(`/ai/quality-predictions/${id}`)
}

/** 按产品查询历史 */
export function listQualityPredictionsByProduct(params: { product_id: number; limit?: number }) {
  return request.get<{ items: AiQualityPrediction[] }>('/ai/quality-predictions/by-product', {
    params,
  })
}

/** 批量质量预测（最多 20 条） */
export function batchCreateQualityPredictions(requests: QualityPredRequest[]) {
  return request.post<{
    total: number
    succeeded: number
    failed: number
    results: unknown[]
  }>('/ai/quality-predictions/batch', { requests: requests.map((r) => ({ request: r })) })
}

// =====================================================
// 看板 / 健康检查（2 端点）
// =====================================================

/** AI 概览 */
export function getAiSummary() {
  return request.get<AiSummary>('/ai/summary')
}

/** AI 服务健康检查 */
export function getAiHealth() {
  return request.get<{
    status: string
    version: string
    modules: Record<string, { algorithm: string; fallback: string }>
  }>('/ai/health')
}

// =====================================================
// 翻译字典
// =====================================================

export const RISK_LEVEL_LABELS: Record<string, string> = {
  low: '低风险',
  medium: '中风险',
  high: '高风险',
}

export const RISK_LEVEL_COLORS: Record<string, string> = {
  low: '#67c23a',
  medium: '#e6a23c',
  high: '#f56c6c',
}

export const TREND_LABELS: Record<string, string> = {
  up: '上升',
  flat: '平稳',
  down: '下降',
  nodata: '无数据',
}

export const TREND_ICONS: Record<string, string> = {
  up: 'CaretTop',
  flat: 'Minus',
  down: 'CaretBottom',
  nodata: 'QuestionFilled',
}

export const SOURCE_LABELS: Record<string, string> = {
  knn: 'k-NN 加权',
  fallback: '典型参数表',
  history: '历史趋势',
}

export const INSPECTION_TYPE_LABELS: Record<string, string> = {
  all: '全部',
  incoming: '来料',
  inprocess: '过程',
  final: '成品',
  outgoing: '出货',
}

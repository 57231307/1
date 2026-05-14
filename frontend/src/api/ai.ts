import { request } from './request'
import type { ApiResponse, PageResult } from './request'

export interface SalesTrendAnalysis {
  period: string
  actual_value: number
  predicted_value: number
  trend_direction: 'up' | 'down' | 'stable'
}

export interface InventoryPrediction {
  product_id: number
  product_name: string
  current_stock: number
  predicted_stock: number
  recommended_action: string
}

export interface AnomalyDetectionResult {
  type: string
  severity: 'low' | 'medium' | 'high' | 'critical'
  message: string
  suggestion: string
}

export interface AIRecommendation {
  id: number
  type: string
  content: string
  confidence: number
  suggested_action: string
}

export interface AnalysisHistoryItem {
  id: number
  analysis_type: string
  created_at: string
  status: string
  summary: string
}

export interface AnalyzeSalesTrendParams {
  start_date: string
  end_date: string
  product_id?: number
  category_id?: number
}

export interface PredictInventoryParams {
  warehouse_id?: number
  product_id?: number
  days_ahead?: number
}

export interface DetectAnomaliesParams {
  start_date: string
  end_date: string
  data_type?: 'sales' | 'inventory' | 'finance' | 'all'
}

export interface GenerateReportParams {
  report_type: string
  start_date: string
  end_date: string
  format?: 'pdf' | 'excel' | 'json'
}

export const aiApi = {
  analyzeSalesTrend: (params: AnalyzeSalesTrendParams) =>
    request.post<ApiResponse<SalesTrendAnalysis[]>>(
      '/ai/analysis/sales-trend',
      params
    ),

  predictInventory: (params: PredictInventoryParams) =>
    request.post<ApiResponse<InventoryPrediction[]>>(
      '/ai/analysis/inventory-prediction',
      params
    ),

  detectAnomalies: (params: DetectAnomaliesParams) =>
    request.post<ApiResponse<AnomalyDetectionResult[]>>(
      '/ai/analysis/anomaly-detection',
      params
    ),

  generateReport: (params: GenerateReportParams) =>
    request.post<ApiResponse<{ report_url: string; report_id: number }>>(
      '/ai/generate/report',
      params
    ),

  getAnalysisHistory: (params?: { page?: number; page_size?: number }) =>
    request.get<ApiResponse<PageResult<AnalysisHistoryItem>>>(
      '/ai/analysis/history',
      { params }
    ),

  getRecommendations: (params?: { type?: string; limit?: number }) =>
    request.get<ApiResponse<AIRecommendation[]>>('/ai/recommendations', {
      params,
    }),
}

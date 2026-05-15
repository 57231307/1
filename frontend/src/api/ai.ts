import { request } from './request'
import type { ApiResponse } from './request'

export interface AIAnalysisRequest {
  analysisType: string
  data: Record<string, any>
  parameters?: Record<string, any>
}

export interface AIAnalysisResult {
  analysisType: string
  result: Record<string, any>
  insights?: string[]
  recommendations?: string[]
  confidence?: number
  generatedAt?: string
}

export interface AIForecastRequest {
  forecastType: string
  targetMetric: string
  historicalData: Array<{ date: string; value: number }>
  periods?: number
}

export interface AIForecastResult {
  forecastType: string
  predictions: Array<{ date: string; value: number; confidence?: number }>
  trend?: 'up' | 'down' | 'stable'
  insights?: string
}

export const aiApi = {
  analyze: (data: AIAnalysisRequest) =>
    request.post<ApiResponse<AIAnalysisResult>>('/ai/analyze', data),

  forecast: (data: AIForecastRequest) =>
    request.post<ApiResponse<AIForecastResult>>('/ai/forecast', data),

  getInsights: (businessType: string, businessId: number) =>
    request.get<ApiResponse<{ insights: string[] }>>('/ai/insights', {
      params: { businessType, businessId }
    }),

  generateReport: (reportType: string, parameters?: Record<string, any>) =>
    request.post<ApiResponse<{ reportUrl: string }>>('/ai/generate-report', { reportType, ...parameters }),
}

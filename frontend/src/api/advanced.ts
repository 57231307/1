import { request } from './request'

// Advanced AI 分析 API
export const forecastSales = (data: { period: string; product_id?: number }) => 
  request.post('/advanced/ai/sales-forecast', data)

export const optimizeInventory = (data?: { warehouse_id?: number }) => 
  request.post('/advanced/ai/inventory-optimization', data)

export const detectAnomalies = (data: { data_type: string; date_range?: any }) => 
  request.post('/advanced/ai/anomaly-detection', data)

export const getRecommendations = (data?: { type?: string }) => 
  request.post('/advanced/ai/recommendations', data)

export const listReportTemplates = () => 
  request.get('/advanced/reports/templates')

export const executeReport = (template_code: string) => 
  request.post('/advanced/reports/execute', { template_code })

export const listTenants = () => 
  request.get('/advanced/tenants')

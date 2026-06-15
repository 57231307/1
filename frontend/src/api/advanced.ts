import { request } from './request'

// Advanced AI 分析 API
export const forecastSales = (data: { period: string; product_id?: number }) =>
  request.post('/advanced/ai/sales-forecast', data)

export const optimizeInventory = (data?: { warehouse_id?: number }) =>
  request.post('/advanced/ai/inventory-optimization', data)

export const detectAnomalies = (data: {
  data_type: string
  date_range?: Record<string, unknown>
}) => request.post('/advanced/ai/anomaly-detection', data)

export const getRecommendations = (data?: { type?: string }) =>
  request.post('/advanced/ai/recommendations', data)

// 染色工艺参数智能推荐（A2-1 工艺优化）
export interface RecipeOptParams {
  color_no: string
  fabric_type: string
  dye_type?: string
  color_name?: string
  k?: number
}

export const optimizeRecipe = (data: RecipeOptParams) =>
  request.post('/advanced/ai/recipe-optimization', data)

export const listReportTemplates = () => request.get('/advanced/reports/templates')

export const executeReport = (template_code: string) =>
  request.post('/advanced/reports/execute', { template_code })

export const listTenants = () => request.get('/advanced/tenants')

export const createTenant = (data: any) => request.post('/advanced/tenants', data)

export const updateTenant = (id: number, data: any) => request.put(`/advanced/tenants/${id}`, data)

export const deleteTenant = (id: number) => request.delete(`/advanced/tenants/${id}`)

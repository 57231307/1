import { request } from './request';

// Advanced AI 分析 API
export const forecastSales = (data: { period: string; product_id?: number }) =>
  request.post('/advanced/ai/sales-forecast', data);

export const optimizeInventory = (data?: { warehouse_id?: number }) =>
  request.post('/advanced/ai/inventory-optimization', data);

export const detectAnomalies = (data: {
  data_type: string;
  date_range?: Record<string, unknown>;
}) => request.post('/advanced/ai/anomaly-detection', data);

// 键名逐字对齐后端 RecommendationRequest（handlers/advanced/rec.rs：recommendation_type/limit，
// 均 Option → 可选；原键 type 后端不存在，被 serde 静默丢弃 → 过滤恒不生效）
export const getRecommendations = (data?: { recommendation_type?: string; limit?: number }) =>
  request.post('/advanced/ai/recommendations', data);

// 染色工艺参数智能推荐（A2-1 工艺优化）
export interface RecipeOptParams {
  color_no: string;
  fabric_type: string;
  dye_type?: string;
  color_name?: string;
  k?: number;
}

export const optimizeRecipe = (data: RecipeOptParams) =>
  request.post('/advanced/ai/recipe-optimization', data);

// 质量预测（A2-2 质量预测）
export interface QualityPredParams {
  product_id?: number;
  inspection_type?: string;
  window_days?: number;
}

export const predictQuality = (data: QualityPredParams) =>
  request.post('/advanced/ai/quality-prediction', data);

export const getReportTemplateList = () => request.get('/advanced/reports/templates');

export const executeReport = (template_code: string) =>
  request.post('/advanced/reports/execute', { template_code });

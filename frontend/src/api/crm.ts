import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

export interface Lead {
  id: number;
  lead_no: string;
  // 后端 crm_lead 契约字段（CreateLeadRequest / crm_lead.rs Model）
  lead_source?: string;
  lead_status?: string;
  company_name?: string;
  contact_name?: string;
  contact_title?: string;
  mobile_phone?: string;
  tel_phone?: string;
  wechat?: string;
  qq?: string;
  owner_id?: number;
  owner_name?: string;
  priority?: string;
  requirement_desc?: string;
  remarks?: string;
  // 兼容历史前端字段
  name: string;
  phone: string;
  email: string;
  company: string;
  source: string;
  status: 'new' | 'contacted' | 'qualified' | 'proposal' | 'converted' | 'lost';
  rating: number;
  address: string;
  description: string;
  created_by: number;
  created_by_name: string;
  assigned_to: number;
  assigned_to_name: string;
  created_at: string;
  updated_at: string;
}

/** 线索导入结果（v11 批次 157d-4） */
export interface ImportLeadError {
  row: number;
  message: string;
}

export interface ImportLeadsResult {
  total: number;
  success_count: number;
  failed_count: number;
  errors: ImportLeadError[];
}

export interface Opportunity {
  id: number;
  opportunity_no: string;
  // 后端 CreateOpportunityRequest / crm_opportunity 契约字段
  opportunity_name?: string;
  opportunity_type?: string;
  win_probability?: number;
  owner_id?: number;
  product_desc?: string;
  remarks?: string;
  name: string;
  customer_id: number;
  customer_name: string;
  /** 商机阶段（后端字段名，大写值：QUALIFICATION/NEEDS_ANALYSIS/PROPOSAL/NEGOTIATION/CLOSED_WON/CLOSED_LOST） */
  opportunity_stage?:
    'QUALIFICATION' | 'NEEDS_ANALYSIS' | 'PROPOSAL' | 'NEGOTIATION' | 'CLOSED_WON' | 'CLOSED_LOST';
  /** @deprecated 向后兼容字段，新代码应使用 opportunity_stage */
  stage?:
    | 'qualification'
    | 'needs_analysis'
    | 'value_proposition'
    | 'proposal'
    | 'negotiation'
    | 'closed_won'
    | 'closed_lost';
  estimated_amount: number;
  probability: number;
  expected_close_date: string;
  description: string;
  created_by: number;
  created_by_name: string;
  created_at: string;
  updated_at: string;
}

// 后端 crm_service::list_leads 构造 json!({ "data": [...], "total", "page", "page_size" })，
// 经 crm_handler::list_leads 原样返回，真实列表键为 data（非裸数组）。
export function getLeadList(
  params?: QueryParams
): Promise<ApiResponse<{ data: Lead[]; total: number; page: number; page_size: number }>> {
  return request.get('/crm/leads', { params });
}

export function getLead(id: number): Promise<ApiResponse<Lead>> {
  return request.get(`/crm/leads/${id}`);
}

export function createLead(data: Partial<Lead>): Promise<ApiResponse<Lead>> {
  return request.post('/crm/leads', data);
}

export function updateLead(id: number, data: Partial<Lead>): Promise<ApiResponse<Lead>> {
  return request.put(`/crm/leads/${id}`, data);
}

export function deleteLead(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/crm/leads/${id}`);
}

export function updateLeadStatus(
  id: number,
  data: { status: Lead['status'] }
): Promise<ApiResponse<void>> {
  return request.put(`/crm/leads/${id}/status`, data);
}

export function convertLead(
  id: number
): Promise<ApiResponse<{ customer_id: number; opportunity_id: number }>> {
  return request.post(`/crm/leads/${id}/convert`);
}

// 后端 crm_service::list_opportunities 构造 json!({ "data": [...], "total", "page", "page_size" })，
// 真实列表键为 data（非裸数组）。
export function getOpportunityList(
  params?: QueryParams
): Promise<ApiResponse<{ data: Opportunity[]; total: number; page: number; page_size: number }>> {
  return request.get('/crm/opportunities', { params });
}

export function getOpportunity(id: number): Promise<ApiResponse<Opportunity>> {
  return request.get(`/crm/opportunities/${id}`);
}

export function createOpportunity(data: Partial<Opportunity>): Promise<ApiResponse<Opportunity>> {
  return request.post('/crm/opportunities', data);
}

export function updateOpportunity(
  id: number,
  data: Partial<Opportunity>
): Promise<ApiResponse<Opportunity>> {
  return request.put(`/crm/opportunities/${id}`, data);
}

export function deleteOpportunity(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/crm/opportunities/${id}`);
}

export interface CustomerSummary {
  customer_id: number;
  customer_name: string;
  total_orders: number;
  total_amount: number;
  last_order_date?: string;
  credit_limit?: number;
  credit_used?: number;
}

export function getCustomerSummary(customerId: number): Promise<ApiResponse<CustomerSummary>> {
  return request.get(`/crm/customers/${customerId}/summary`);
}

// 批次 94 P2-12 修复：补全 CRM 线索导出接口（原缺失，导致 leads/index.vue 导出占位假成功）
// 返回 blob，前端用 URL.createObjectURL 触发下载
export function exportLeads(params?: QueryParams): Promise<Blob> {
  return request.get('/crm/leads/export', {
    params,
    responseType: 'blob',
  });
}

// v11 批次 157d-4 新增：批量导入线索（xlsx），用 FormData 上传文件
export function importLeads(file: File): Promise<ApiResponse<ImportLeadsResult>> {
  const formData = new FormData();
  formData.append('file', file);
  return request.post('/crm/leads/import', formData);
}

// v11 批次 141 修复：补全 CRM 商机导出接口（原缺失，导致 opportunities/index.vue 导出假成功）
// 返回 blob，前端用 URL.createObjectURL 触发下载
export function exportOpportunities(params?: QueryParams): Promise<Blob> {
  return request.get('/crm/opportunities/export', {
    params,
    responseType: 'blob',
  });
}

// ============== V15 P1/P2 商机分析与线索增强（Batch 补齐 API 封装）==============

/** 销售漏斗报告（对应后端 services/crm/opp.rs::SalesFunnelReport） */
export interface SalesFunnelReport {
  lead_count: number;
  opportunity_count: number;
  opportunity_amount: number;
  quotation_count: number;
  won_count: number;
  won_amount: number;
  order_count: number;
  order_amount: number;
  collected_amount: number;
  lead_to_opp_rate: number;
  opp_to_quotation_rate: number;
  opp_to_order_rate: number;
  order_to_collection_rate: number;
}

/** 加权预测项（对应后端 WeightedForecastItem） */
export interface WeightedForecastItem {
  opportunity_id: number;
  opportunity_no: string;
  opportunity_name: string;
  stage: string;
  estimated_amount: number;
  win_probability: number;
  weighted_amount: number;
  expected_close_date?: string;
}

/** 加权预测结果（对应后端 WeightedForecastResult） */
export interface WeightedForecastResult {
  total_opportunities: number;
  total_estimated_amount: number;
  total_weighted_amount: number;
  details: WeightedForecastItem[];
}

/** 预测准确性结果（对应后端 ForecastAccuracyResult） */
export interface ForecastAccuracyResult {
  year: number;
  month: number;
  forecast_amount: number;
  forecast_count: number;
  actual_amount: number;
  won_count: number;
  accuracy_rate: number;
}

/** 阶段计数（对应后端 StageCount） */
export interface StageCount {
  stage: string;
  count: number;
}

/** 转化率分析（对应后端 ConversionRateAnalysis） */
export interface ConversionRateAnalysis {
  period_start: string;
  period_end: string;
  total_opportunities: number;
  won_count: number;
  lost_count: number;
  open_count: number;
  win_rate: number;
  conversion_rate: number;
  stage_distribution: StageCount[];
}

/** 阶段停留时长项（对应后端 StageDurationItem） */
export interface StageDurationItem {
  opportunity_id: number;
  from_stage: string;
  to_stage: string;
  changed_at: string;
  duration_days: number;
}

/** 线索评分结果（对应后端 LeadScoreResult） */
export interface LeadScoreResult {
  lead_id: number;
  score: number;
  priority: string;
  breakdown: unknown;
}

/** 重复线索组（对应后端 DuplicateLeadGroup） */
export interface DuplicateLeadGroup {
  match_key: string;
  match_type: string;
  lead_ids: number[];
  lead_nos: string[];
  company_names: string[];
  count: number;
}

/** 合并线索结果（对应后端 MergeResult） */
export interface MergeResult {
  master_lead_id: number;
  master_lead_no: string;
  merged_count: number;
  merged_lead_nos: string[];
}

/** 线索转化漏斗报表（对应后端 LeadFunnelReport） */
export interface LeadFunnelReport {
  total_leads: number;
  converted_leads: number;
  total_opportunities: number;
  won_opportunities: number;
  total_customers: number;
  total_orders: number;
  lead_to_opp_rate: number;
  opp_to_customer_rate: number;
  opp_to_order_rate: number;
  overall_conversion_rate: number;
}

/** 商机转订单结果（对应后端 convert_opportunity_to_order 返回 json） */
export interface OpportunityConvertResult {
  order_id: number;
  order_no: string;
}

/**
 * 销售漏斗报告（V15 P2 18.2-D4）
 * 后端路由：GET /api/v1/erp/crm/opportunities/sales-funnel（routes/crm.rs crm_opportunity_routes）
 */
export function getSalesFunnel(params?: {
  start_date?: string;
  end_date?: string;
}): Promise<ApiResponse<SalesFunnelReport>> {
  return request.get('/crm/opportunities/sales-funnel', { params });
}

/**
 * 加权销售预测（V15 P2 18.2-D5）
 * 后端路由：GET /api/v1/erp/crm/opportunities/weighted-forecast（routes/crm.rs crm_opportunity_routes）
 */
export function getWeightedForecast(params?: {
  owner_id?: number;
}): Promise<ApiResponse<WeightedForecastResult>> {
  return request.get('/crm/opportunities/weighted-forecast', { params });
}

/**
 * 预测准确性分析（V15 P2 18.2-D4；year/month 缺省时取当前年月）
 * 后端路由：GET /api/v1/erp/crm/opportunities/forecast-accuracy（routes/crm.rs crm_opportunity_routes）
 */
export function getForecastAccuracy(params?: {
  year?: number;
  month?: number;
}): Promise<ApiResponse<ForecastAccuracyResult>> {
  return request.get('/crm/opportunities/forecast-accuracy', { params });
}

/**
 * 商机转化率分析（V15 P2 18.2-D5；months_back 缺省时后端取 12 个月）
 * 后端路由：GET /api/v1/erp/crm/opportunities/conversion-rate（routes/crm.rs crm_opportunity_routes）
 */
export function getConversionRate(params?: {
  months_back?: number;
}): Promise<ApiResponse<ConversionRateAnalysis>> {
  return request.get('/crm/opportunities/conversion-rate', { params });
}

/**
 * 商机阶段停留时长分析（V15 P2 18.2-D5）
 * 后端路由：GET /api/v1/erp/crm/opportunities/stage-duration（routes/crm.rs crm_opportunity_routes）
 */
export function getStageDuration(params?: {
  opportunity_id?: number;
}): Promise<ApiResponse<StageDurationItem[]>> {
  return request.get('/crm/opportunities/stage-duration', { params });
}

/**
 * 线索评分（V15 P1 18.1-D1）
 * 后端路由：POST /api/v1/erp/crm/leads/{id}/score（routes/crm.rs crm_lead_routes）
 */
export function scoreLead(id: number): Promise<ApiResponse<LeadScoreResult>> {
  return request.post(`/crm/leads/${id}/score`);
}

/**
 * 重复线索检测（V15 P1 18.1-D2；mobile_phone 与 company_name 至少传一项）
 * 后端路由：POST /api/v1/erp/crm/leads/detect-duplicates（routes/crm.rs crm_lead_routes）
 */
export function detectDuplicateLeads(data: {
  mobile_phone?: string;
  company_name?: string;
}): Promise<ApiResponse<DuplicateLeadGroup[]>> {
  return request.post('/crm/leads/detect-duplicates', data);
}

/**
 * 合并重复线索（V15 P1 18.1-D3；primary_id 必填，duplicate_ids 为被合并线索 ID 列表）
 * 后端路由：POST /api/v1/erp/crm/leads/merge（routes/crm.rs crm_lead_routes）
 */
export function mergeLeads(data: {
  primary_id: number;
  duplicate_ids: number[];
}): Promise<ApiResponse<MergeResult>> {
  return request.post('/crm/leads/merge', data);
}

/**
 * 线索转化漏斗报表（V15 P1 18.1-D3：线索→商机→客户→订单）
 * 后端路由：GET /api/v1/erp/crm/leads/funnel-report（routes/crm.rs crm_lead_routes）
 */
export function getLeadFunnelReport(params?: {
  start_date?: string;
  end_date?: string;
}): Promise<ApiResponse<LeadFunnelReport>> {
  return request.get('/crm/leads/funnel-report', { params });
}

/**
 * 商机转化为销售订单（生成草稿订单并将商机标记为 CLOSED_WON）
 * 后端路由：POST /api/v1/erp/crm/opportunities/{id}/convert（routes/crm.rs crm_opportunity_routes）
 */
export function convertOpportunityToOrder(
  id: number
): Promise<ApiResponse<OpportunityConvertResult>> {
  return request.post(`/crm/opportunities/${id}/convert`);
}

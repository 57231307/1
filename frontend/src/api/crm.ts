import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface Lead {
  id: number
  lead_no: string
  name: string
  phone: string
  email: string
  company: string
  source: string
  status: 'new' | 'contacted' | 'qualified' | 'proposal' | 'converted' | 'lost'
  rating: number
  address: string
  description: string
  created_by: number
  created_by_name: string
  assigned_to: number
  assigned_to_name: string
  created_at: string
  updated_at: string
}

/** 线索导入结果（v11 批次 157d-4） */
export interface ImportLeadError {
  row: number
  message: string
}

export interface ImportLeadsResult {
  total: number
  success_count: number
  failed_count: number
  errors: ImportLeadError[]
}

export interface Opportunity {
  id: number
  opportunity_no: string
  name: string
  customer_id: number
  customer_name: string
  /** 商机阶段（后端字段名，大写值：QUALIFICATION/NEEDS_ANALYSIS/PROPOSAL/NEGOTIATION/CLOSED_WON/CLOSED_LOST） */
  opportunity_stage?:
    | 'QUALIFICATION'
    | 'NEEDS_ANALYSIS'
    | 'PROPOSAL'
    | 'NEGOTIATION'
    | 'CLOSED_WON'
    | 'CLOSED_LOST'
  /** @deprecated 向后兼容字段，新代码应使用 opportunity_stage */
  stage?:
    | 'qualification'
    | 'needs_analysis'
    | 'value_proposition'
    | 'proposal'
    | 'negotiation'
    | 'closed_won'
    | 'closed_lost'
  estimated_amount: number
  probability: number
  expected_close_date: string
  description: string
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export function listLeads(params?: QueryParams): Promise<ApiResponse<Lead[]>> {
  return request.get('/crm/leads', { params })
}

export function getLead(id: number): Promise<ApiResponse<Lead>> {
  return request.get(`/crm/leads/${id}`)
}

export function createLead(data: Partial<Lead>): Promise<ApiResponse<Lead>> {
  return request.post('/crm/leads', data)
}

export function updateLead(id: number, data: Partial<Lead>): Promise<ApiResponse<Lead>> {
  return request.put(`/crm/leads/${id}`, data)
}

export function deleteLead(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/crm/leads/${id}`)
}

export function updateLeadStatus(
  id: number,
  data: { status: Lead['status'] }
): Promise<ApiResponse<void>> {
  return request.put(`/crm/leads/${id}/status`, data)
}

export function convertLead(
  id: number
): Promise<ApiResponse<{ customer_id: number; opportunity_id: number }>> {
  return request.post(`/crm/leads/${id}/convert`)
}

export function listOpportunities(params?: QueryParams): Promise<ApiResponse<Opportunity[]>> {
  return request.get('/crm/opportunities', { params })
}

export function getOpportunity(id: number): Promise<ApiResponse<Opportunity>> {
  return request.get(`/crm/opportunities/${id}`)
}

export function createOpportunity(data: Partial<Opportunity>): Promise<ApiResponse<Opportunity>> {
  return request.post('/crm/opportunities', data)
}

export function updateOpportunity(
  id: number,
  data: Partial<Opportunity>
): Promise<ApiResponse<Opportunity>> {
  return request.put(`/crm/opportunities/${id}`, data)
}

export function deleteOpportunity(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/crm/opportunities/${id}`)
}

export interface CustomerSummary {
  customer_id: number
  customer_name: string
  total_orders: number
  total_amount: number
  last_order_date?: string
  credit_limit?: number
  credit_used?: number
}

export function getCustomerSummary(customerId: number): Promise<ApiResponse<CustomerSummary>> {
  return request.get(`/crm/customers/${customerId}/summary`)
}

// 批次 94 P2-12 修复：补全 CRM 线索导出接口（原缺失，导致 leads/index.vue 导出占位假成功）
// 返回 blob，前端用 URL.createObjectURL 触发下载
export function exportLeads(params?: QueryParams): Promise<Blob> {
  return request.get('/crm/leads/export', {
    params,
    responseType: 'blob',
  })
}

// v11 批次 157d-4 新增：批量导入线索（xlsx），用 FormData 上传文件
export function importLeads(file: File): Promise<ApiResponse<ImportLeadsResult>> {
  const formData = new FormData()
  formData.append('file', file)
  return request.post('/crm/leads/import', formData)
}

// v11 批次 141 修复：补全 CRM 商机导出接口（原缺失，导致 opportunities/index.vue 导出假成功）
// 返回 blob，前端用 URL.createObjectURL 触发下载
export function exportOpportunities(params?: QueryParams): Promise<Blob> {
  return request.get('/crm/opportunities/export', {
    params,
    responseType: 'blob',
  })
}

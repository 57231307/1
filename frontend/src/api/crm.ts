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

export interface Opportunity {
  id: number
  opportunity_no: string
  name: string
  customer_id: number
  customer_name: string
  stage:
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

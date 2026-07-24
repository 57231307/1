import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface ApiEndpoint {
  id: number
  path: string
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'
  description: string
  module: string
  status: 'active' | 'inactive' | 'deprecated'
  rate_limit: number
  timeout: number
  authentication: boolean
  authorization: string[]
  request_schema: Record<string, unknown>
  response_schema: Record<string, unknown>
  created_at: string
  updated_at: string
}

export interface ApiLog {
  id: number
  endpoint_id: number
  endpoint_path: string
  method: string
  request_body: string
  response_body: string
  status_code: number
  response_time: number
  ip_address: string
  user_agent: string
  user_id: number
  user_name: string
  created_at: string
}

export interface ApiKey {
  id: number
  key_name: string
  api_key: string
  description: string
  permissions: string[]
  rate_limit: number
  expires_at: string
  status: 'active' | 'inactive' | 'expired'
  created_by: number
  created_by_name: string
  created_at: string
  last_used_at: string
}

export function getApiEndpointList(params?: QueryParams): Promise<ApiResponse<ApiEndpoint[]>> {
  return request.get('/api-gateway/endpoints', { params })
}

export function getApiEndpoint(id: number): Promise<ApiResponse<ApiEndpoint>> {
  return request.get(`/api-gateway/endpoints/${id}`)
}

export function createApiEndpoint(data: Partial<ApiEndpoint>): Promise<ApiResponse<ApiEndpoint>> {
  return request.post('/api-gateway/endpoints', data)
}

export function updateApiEndpoint(
  id: number,
  data: Partial<ApiEndpoint>
): Promise<ApiResponse<ApiEndpoint>> {
  return request.put(`/api-gateway/endpoints/${id}`, data)
}

export function deleteApiEndpoint(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api-gateway/endpoints/${id}`)
}

export function getApiLogList(params?: QueryParams): Promise<ApiResponse<ApiLog[]>> {
  return request.get('/api-gateway/logs', { params })
}

export function getApiLog(id: number): Promise<ApiResponse<ApiLog>> {
  return request.get(`/api-gateway/logs/${id}`)
}

export function getApiKeyList(params?: QueryParams): Promise<ApiResponse<ApiKey[]>> {
  return request.get('/api-gateway/keys', { params })
}

export function getApiKey(id: number): Promise<ApiResponse<ApiKey>> {
  return request.get(`/api-gateway/keys/${id}`)
}

export function createApiKey(data: Partial<ApiKey>): Promise<ApiResponse<ApiKey>> {
  return request.post('/api-gateway/keys', data)
}

export function updateApiKey(id: number, data: Partial<ApiKey>): Promise<ApiResponse<ApiKey>> {
  return request.put(`/api-gateway/keys/${id}`, data)
}

export function deleteApiKey(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api-gateway/keys/${id}`)
}

export function regenerateApiKey(id: number): Promise<ApiResponse<ApiKey>> {
  return request.post(`/api-gateway/keys/${id}/regenerate`)
}

// P2-16 修复（批次 86 v2 复审）：getApiStats ApiResponse<any> → 显式接口

/** API 网关统计 */
export interface ApiStats {
  total_endpoints: number
  active_endpoints: number
  inactive_endpoints: number
  total_keys: number
  active_keys: number
  total_requests: number
  total_errors: number
  avg_response_time_ms: number
  [key: string]: unknown
}

export function getApiStats(): Promise<ApiResponse<ApiStats>> {
  return request.get('/api-gateway/stats')
}

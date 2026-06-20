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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  request_schema: Record<string, any>
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  response_schema: Record<string, any>
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

export function listApiEndpoints(params?: QueryParams): Promise<ApiResponse<ApiEndpoint[]>> {
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

export function listApiLogs(params?: QueryParams): Promise<ApiResponse<ApiLog[]>> {
  return request.get('/api-gateway/logs', { params })
}

export function getApiLog(id: number): Promise<ApiResponse<ApiLog>> {
  return request.get(`/api-gateway/logs/${id}`)
}

export function listApiKeys(params?: QueryParams): Promise<ApiResponse<ApiKey[]>> {
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

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getApiStats(): Promise<ApiResponse<any>> {
  return request.get('/api-gateway/stats')
}

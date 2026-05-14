import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface ApiKey {
  id: number
  name: string
  key: string
  scopes: string[]
  expiresAt: string
  lastUsedAt: string
  createdAt: string
  active: boolean
}

export interface CreateApiKeyRequest {
  name: string
  scopes: string[]
  expiresIn?: number
}

export interface ApiKeyQueryParams extends QueryParams {
  active?: boolean
}

export function listApiKeys(params?: ApiKeyQueryParams): Promise<ApiResponse<{ list: ApiKey[]; total: number }>> {
  return request.get('/api-keys', { params })
}

export function getApiKey(id: number): Promise<ApiResponse<ApiKey>> {
  return request.get(`/api-keys/${id}`)
}

export function createApiKey(data: CreateApiKeyRequest): Promise<ApiResponse<{ key: ApiKey; secret: string }>> {
  return request.post('/api-keys', data)
}

export function deleteApiKey(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api-keys/${id}`)
}

export function enableApiKey(id: number): Promise<ApiResponse<void>> {
  return request.post(`/api-keys/${id}/enable`)
}

export function disableApiKey(id: number): Promise<ApiResponse<void>> {
  return request.post(`/api-keys/${id}/disable`)
}

export function regenerateApiKey(id: number): Promise<ApiResponse<{ key: ApiKey; secret: string }>> {
  return request.post(`/api-keys/${id}/regenerate`)
}

export function getAvailableScopes(): Promise<ApiResponse<{ scopes: string[]; descriptions: Record<string, string> }>> {
  return request.get('/api-keys/scopes')
}

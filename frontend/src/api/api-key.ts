import { request } from './request'
import type { ApiResponse } from './request'

export interface ApiKey {
  id?: number
  name: string
  key?: string
  keyPrefix?: string
  permissions?: string[]
  expiresAt?: string
  lastUsedAt?: string
  isActive?: boolean
  description?: string
  createdAt?: string
  updatedAt?: string
}

export interface ApiKeyCreateRequest {
  name: string
  permissions?: string[]
  expiresAt?: string
  description?: string
}

export const apiKeyApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: ApiKey[]; total: number }>>('/api-keys', { params }),

  create: (data: ApiKeyCreateRequest) =>
    request.post<ApiResponse<ApiKey>>('/api-keys', data),
}

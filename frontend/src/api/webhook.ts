import { request } from './request'
import type { ApiResponse } from './request'

export interface Webhook {
  id?: number
  name: string
  url: string
  events: string[]
  isActive?: boolean
  secret?: string
  headers?: Record<string, string>
  retryCount?: number
  timeout?: number
  lastTriggeredAt?: string
  createdAt?: string
  updatedAt?: string
}

export interface WebhookEvent {
  id?: number
  webhookId?: number
  eventType: string
  payload: Record<string, any>
  status: 'pending' | 'success' | 'failed'
  attempts?: number
  response?: string
  createdAt?: string
}

export const webhookApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: Webhook[]; total: number }>>('/webhooks', { params }),

  create: (data: Partial<Webhook>) =>
    request.post<ApiResponse<Webhook>>('/webhooks', data),
}

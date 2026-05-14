import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface Webhook {
  id: number
  name: string
  url: string
  events: string[]
  secret: string
  active: boolean
  headers: Record<string, string>
  createdAt: string
  updatedAt: string
}

export interface WebhookLog {
  id: number
  webhookId: number
  event: string
  requestBody: string
  responseStatus: number
  responseBody: string
  error: string
  createdAt: string
}

export interface CreateWebhookRequest {
  name: string
  url: string
  events: string[]
  secret: string
  headers?: Record<string, string>
}

export interface UpdateWebhookRequest {
  name?: string
  url?: string
  events?: string[]
  secret?: string
  active?: boolean
  headers?: Record<string, string>
}

export interface WebhookQueryParams extends QueryParams {
  active?: boolean
  event?: string
}

export function listWebhooks(params?: WebhookQueryParams): Promise<ApiResponse<{ list: Webhook[]; total: number }>> {
  return request.get('/webhooks', { params })
}

export function getWebhook(id: number): Promise<ApiResponse<Webhook>> {
  return request.get(`/webhooks/${id}`)
}

export function createWebhook(data: CreateWebhookRequest): Promise<ApiResponse<Webhook>> {
  return request.post('/webhooks', data)
}

export function updateWebhook(id: number, data: UpdateWebhookRequest): Promise<ApiResponse<Webhook>> {
  return request.put(`/webhooks/${id}`, data)
}

export function deleteWebhook(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/webhooks/${id}`)
}

export function enableWebhook(id: number): Promise<ApiResponse<void>> {
  return request.post(`/webhooks/${id}/enable`)
}

export function disableWebhook(id: number): Promise<ApiResponse<void>> {
  return request.post(`/webhooks/${id}/disable`)
}

export function testWebhook(id: number): Promise<ApiResponse<{ success: boolean; message: string }>> {
  return request.post(`/webhooks/${id}/test`)
}

export function getWebhookLogs(webhookId: number, params?: QueryParams): Promise<ApiResponse<{ list: WebhookLog[]; total: number }>> {
  return request.get(`/webhooks/${webhookId}/logs`, { params })
}

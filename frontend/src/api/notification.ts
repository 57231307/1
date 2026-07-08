import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface NotificationQueryParams extends QueryParams {
  status?: string
  notificationType?: string
}

export interface Notification {
  id?: number
  title?: string
  content?: string
  status?: string
  notificationType?: string
  businessType?: string
  businessId?: number
  createdAt?: string
  readAt?: string
}

export interface NotificationSetting {
  id?: number
  businessType?: string
  enableInternal?: boolean
  enableEmail?: boolean
  enableSms?: boolean
}

export interface BatchOperationRequest {
  ids: number[]
}

export interface UpdateSettingRequest {
  businessType: string
  enableInternal: boolean
  enableEmail: boolean
  enableSms: boolean
}

export function listNotifications(
  params?: NotificationQueryParams
): Promise<ApiResponse<{ list: Notification[]; total: number }>> {
  return request.get('/notifications/', { params })
}

export function getNotification(id: number): Promise<ApiResponse<Notification>> {
  return request.get(`/notifications/${id}`)
}

export function getUnreadCount(): Promise<ApiResponse<number>> {
  return request.get('/notifications/unread-count')
}

export function markAsRead(id: number): Promise<ApiResponse<void>> {
  return request.post(`/notifications/${id}/read`)
}

export function batchMarkAsRead(data: BatchOperationRequest): Promise<ApiResponse<void>> {
  return request.post('/notifications/batch-read', data)
}

export function markAllAsRead(): Promise<ApiResponse<void>> {
  return request.post('/notifications/read-all')
}

export function deleteNotification(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/notifications/${id}`)
}

export function getSettings(): Promise<ApiResponse<NotificationSetting[]>> {
  return request.get('/notifications/settings')
}

export function updateSetting(
  data: UpdateSettingRequest
): Promise<ApiResponse<NotificationSetting>> {
  return request.put('/notifications/settings', data)
}

/** WebSocket 票据响应（v12 P1-4：一次性短时票据替代 URL query JWT） */
export interface WsTicketResponse {
  ticket: string
  expires_in: number
}

/**
 * 获取 WebSocket 连接票据
 *
 * v12 P1-4 修复：浏览器 WebSocket API 不支持自定义 header，
 * 原方案通过 URL query 传递 JWT 会导致 token 泄露到浏览器历史、
 * 服务器 access log、中间代理日志。
 *
 * 新方案：客户端先通过 HTTP POST（自动携带 httpOnly Cookie JWT）获取
 * 一次性短时票据（30 秒有效），再用票据建立 WebSocket 连接。
 * 票据一次性消费，即使泄露也无法复用。
 */
export function fetchWsTicket(): Promise<WsTicketResponse> {
  return request.post<WsTicketResponse>('/ws/ticket')
}

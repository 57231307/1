import { request } from './request'
import type { ApiResponse, QueryParams as BaseQueryParams } from '../types/api'

export interface NotificationQueryParams extends BaseQueryParams {
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

export function listNotifications(params?: NotificationQueryParams): Promise<ApiResponse<{ list: Notification[]; total: number }>> {
  return request.get('/erp/notifications', { params })
}

export function getNotification(id: number): Promise<ApiResponse<Notification>> {
  return request.get(`/erp/notifications/${id}`)
}

export function getUnreadCount(): Promise<ApiResponse<number>> {
  return request.get('/erp/notifications/unread-count')
}

export function markAsRead(id: number): Promise<ApiResponse<void>> {
  return request.post(`/erp/notifications/${id}/read`)
}

export function batchMarkAsRead(data: BatchOperationRequest): Promise<ApiResponse<void>> {
  return request.post('/erp/notifications/batch-read', data)
}

export function markAllAsRead(): Promise<ApiResponse<void>> {
  return request.post('/erp/notifications/mark-all-read')
}

export function deleteNotification(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/erp/notifications/${id}`)
}

export function getSettings(): Promise<ApiResponse<NotificationSetting[]>> {
  return request.get('/erp/notifications/settings')
}

export function updateSetting(data: UpdateSettingRequest): Promise<ApiResponse<NotificationSetting>> {
  return request.put('/erp/notifications/settings', data)
}

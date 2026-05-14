import { request } from './request'

export interface QueryParams {
  page?: number
  pageSize?: number
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

export function listNotifications(params?: QueryParams) {
  return request.get('/api/v1/notifications', { params })
}

export function getNotification(id: number) {
  return request.get(`/api/v1/notifications/${id}`)
}

export function getUnreadCount() {
  return request.get('/api/v1/notifications/unread-count')
}

export function markAsRead(id: number) {
  return request.post(`/api/v1/notifications/${id}/read`)
}

export function batchMarkAsRead(data: BatchOperationRequest) {
  return request.post('/api/v1/notifications/batch-read', data)
}

export function markAllAsRead() {
  return request.post('/api/v1/notifications/mark-all-read')
}

export function deleteNotification(id: number) {
  return request.delete(`/api/v1/notifications/${id}`)
}

export function getSettings() {
  return request.get('/api/v1/notifications/settings')
}

export function updateSetting(data: UpdateSettingRequest) {
  return request.post('/api/v1/notifications/settings', data)
}

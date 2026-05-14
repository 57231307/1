import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface SystemUpdate {
  id: number
  version: string
  releaseDate: string
  description: string
  status: 'available' | 'downloading' | 'installed' | 'failed'
  size: number
  checksum: string
}

export interface UpdateLog {
  id: number
  updateId: number
  action: string
  status: 'success' | 'failed'
  message: string
  createdAt: string
}

export interface UpdateCheckResult {
  available: boolean
  latestVersion: string
  currentVersion: string
  changelog: string[]
}

export function checkUpdate(): Promise<ApiResponse<UpdateCheckResult>> {
  return request.get('/system-update/check')
}

export function listUpdates(): Promise<ApiResponse<{ list: SystemUpdate[]; total: number }>> {
  return request.get('/system-update/versions')
}

export function getUpdateDetail(id: number): Promise<ApiResponse<SystemUpdate>> {
  return request.get(`/system-update/versions/${id}`)
}

export function downloadUpdate(id: number): Promise<ApiResponse<{ downloadUrl: string }>> {
  return request.post(`/system-update/versions/${id}/download`)
}

export function installUpdate(id: number): Promise<ApiResponse<void>> {
  return request.post(`/system-update/versions/${id}/install`)
}

export function rollbackUpdate(id: number): Promise<ApiResponse<void>> {
  return request.post(`/system-update/versions/${id}/rollback`)
}

export function getUpdateLogs(params?: QueryParams): Promise<ApiResponse<{ list: UpdateLog[]; total: number }>> {
  return request.get('/system-update/logs', { params })
}

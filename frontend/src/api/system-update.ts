import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface SystemVersion {
  id: number
  version: string
  release_date: string
  release_notes: string
  features: string[]
  bug_fixes: string[]
  breaking_changes: string[]
  download_url: string
  file_size: number
  checksum: string
  status: 'available' | 'downloading' | 'downloaded' | 'installing' | 'installed' | 'failed'
  created_at: string
}

export interface UpdateTask {
  id: number
  task_code: string
  from_version: string
  to_version: string
  status:
    | 'pending'
    | 'downloading'
    | 'downloaded'
    | 'installing'
    | 'completed'
    | 'failed'
    | 'rolled_back'
  progress: number
  error_message: string
  backup_path: string
  started_at: string
  completed_at: string
  created_by: number
  created_by_name: string
  created_at: string
}

export interface SystemBackup {
  id: number
  backup_code: string
  backup_type: 'full' | 'incremental' | 'database' | 'files'
  file_path: string
  file_size: number
  description: string
  status: 'creating' | 'completed' | 'failed'
  created_by: number
  created_by_name: string
  created_at: string
}

export function checkForUpdates(): Promise<ApiResponse<SystemVersion>> {
  return request.get('/system-update/check')
}

export function getSystemVersionList(params?: QueryParams): Promise<ApiResponse<SystemVersion[]>> {
  return request.get('/system-update/versions', { params })
}

export function getSystemVersion(id: number): Promise<ApiResponse<SystemVersion>> {
  return request.get(`/system-update/versions/${id}`)
}

export function downloadUpdate(versionId: number): Promise<ApiResponse<UpdateTask>> {
  return request.post(`/system-update/versions/${versionId}/download`)
}

export function installUpdate(versionId: number): Promise<ApiResponse<UpdateTask>> {
  return request.post(`/system-update/versions/${versionId}/install`)
}

export function getUpdateTaskList(params?: QueryParams): Promise<ApiResponse<UpdateTask[]>> {
  return request.get('/system-update/tasks', { params })
}

export function getUpdateTask(id: number): Promise<ApiResponse<UpdateTask>> {
  return request.get(`/system-update/tasks/${id}`)
}

export function cancelUpdateTask(id: number): Promise<ApiResponse<void>> {
  return request.post(`/system-update/tasks/${id}/cancel`)
}

/**
 * 回滚到指定版本
 * 后端路由 POST /api/v1/erp/system-update/rollback
 * 请求体：{ version: string }（后端 RollbackRequest 需要 version 字段，非 taskId）
 */
export function rollbackUpdate(version: string): Promise<ApiResponse<void>> {
  return request.post('/system-update/rollback', { version })
}

export function getSystemBackupList(params?: QueryParams): Promise<ApiResponse<SystemBackup[]>> {
  return request.get('/system-update/backups', { params })
}

export function getSystemBackup(id: number): Promise<ApiResponse<SystemBackup>> {
  return request.get(`/system-update/backups/${id}`)
}

export function createSystemBackup(
  data: Partial<SystemBackup>
): Promise<ApiResponse<SystemBackup>> {
  return request.post('/system-update/backups', data)
}

export function deleteSystemBackup(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/system-update/backups/${id}`)
}

export function restoreFromBackup(id: number): Promise<ApiResponse<void>> {
  return request.post(`/system-update/backups/${id}/restore`)
}

export function downloadBackup(id: number): Promise<Blob> {
  return request.get(`/system-update/backups/${id}/download`, {
    responseType: 'blob',
  })
}

export function getCurrentVersion(): Promise<ApiResponse<{ version: string; build_date: string }>> {
  return request.get('/system-update/current-version')
}

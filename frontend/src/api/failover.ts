// 主备隔离 API
import { request } from './request'

/** 主备状态 DTO */
export interface FailoverStatusDto {
  function_name: string
  current_state: string
  circuit_state: string
  primary_url?: string
  backup_type?: string
  last_switch_at?: string
  last_success_at?: string
  consecutive_failures: number
  total_primary_calls: number
  total_backup_calls: number
  total_switches: number
  updated_at: string
}

/** 切换事件 DTO */
export interface FailoverEventDto {
  id: number
  function_name: string
  event_type: string
  from_state?: string
  to_state?: string
  reason?: string
  latency_ms?: number
  created_at: string
}

/** 主备状态响应 */
export interface StatusResponse {
  statuses: FailoverStatusDto[]
  events: FailoverEventDto[]
}

/** 获取主备状态 */
export function getFailoverStatus(): Promise<StatusResponse> {
  return request.get<StatusResponse>('/admin/failover/status')
}

/** 获取 Prometheus 指标 */
export function getFailoverMetrics(): Promise<string> {
  return request.get<string>('/admin/failover/metrics', {
    responseType: 'text',
  })
}

/** 手动切换 */
export function triggerSwitch(functionName: string): Promise<{ data: string }> {
  return request.post<{ data: string }>('/admin/failover/test/switch', {
    function: functionName,
  })
}

/** 健康检查 */
export function getFailoverHealth(): Promise<{ data: { database: string; cache: string } }> {
  return request.get<{ data: { database: string; cache: string } }>('/admin/failover/health')
}

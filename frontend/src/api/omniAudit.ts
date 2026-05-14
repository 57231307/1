import { request } from './request'

export interface TrackEventRequest {
  event_type: string
  event_name: string
  resource: string
  action: string
  payload?: Record<string, any>
  duration_ms?: number
  status?: string
}

export interface AuditStats {
  total_events: number
  today_events: number
  error_count: number
  avg_duration_ms: number
  top_resources: Array<{ name: string; count: number }>
  top_users: Array<{ name: string; count: number }>
}

export interface AuditLog {
  id: string
  trace_id: string
  user_id: number
  user_name?: string
  event_type: string
  event_name: string
  resource: string
  action: string
  payload?: Record<string, any>
  ip_address?: string
  user_agent?: string
  duration_ms: number
  status: string
  error_msg?: string
  created_at: string
}

export interface AuditQueryFilter {
  user_id?: number
  event_type?: string
  resource?: string
  action?: string
  status?: string
  start_time?: string
  end_time?: string
  page?: number
  page_size?: number
}

export function trackEvent(data: TrackEventRequest) {
  return request.post('/omni-audit/track', data)
}

export function getDashboardStats() {
  return request.get('/omni-audit/dashboard')
}

export function searchLogs(filter: AuditQueryFilter) {
  return request.get('/omni-audit/search', { params: filter })
}
import { request } from './request';

export interface TrackEventRequest {
  event_type: string;
  event_name: string;
  resource: string;
  action: string;
  payload?: Record<string, unknown>;
  duration_ms?: number;
  status?: string;
}

export interface AuditStats {
  total_events: number;
  today_events: number;
  error_count: number;
  avg_duration_ms: number;
  top_resources: Array<{ name: string; count: number }>;
  top_users: Array<{ name: string; count: number }>;
}

// GET /finance/audit/search 出参行（唯一真相：omni_audit_handler.rs row_to_json 的键集，
// 数据源 omni_audit_logs 原始列；include_sensitive=false 时无 request_body/user_agent/ip_address）。
// 后端不回传 event_name/resource/status/payload/error_msg/user_name（此前为臆造键，读取处恒空）。
export interface AuditLog {
  id: number;
  trace_id: string;
  user_id: number;
  username: string;
  module: string;
  action: string;
  resource_type: string;
  resource_id: string;
  resource_name: string;
  description: string;
  request_method: string;
  request_path: string;
  response_status: number;
  duration_ms: number;
  created_at: string;
  request_body?: string;
  user_agent?: string;
  ip_address?: string;
}

/**
 * 查询过滤（唯一真相：services/omni_audit_query_service.rs AuditQueryFilter）。
 * start_date/end_date 为 chrono NaiveDate → 'YYYY-MM-DD'；后端无
 * resource/action/status/start_time/end_time 键（传入即被静默丢弃，属后端缺口，
 * 已登记串行清单），故不再声明。
 */
export interface AuditQueryFilter {
  user_id?: number;
  event_type?: string;
  start_date?: string;
  end_date?: string;
  keyword?: string;
  page?: number;
  page_size?: number;
  include_sensitive?: boolean;
}

export function trackEvent(data: TrackEventRequest) {
  return request.post('/finance/audit/track', data);
}

export function getDashboardStats() {
  return request.get('/finance/audit/stats');
}

export function searchLogs(filter: AuditQueryFilter) {
  return request.get('/finance/audit/search', { params: filter });
}
